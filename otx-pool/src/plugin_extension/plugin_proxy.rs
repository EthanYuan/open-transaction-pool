use crate::notify::RuntimeHandle;

use otx_pool_plugin_protocol::{
    HostServiceHandler, MessageFromHost, MessageFromPlugin, MessageType, Plugin, PluginInfo,
    PluginMeta,
};

use ckb_types::core::service::Request;
use crossbeam_channel::{bounded, select, unbounded, Sender};
use tokio::task::JoinHandle;

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};

pub type RequestHandler = Sender<Request<(u64, MessageFromHost), (u64, MessageFromPlugin)>>;
pub type MsgHandler = Sender<(u64, MessageFromHost)>;

pub struct PluginProcess {
    _plugin_process: Child,
    _stdin_thread: JoinHandle<()>,
    _stdout_thread: JoinHandle<()>,
}

pub struct PluginProxy {
    state: PluginMeta,
    info: PluginInfo,
    _process: PluginProcess,

    /// Send request to stdin thread, and expect a response from stdout thread.
    request_handler: RequestHandler,

    /// Send notifaction/response to stdin thread.
    msg_handler: MsgHandler,
}

impl Plugin for PluginProxy {
    fn get_name(&self) -> String {
        self.info.name.clone()
    }

    fn get_info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn get_meta(&self) -> PluginMeta {
        self.state.clone()
    }
}

impl PluginProxy {
    pub fn msg_handler(&self) -> MsgHandler {
        self.msg_handler.clone()
    }

    pub fn request_handler(&self) -> RequestHandler {
        self.request_handler.clone()
    }

    /// This function will create a temporary plugin process to fetch plugin information.
    pub fn load_plugin_info(binary_path: PathBuf) -> Result<PluginInfo, String> {
        let mut child = Command::new(&binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|err| err.to_string())?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| String::from("Get stdin failed"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| String::from("Get stdout failed"))?;

        // request from host to plugin
        let request = (0u64, MessageFromHost::GetPluginInfo);
        let request_string = serde_json::to_string(&request).expect("Serialize request error");
        log::debug!("Send request to plugin: {}", request_string);
        stdin
            .write_all(format!("{}\n", request_string).as_bytes())
            .map_err(|err| err.to_string())?;
        stdin.flush().map_err(|err| err.to_string())?;

        // get response from plugin
        let mut buf_reader = BufReader::new(stdout);
        let mut response_string = String::new();
        buf_reader
            .read_line(&mut response_string)
            .map_err(|err| err.to_string())?;
        log::debug!("Receive response from plugin: {}", response_string.trim());
        let (id, response): (u64, MessageFromPlugin) =
            serde_json::from_str(&response_string).map_err(|err| err.to_string())?;

        if let (0u64, MessageFromPlugin::PluginInfo(plugin_info)) = (id, response) {
            Ok(plugin_info)
        } else {
            Err(format!(
                "Invalid response for get_info call to plugin {:?}, response: {}",
                binary_path, response_string
            ))
        }
    }

    pub fn start_process(
        runtime: RuntimeHandle,
        plugin_state: PluginMeta,
        plugin_info: PluginInfo,
        service_handler: HostServiceHandler,
    ) -> Result<PluginProxy, String> {
        let mut child = Command::new(plugin_state.binary_path.clone())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|err| err.to_string())?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| String::from("Get stdin failed"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| String::from("Get stdout failed"))?;

        // the host request channel receives request from host to plugin
        let (host_request_sender, host_request_receiver) = bounded(1);

        // the plugin response channel receives response from plugin,
        // it cooperates with the host request channel to complete the request-response pair
        let (plugin_response_sender, plugin_response_receiver) = bounded(1);

        // the channel sends notifications or responses from the host to plugin
        let (host_msg_sender, host_msg_receiver) = unbounded();

        let plugin_name = plugin_info.name.clone();
        // this thread processes stdin information from host to plugin
        let stdin_thread = runtime.spawn(async move  {
            let handle_host_msg =
                |stdin: &mut ChildStdin, (id, response)| -> Result<bool, String> {
                    let response_string =
                        serde_json::to_string(&(id, response)).expect("Serialize response error");
                    log::debug!("Send response/notification to plugin: {}", response_string);
                    stdin
                        .write_all(format!("{}\n", response_string).as_bytes())
                        .map_err(|err| err.to_string())?;
                    stdin.flush().map_err(|err| err.to_string())?;
                    Ok(false)
                };

            let mut do_select = || -> Result<bool, String> {
                select! {
                    // request from host to plugin
                    recv(host_request_receiver) -> msg => {
                        match msg {
                            Ok(Request { responder, arguments }) => {
                                let request_string = serde_json::to_string(&arguments).expect("Serialize request error");
                                log::debug!("Send request to plugin: {}", request_string);
                                stdin.write_all(format!("{}\n", request_string).as_bytes()).map_err(|err| err.to_string())?;
                                stdin.flush().map_err(|err| err.to_string())?;
                                loop {
                                    select!{
                                        recv(plugin_response_receiver) -> msg => {
                                            match msg {
                                                Ok(response) => {
                                                    responder.send(response).map_err(|err| err.to_string())?;
                                                    return Ok(false);
                                                }
                                                Err(err) => {
                                                    return Err(err.to_string());
                                                }
                                            }
                                        },
                                        recv(host_msg_receiver) -> msg => {
                                            match msg {
                                                Ok(msg) => {
                                                    handle_host_msg(&mut stdin, msg)?;
                                                },
                                                Err(err) => {
                                                    return Err(err.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(err) => Err(err.to_string())
                        }
                    }
                    // repsonse/notification from host to plugin
                    recv(host_msg_receiver) -> msg => {
                        match msg {
                            Ok(msg) => handle_host_msg(&mut stdin, msg),
                            Err(err) => Err(err.to_string())
                        }
                    }
                    // ignore the unexpected response from plugin
                    recv(plugin_response_receiver) -> msg => {
                        log::debug!("Received unexpected response/notification to plugin: {:?}", msg);
                        match msg {
                            Ok(_) => Ok(false),
                            Err(err) => Err(err.to_string())
                        }
                    }
                }
            };
            loop {
                match do_select() {
                    Ok(true) => {
                        break;
                    }
                    Ok(false) => (),
                    Err(err) => {
                        log::error!("plugin {} stdin error: {}", plugin_name, err);
                        break;
                    }
                }
            }
        });

        let plugin_name = plugin_info.name.clone();
        let msg_sender = host_msg_sender.clone();
        let mut buf_reader = BufReader::new(stdout);
        let stdout_thread = runtime.spawn(async move {
            let mut do_recv = || -> Result<bool, String> {
                let mut content = String::new();
                if buf_reader
                    .read_line(&mut content)
                    .map_err(|err| err.to_string())?
                    == 0
                {
                    // EOF
                    return Ok(true);
                }

                let (id, message_from_plugin): (u64, MessageFromPlugin) =
                    serde_json::from_str(&content).map_err(|err| err.to_string())?;
                match message_from_plugin.get_message_type() {
                    MessageType::Response => {
                        // Receive response from plugin
                        log::debug!("Receive response from plugin: {}", content.trim());
                        plugin_response_sender
                            .send((id, message_from_plugin))
                            .map_err(|err| err.to_string())?;
                    }
                    MessageType::Request => {
                        // Handle request from plugin
                        log::debug!("Receive request from plugin: {}", content.trim());
                        log::debug!("Sending request to ServiceProvider");
                        let message_from_host =
                            Request::call(&service_handler, message_from_plugin).ok_or_else(
                                || String::from("Send request to ServiceProvider failed"),
                            )?;
                        log::debug!(
                            "Received response from ServiceProvider: {:?}",
                            message_from_host
                        );
                        msg_sender
                            .send((id, message_from_host))
                            .map_err(|err| err.to_string())?;
                    }
                    MessageType::Notify => {
                        unreachable!()
                    }
                }

                Ok(false)
            };
            loop {
                match do_recv() {
                    Ok(true) => {
                        log::info!("plugin {} quit", plugin_name);
                        break;
                    }
                    Ok(false) => {}
                    Err(err) => {
                        log::warn!("plugin {} stdout error: {}", plugin_name, err);
                        break;
                    }
                }
            }
        });

        let process = PluginProcess {
            _plugin_process: child,
            _stdin_thread: stdin_thread,
            _stdout_thread: stdout_thread,
        };

        Ok(PluginProxy {
            state: plugin_state,
            info: plugin_info,
            _process: process,
            request_handler: host_request_sender,
            msg_handler: host_msg_sender,
        })
    }
}

impl Drop for PluginProxy {
    fn drop(&mut self) {
        // TODO: send term signal to the process
    }
}
