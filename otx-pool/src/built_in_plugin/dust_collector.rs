use super::{BuiltInPlugin, Context};
use crate::notify::RuntimeHandle;
use crate::plugin::host_service::ServiceHandler;
use crate::plugin::plugin_proxy::{MsgHandler, PluginState, RequestHandler};
use crate::plugin::Plugin;

use utils::aggregator::SecpSignInfo;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::PluginInfo;

use dashmap::DashSet;
use tokio::task::JoinHandle;

use std::path::PathBuf;
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
    secp_sign_info: SecpSignInfo,
    ckb_uri: String,
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
        secp_sign_info: SecpSignInfo,
        ckb_uri: &str,
    ) -> Result<DustCollector, String> {
        let name = "dust collector";
        let state = PluginState::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(
            name,
            "Collect micropayment otx and aggregate them into ckb tx.",
            "1.0",
        );
        let raw_otxs = Arc::new(DashSet::default());
        let context = Context::new(raw_otxs.clone());
        let (msg_handler, request_handler, thread) =
            DustCollector::start_process(context, name, runtime_handle, service_handler)?;
        Ok(DustCollector {
            state,
            info,
            msg_handler,
            request_handler,
            _thread: thread,
            _raw_otxs: raw_otxs,
            secp_sign_info,
            ckb_uri: ckb_uri.to_owned(),
        })
    }
}

impl BuiltInPlugin for DustCollector {
    fn on_new_open_tx(context: Context, otx: OpenTransaction) {
        context.otx_set.insert(otx);
    }

    fn on_new_intervel(context: Context, elapsed: u64) {
        if elapsed % 10 == 0 {
            log::debug!("otx set len: {:?}", context.otx_set.len());

            // merge_otx
            // OtxAggregator::new(
            //     self.secp_sign_info.secp_address(),
            //     self.secp_sign_info.privkey(),
            //     &self.ckb_uri,
            // );

            // add inputs and outputs

            // send_ckb

            // notify service
            // the ckb tx and otxs merged
            // service notify the remove event
        }
    }
}
