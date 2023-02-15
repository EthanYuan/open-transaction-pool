use crate::notify::RuntimeHandle;
use crate::plugin::host_service::ServiceHandler;
use crate::plugin::plugin_proxy::{MsgHandler, PluginState, RequestHandler};
use crate::plugin::Plugin;

use otx_plugin_protocol::PluginInfo;

use tokio::task::JoinHandle;

use std::path::PathBuf;

pub struct DustCollector {
    state: PluginState,
    info: PluginInfo,

    /// Send request to stdin thread, and expect a response from stdout thread.
    request_handler: RequestHandler,

    /// Send notifaction/response to stdin thread.
    msg_handler: MsgHandler,

    _thread: JoinHandle<()>,
}

impl Plugin for DustCollector {
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

impl DustCollector {
    pub fn new(
        runtime: RuntimeHandle,
        service_handler: ServiceHandler,
    ) -> Result<DustCollector, String> {
        let state = PluginState::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(
            "dust collector",
            "Collect micropayment otx and aggregate them into ckb tx.",
            "1.0",
        );
        let (msg_handler, request_handler, thread) =
            DustCollector::start_process(runtime, service_handler)?;
        Ok(DustCollector {
            state,
            info,
            msg_handler,
            request_handler,
            _thread: thread,
        })
    }

    pub fn start_process(
        _runtime: RuntimeHandle,
        _service_handler: ServiceHandler,
    ) -> Result<(MsgHandler, RequestHandler, JoinHandle<()>), String> {
        todo!()
    }

    pub fn get_plugin_info(&self) -> PluginInfo {
        self.info.clone()
    }

    pub fn get_plugin_state(&self) -> PluginState {
        self.state.clone()
    }
}
