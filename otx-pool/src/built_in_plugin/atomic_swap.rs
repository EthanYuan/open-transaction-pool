use super::{BuiltInPlugin, Context};
use crate::notify::RuntimeHandle;
use crate::plugin::host_service::ServiceHandler;
use crate::plugin::plugin_proxy::{MsgHandler, PluginState, RequestHandler};
use crate::plugin::Plugin;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::PluginInfo;

use dashmap::DashSet;
use tokio::task::JoinHandle;

use std::path::PathBuf;
use std::sync::Arc;

pub struct AtomicSwap {
    state: PluginState,
    info: PluginInfo,

    /// Send request to stdin thread, and expect a response from stdout thread.
    request_handler: RequestHandler,

    /// Send notifaction/response to stdin thread.
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
    pub fn new(
        runtime: RuntimeHandle,
        service_handler: ServiceHandler,
    ) -> Result<AtomicSwap, String> {
        let name = "atomic swap";
        let state = PluginState::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(
            name,
            "Atomic swap engine merges matched asset swaps open transactions.",
            "1.0",
        );
        let raw_otxs = Arc::new(DashSet::default());
        let context = Context::new(raw_otxs);
        let (msg_handler, request_handler, thread) =
            AtomicSwap::start_process(context, name, runtime, service_handler)?;
        Ok(AtomicSwap {
            state,
            info,
            msg_handler,
            request_handler,
            _thread: thread,
        })
    }
}
impl BuiltInPlugin for AtomicSwap {
    fn on_new_open_tx(context: Context, otx: OpenTransaction) {
        context.otx_set.insert(otx);
    }

    fn on_new_intervel(context: Context, elapsed: u64) {
        if elapsed % 10 == 0 {
            log::debug!("otx set len: {:?}", context.otx_set.len());
        }
    }
}
