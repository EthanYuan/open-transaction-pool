mod p2p;

use std::path::PathBuf;

use crate::plugin::{
    host_service::ServiceHandler,
    plugin_proxy::{MsgHandler, PluginState, RequestHandler},
    Plugin,
};
use anyhow::Result;
use ckb_async_runtime::Handle as RuntimeHandle;
use crossbeam_channel::{bounded, unbounded};
use otx_plugin_protocol::PluginInfo;
use utils::config::built_in_plugins::P2PRelayerConfig;

pub struct P2PRelayer {
    info: PluginInfo,
    state: PluginState,
    request_handler: RequestHandler,
    msg_handler: MsgHandler,
}

impl P2PRelayer {
    pub fn new(
        runtime_handle: &RuntimeHandle,
        _service_handler: ServiceHandler,
        config: P2PRelayerConfig,
    ) -> Result<Self> {
        log::info!("P2PRelayer started with config: {:?}", config);
        let name = "p2p_relayer";
        let state = PluginState::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(name, "This plugin relays OTXs via P2P network.", "1.0");

        let (msg_handler, request_handler) = P2PRelayer::start_process(runtime_handle, config)?;
        Ok(P2PRelayer {
            state,
            info,
            msg_handler,
            request_handler,
        })
    }

    fn start_process(
        runtime_handle: &RuntimeHandle,
        config: P2PRelayerConfig,
    ) -> Result<(MsgHandler, RequestHandler)> {
        // the host request channel receives request from host to plugin
        let (host_request_sender, host_request_receiver) = bounded(1);
        // the channel sends notifications or responses from the host to plugin
        let (host_msg_sender, host_msg_receiver) = unbounded();

        // TODO: broadcast new tx

        let mut p2p_builder = p2p::builder();
        if let Some(listen) = config.listen() {
            p2p_builder.listen(listen);
        }
        if let Some(dial) = config.dial() {
            p2p_builder.dial(dial);
        }

        p2p_builder.spawn(runtime_handle, host_request_receiver, host_msg_receiver);

        Ok((host_msg_sender, host_request_sender))
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
