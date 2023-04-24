use std::path::PathBuf;
use std::thread::{self, JoinHandle};

use crate::plugin::{
    host_service::ServiceHandler,
    plugin_proxy::{MsgHandler, PluginState, RequestHandler},
    Plugin,
};
use anyhow::Result;
use ckb_types::core::service::Request;
use crossbeam_channel::{bounded, select, unbounded};
use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{MessageFromHost, MessageFromPlugin, PluginInfo};
use utils::config::built_in_plugins::P2PRelayerConfig;

pub struct P2PRelayer {
    info: PluginInfo,
    state: PluginState,
    request_handler: RequestHandler,
    msg_handler: MsgHandler,
    _thread: JoinHandle<()>,
}

#[derive(Clone)]
struct Context {
    plugin_name: String,
    config: P2PRelayerConfig,
    service_handler: ServiceHandler,
}

impl P2PRelayer {
    pub fn new(service_handler: ServiceHandler, config: P2PRelayerConfig) -> Result<Self> {
        log::info!("P2PRelayer started with config: {:?}", config);
        let name = "p2p_relayer";
        let state = PluginState::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(name, "This plugin relays OTXs via P2P network.", "1.0");

        let (msg_handler, request_handler, thread) =
            P2PRelayer::start_process(Context::new(name.to_owned(), config, service_handler))?;
        Ok(P2PRelayer {
            state,
            info,
            msg_handler,
            request_handler,
            _thread: thread,
        })
    }

    fn start_process(context: Context) -> Result<(MsgHandler, RequestHandler, JoinHandle<()>)> {
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
                                    (_, MessageFromHost::NewInterval(_)) => {
                                    }
                                    (_, MessageFromHost::NewOtx(otx)) => {
                                        on_new_open_tx(context.clone(), otx);
                                    }
                                    (_, MessageFromHost::CommitOtx(_)) => {
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
}

impl Plugin for P2PRelayer {
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

impl Context {
    pub fn new(
        plugin_name: String,
        config: P2PRelayerConfig,
        service_handler: ServiceHandler,
    ) -> Self {
        Self {
            plugin_name,
            config,
            service_handler,
        }
    }
}

fn on_new_open_tx(context: Context, otx: OpenTransaction) {
    log::trace!("{:?}", context.plugin_name);
    log::trace!("{:?}", context.config);
    log::trace!("{:?}", context.service_handler);
    log::debug!("{:?}", otx);
}
