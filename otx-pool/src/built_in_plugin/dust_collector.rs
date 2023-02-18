use crate::notify::RuntimeHandle;
use crate::plugin::host_service::ServiceHandler;
use crate::plugin::plugin_proxy::{MsgHandler, PluginState, RequestHandler};
use crate::plugin::Plugin;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{MessageFromHost, MessageFromPlugin, PluginInfo};

use ckb_types::core::service::Request;
use crossbeam_channel::{bounded, select, unbounded};
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

#[derive(Clone)]
pub struct Context {
    pub otx_set: Arc<DashSet<OpenTransaction>>,
    pub interval_counter: Arc<AtomicU32>,
}

impl Context {
    fn new(otx_set: Arc<DashSet<OpenTransaction>>, interval_counter: Arc<AtomicU32>) -> Self {
        Context {
            otx_set,
            interval_counter,
        }
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

    pub fn get_plugin_info(&self) -> PluginInfo {
        self.info.clone()
    }

    pub fn get_plugin_state(&self) -> PluginState {
        self.state.clone()
    }

    pub fn start_process(
        plugin_name: &str,
        runtime: RuntimeHandle,
        _service_handler: ServiceHandler,
        context: Context,
    ) -> Result<(MsgHandler, RequestHandler, JoinHandle<()>), String> {
        // the host request channel receives request from host to plugin
        let (host_request_sender, host_request_receiver) = bounded(1);
        // the channel sends notifications or responses from the host to plugin
        let (host_msg_sender, host_msg_receiver) = unbounded();

        let plugin_name = plugin_name.to_owned();
        // this thread processes information from host to plugin
        let thread = runtime.spawn(async move {
            let do_select = || -> Result<bool, String> {
                select! {
                    // request from host to plugin
                    recv(host_request_receiver) -> msg => {
                        match msg {
                            Ok(Request { responder, arguments }) => {
                                log::debug!("dust collector receives request arguments: {:?}", arguments);
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
                                log::debug!("dust collector receivers msg: {:?}", msg);
                                match msg {
                                    (_, MessageFromHost::NewInterval) => {
                                        on_new_intervel(context.clone());
                                    }
                                    (_, MessageFromHost::NewOtx(otx)) => {
                                        on_new_open_tx(otx, context.clone());
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
                        log::info!("plugin {} quit", plugin_name);
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
