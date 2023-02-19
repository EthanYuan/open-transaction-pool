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

pub struct DustCollector {
    state: PluginState,
    info: PluginInfo,

    /// Send request to stdin thread, and expect a response from stdout thread.
    request_handler: RequestHandler,

    /// Send notifaction/response to stdin thread.
    msg_handler: MsgHandler,

    _thread: JoinHandle<()>,

    _raw_otxs: Arc<DashSet<OpenTransaction>>,
    _interval_counter: Arc<AtomicU32>,
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
        runtime_handle: RuntimeHandle,
        service_handler: ServiceHandler,
    ) -> Result<DustCollector, String> {
        let name = "dust collector";
        let state = PluginState::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(
            name,
            "Collect micropayment otx and aggregate them into ckb tx.",
            "1.0",
        );
        let raw_otxs = Arc::new(DashSet::default());
        let interval_counter = Arc::new(AtomicU32::new(0));
        let context = Context::new(raw_otxs.clone(), interval_counter.clone());
        let (msg_handler, request_handler, thread) =
            DustCollector::start_process(name, runtime_handle, service_handler, context)?;
        Ok(DustCollector {
            state,
            info,
            msg_handler,
            request_handler,
            _thread: thread,
            _raw_otxs: raw_otxs,
            _interval_counter: interval_counter,
        })
    }
}

impl BuiltInPlugin for DustCollector {
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
