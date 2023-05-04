use crate::plugin::host_service::ServiceHandler;
use crate::plugin::plugin_proxy::{MsgHandler, PluginState, RequestHandler};
use crate::plugin::Plugin;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{MessageFromHost, MessageFromPlugin, PluginInfo};
use utils::aggregator::{OtxAggregator, OutputAmount};
use utils::config::built_in_plugins::DustCollectorConfig;
use utils::config::{CkbConfig, ScriptConfig};

use anyhow::{anyhow, Result};
use ckb_sdk::rpc::ckb_indexer::{Order, ScriptType, SearchKey};
use ckb_sdk::rpc::IndexerRpcClient;
use ckb_sdk_open_tx::types::{Address, HumanCapacity};
use ckb_types::core::service::Request;
use ckb_types::packed::Script;
use ckb_types::{packed, H256};
use crossbeam_channel::{bounded, select, unbounded};
use dashmap::DashMap;

use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

pub const EVERY_INTERVALS: usize = 10;
pub const MIN_PAYMENT: usize = 1_0000_0000;
pub const DEFAULT_FEE: usize = 1000_0000;

#[derive(Clone)]
struct Context {
    plugin_name: String,
    otxs: Arc<DashMap<H256, OpenTransaction>>,
    default_address: Address,
    ckb_config: CkbConfig,
    script_config: ScriptConfig,
    service_handler: ServiceHandler,
}

impl Context {
    fn new(
        plugin_name: &str,
        default_address: Address,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
        service_handler: ServiceHandler,
    ) -> Self {
        Context {
            plugin_name: plugin_name.to_owned(),
            otxs: Arc::new(DashMap::new()),
            default_address,
            ckb_config,
            script_config,
            service_handler,
        }
    }
}

pub struct DustCollector {
    state: PluginState,
    info: PluginInfo,

    /// Send request to plugin thread, and expect a response.
    request_handler: RequestHandler,

    /// Send notifaction/response to plugin thread.
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
        service_handler: ServiceHandler,
        config: DustCollectorConfig,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
    ) -> Result<DustCollector> {
        let name = "dust collector";
        let state = PluginState::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(
            name,
            "Collect micropayment otx and aggregate them into ckb tx.",
            "1.0",
        );
        let address = env::var(config.get_env_default_address())?
            .parse::<Address>()
            .map_err(|e| anyhow!(e))?;

        let (msg_handler, request_handler, thread) = DustCollector::start_process(Context::new(
            name,
            address,
            ckb_config,
            script_config,
            service_handler,
        ))?;
        Ok(DustCollector {
            state,
            info,
            msg_handler,
            request_handler,
            _thread: thread,
        })
    }
}

impl DustCollector {
    fn start_process(context: Context) -> Result<(MsgHandler, RequestHandler, JoinHandle<()>)> {
        // the host request channel receives request from host to plugin
        let (host_request_sender, host_request_receiver) = bounded(1);
        // the channel sends notifications or responses from the host to plugin
        let (host_msg_sender, host_msg_receiver) = unbounded();

        let plugin_name = context.plugin_name.to_owned();
        // this thread processes information from host to plugin
        let thread = thread::spawn(move || {
            let do_select = || -> Result<bool> {
                select! {
                    // request from host to plugin
                    recv(host_request_receiver) -> msg => {
                        match msg {
                            Ok(Request { responder, arguments }) => {
                                log::debug!("{} receives request arguments: {:?}", context.plugin_name, arguments);
                                // handle
                                let response = (0, MessageFromPlugin::Ok);
                                responder.send(response)?;
                                Ok(false)
                            }
                            Err(err) => Err(anyhow!(err.to_string()))
                        }
                    }
                    // repsonse/notification from host to plugin
                    recv(host_msg_receiver) -> msg => {
                        match msg {
                            Ok(msg) => {
                                match msg {
                                    (_, MessageFromHost::NewInterval(elapsed)) => {
                                        on_new_intervel(context.clone(), elapsed);
                                    }
                                    (_, MessageFromHost::NewOtx(otx)) => {
                                        log::info!("{} receivers msg NewOtx hash: {:?}",
                                            context.plugin_name,
                                            otx.get_tx_hash().expect("get tx hash"));
                                        on_new_open_tx(context.clone(), otx);
                                    }
                                    (_, MessageFromHost::CommitOtx(otx_hashes)) => {
                                        on_commit_open_tx(context.clone(), otx_hashes);
                                    }
                                    _ => unreachable!(),
                                }
                                Ok(false)
                            }
                            Err(err) => Err(anyhow!(err.to_string()))
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

fn on_new_open_tx(context: Context, otx: OpenTransaction) {
    if !otx.get_pending_signature_locks().is_empty() {
        return;
    }
    if let Ok(aggregate_count) = otx.get_aggregate_count() {
        log::info!("aggregate count: {:?}", aggregate_count);
        if aggregate_count > 1 {
            return;
        }
    }
    if let Ok(payment_amount) = otx.get_payment_amount() {
        log::info!("payment: {:?}", payment_amount);
        if payment_amount.capacity < MIN_PAYMENT as i128
            || !payment_amount.x_udt_amount.is_empty()
            || !payment_amount.s_udt_amount.is_empty()
        {
            return;
        }
    } else {
        return;
    };
    let otx_hash = otx.get_tx_hash().expect("get tx hash");
    context.otxs.insert(otx_hash, otx);
    log::info!("on_new_open_tx, index otxs count: {:?}", context.otxs.len());
}

fn on_commit_open_tx(context: Context, otx_hashes: Vec<H256>) {
    log::info!(
        "{} on commit open tx remove committed otx: {:?}",
        context.plugin_name,
        otx_hashes
            .iter()
            .map(|hash| hash.to_string())
            .collect::<Vec<String>>()
    );
    otx_hashes.iter().for_each(|otx_hash| {
        context.otxs.remove(otx_hash);
    })
}

fn on_new_intervel(context: Context, elapsed: u64) {
    if elapsed % EVERY_INTERVALS as u64 != 0 || context.otxs.len() <= 1 {
        return;
    }

    log::info!(
        "on new {} intervals otx set len: {:?}",
        EVERY_INTERVALS,
        context.otxs.len()
    );

    // merge_otx
    let otx_list: Vec<OpenTransaction> = context.otxs.iter().map(|otx| otx.clone()).collect();
    let aggregator = OtxAggregator::new(context.ckb_config.clone(), context.script_config.clone());
    let merged_otx = if let Ok(merged_otx) = aggregator.merge_otxs(otx_list) {
        log::debug!("otxs merge successfully.");
        merged_otx
    } else {
        log::info!(
            "Failed to merge otxs, all otxs staged by {} itself will be cleared.",
            context.plugin_name
        );
        context.otxs.clear();
        return;
    };

    // find a input cell to receive assets
    let mut indexer = IndexerRpcClient::new(context.ckb_config.get_ckb_uri());
    let lock_script: packed::Script = (&context.default_address).into();
    let search_key = SearchKey {
        script: lock_script.into(),
        script_type: ScriptType::Lock,
        filter: None,
        with_data: None,
        group_by_transaction: None,
    };
    let cell = if let Ok(cell) = indexer.get_cells(search_key, Order::Asc, 1.into(), None) {
        let cell = &cell.objects[0];
        log::info!(
            "the broker identified an available cell: {:?}",
            cell.out_point
        );
        cell.clone()
    } else {
        log::error!("broker has no cells available for input");
        return;
    };

    // add input and output
    let receive_ckb_capacity = merged_otx.get_payment_amount().unwrap().capacity;
    let output_capacity =
        receive_ckb_capacity as u64 + cell.output.capacity.value() - DEFAULT_FEE as u64;
    let output_amount = OutputAmount {
        capacity: HumanCapacity::from(output_capacity),
        udt_amount: None,
    };
    let unsigned_otx = if let Ok(ckb_tx) = aggregator.add_input_and_output(
        merged_otx,
        cell.out_point,
        &context.default_address,
        output_amount,
        Script::default(),
    ) {
        ckb_tx
    } else {
        log::error!("failed to assemble final tx.");
        return;
    };

    // call host service
    let hashes: Vec<H256> = context
        .otxs
        .iter()
        .map(|otx| otx.get_tx_hash().expect("get tx hash"))
        .collect();
    let message = MessageFromPlugin::NewMergedOtx((unsigned_otx, hashes));
    if let Some(MessageFromHost::Ok) = Request::call(&context.service_handler, message) {
        context.otxs.clear();
    }
}
