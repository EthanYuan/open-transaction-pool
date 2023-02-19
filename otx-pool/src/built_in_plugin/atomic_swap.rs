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
use std::sync::atomic::{AtomicU32, Ordering};
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
        let interval_counter = Arc::new(AtomicU32::new(0));
        let context = Context::new(raw_otxs, interval_counter);
        let (msg_handler, request_handler, thread) =
            AtomicSwap::start_process(name, runtime, service_handler, context)?;
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
    fn on_new_open_tx(otx: OpenTransaction, context: Context) {
        context.otx_set.insert(otx);
    }

    fn on_new_intervel(context: Context) {
        let _ = context.interval_counter.fetch_add(1, Ordering::SeqCst);
        if context.interval_counter.load(Ordering::SeqCst) == 5 {
            log::debug!("otx set len: {:?}", context.otx_set.len());
            context.interval_counter.store(0, Ordering::SeqCst);
        }
    }
}
