use crate::plugin::host_service::ServiceHandler;
use crate::plugin::plugin_proxy::{MsgHandler, PluginState, RequestHandler};
use crate::plugin::Plugin;

use otx_format::jsonrpc_types::tx_view::otx_to_tx_view;
use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{MessageFromHost, MessageFromPlugin, PluginInfo};

use ckb_types::core::service::Request;
use ckb_types::H256;
use crossbeam_channel::{bounded, select, unbounded};

use std::path::PathBuf;
use std::thread;
use std::thread::JoinHandle;

#[derive(Clone)]
struct Context {
    pub plugin_name: String,
    pub _ckb_uri: String,
    pub _service_handler: ServiceHandler,
}

impl Context {
    fn new(plugin_name: &str, ckb_uri: &str, service_handler: ServiceHandler) -> Self {
        Context {
            plugin_name: plugin_name.to_owned(),
            _ckb_uri: ckb_uri.to_owned(),
            _service_handler: service_handler,
        }
    }
}

impl Context {}

pub struct AtomicSwap {
    state: PluginState,
    info: PluginInfo,

    /// Send request to plugin thread, and expect a response.
    request_handler: RequestHandler,

    /// Send notifaction/response to plugin thread.
    msg_handler: MsgHandler,

    _thread: JoinHandle<()>,
}

impl Plugin for AtomicSwap {
    fn get_name(&self) -> String {
        self.info.name.clone()
    }

    fn msg_handler(&self) -> MsgHandler {
        self.msg_handler.clone()
    }

    fn request_handler(&self) -> RequestHandler {
        self.request_handler.clone()
    }

    fn get_info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn get_state(&self) -> PluginState {
        self.state.clone()
    }
}

impl AtomicSwap {
    pub fn new(service_handler: ServiceHandler, ckb_uri: &str) -> Result<AtomicSwap, String> {
        let name = "agent template";
        let state = PluginState::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(
            name,
            "Collect micropayment otx and aggregate them into ckb tx.",
            "1.0",
        );
        let (msg_handler, request_handler, thread) =
            AtomicSwap::start_process(Context::new(name, ckb_uri, service_handler))?;
        Ok(AtomicSwap {
            state,
            info,
            msg_handler,
            request_handler,
            _thread: thread,
        })
    }
}

impl AtomicSwap {
    fn start_process(
        context: Context,
    ) -> Result<(MsgHandler, RequestHandler, JoinHandle<()>), String> {
        // the host request channel receives request from host to plugin
        let (host_request_sender, host_request_receiver) = bounded(1);
        // the channel sends notifications or responses from the host to plugin
        let (host_msg_sender, host_msg_receiver) = unbounded();

        let plugin_name = context.plugin_name.to_owned();
        // this thread processes information from host to plugin
        let thread = thread::spawn(move || {
            let do_select = || -> Result<bool, String> {
                select! {
                    // request from host to plugin
                    recv(host_request_receiver) -> msg => {
                        match msg {
                            Ok(Request { responder, arguments }) => {
                                log::debug!("{} receives request arguments: {:?}",
                                    context.plugin_name, arguments);
                                // handle
                                let response = (0, MessageFromPlugin::Ok);
                                responder.send(response).map_err(|err| err.to_string())?;
                                Ok(false)
                            }
                            Err(err) => Err(err.to_string())
                        }
                    }
                    // repsonse/notification from host to plugin
                    recv(host_msg_receiver) -> msg => {
                        match msg {
                            Ok(msg) => {
                                match msg {
                                    (_, MessageFromHost::NewInterval(elapsed)) => {
                                        Self::on_new_intervel(context.clone(), elapsed);
                                    }
                                    (_, MessageFromHost::NewOtx(otx)) => {
                                        log::info!("{} receivers msg NewOtx hash: {:?}",
                                            context.plugin_name,
                                            otx_to_tx_view(otx.clone()).unwrap().hash.to_string());
                                        Self::on_new_open_tx(context.clone(), otx);
                                    }
                                    (_, MessageFromHost::CommitOtx(otx_hashes)) => {
                                        Self::on_commit_open_tx(context.clone(), otx_hashes);
                                    }
                                    _ => unreachable!(),
                                }
                                Ok(false)
                            }
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
                        log::error!("plugin {} error: {}", plugin_name, err);
                        break;
                    }
                }
            }
        });

        Ok((host_msg_sender, host_request_sender, thread))
    }

    fn on_new_open_tx(_context: Context, _otx: OpenTransaction) {}

    fn on_commit_open_tx(context: Context, otx_hashes: Vec<H256>) {
        log::info!(
            "{} on commit open tx remove committed otx: {:?}",
            context.plugin_name,
            otx_hashes
                .iter()
                .map(|hash| hash.to_string())
                .collect::<Vec<String>>()
        );
    }

    fn on_new_intervel(_context: Context, _elapsed: u64) {}
}
