pub mod atomic_swap;
pub mod dust_collector;

pub use dust_collector::DustCollector;

use crate::notify::RuntimeHandle;
use crate::plugin::host_service::ServiceHandler;
use crate::plugin::plugin_proxy::{MsgHandler, RequestHandler};

use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{MessageFromHost, MessageFromPlugin};

use ckb_types::core::service::Request;
use crossbeam_channel::{bounded, select, unbounded};
use dashmap::DashSet;
use tokio::task::JoinHandle;

use std::sync::Arc;

#[derive(Clone)]
pub struct Context {
    pub otx_set: Arc<DashSet<OpenTransaction>>,
}

impl Context {
    fn new(otx_set: Arc<DashSet<OpenTransaction>>) -> Self {
        Context { otx_set }
    }
}

pub trait BuiltInPlugin {
    fn on_new_open_tx(context: Context, otx: OpenTransaction);
    fn on_new_intervel(context: Context, elapsed: u64);
    fn start_process(
        context: Context,
        plugin_name: &str,
        runtime: RuntimeHandle,
        _service_handler: ServiceHandler,
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
                                    (_, MessageFromHost::NewInterval(elapsed)) => {
                                        Self::on_new_intervel(context.clone(), elapsed);
                                    }
                                    (_, MessageFromHost::NewOtx(otx)) => {
                                        Self::on_new_open_tx(context.clone(), otx);
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
