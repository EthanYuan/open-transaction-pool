use crate::built_in_plugin::dust_collector::MIN_PAYMENT;
use crate::plugin::host_service::ServiceHandler;
use crate::plugin::plugin_proxy::{MsgHandler, PluginState, RequestHandler};
use crate::plugin::Plugin;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{MessageFromHost, MessageFromPlugin, PluginInfo};
use utils::aggregator::{Committer, OtxAggregator, SignInfo};
use utils::config::{CkbConfig, ScriptConfig};

use ckb_jsonrpc_types::Script;
use ckb_sdk_open_tx::Address;
use ckb_types::core::service::Request;
use ckb_types::H256;
use crossbeam_channel::{bounded, select, unbounded};
use dashmap::DashMap;

use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

pub const EVERY_INTERVALS: usize = 10;

#[derive(Clone)]
struct Context {
    plugin_name: String,
    sign_info: SignInfo,
    ckb_config: CkbConfig,
    script_config: ScriptConfig,
    service_handler: ServiceHandler,

    otxs: Arc<DashMap<H256, OpenTransaction>>,
    orders: Arc<DashMap<OrderKey, HashSet<H256>>>,
}

impl Context {
    fn new(
        plugin_name: &str,
        sign_info: SignInfo,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
        service_handler: ServiceHandler,
    ) -> Self {
        Context {
            plugin_name: plugin_name.to_owned(),
            sign_info,
            ckb_config,
            script_config,
            service_handler,
            otxs: Arc::new(DashMap::new()),
            orders: Arc::new(DashMap::new()),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone)]
struct OrderKey {
    sell_udt: Script,
    sell_amount: u128,
    buy_udt: Script,
    buy_amount: u128,
}

impl OrderKey {
    fn pair_order(&self) -> OrderKey {
        OrderKey {
            sell_udt: self.buy_udt.clone(),
            sell_amount: self.buy_amount,
            buy_udt: self.sell_udt.clone(),
            buy_amount: self.sell_amount,
        }
    }
}

pub struct AtomicSwap {
    state: PluginState,
    info: PluginInfo,

    /// Send request to plugin thread, and expect a response.
    request_handler: RequestHandler,

    /// Send notifaction/response to plugin thread.
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
        service_handler: ServiceHandler,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
    ) -> Result<AtomicSwap, String> {
        let name = "atomic swap";
        let state = PluginState::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(
            name,
            "One kind of xUDT can be used to swap another kind of xUDT.",
            "1.0",
        );
        let (msg_handler, request_handler, thread) = AtomicSwap::start_process(Context::new(
            name,
            SignInfo::new(
                &Address::from_str(
                    "ckb1qgqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqhzeqga", // TODO: refactor
                )
                .unwrap(),
                &H256::default(),
                ckb_config.clone(),
            ),
            ckb_config,
            script_config,
            service_handler,
        ))?;
        Ok(AtomicSwap {
            state,
            info,
            msg_handler,
            request_handler,
            _thread: thread,
        })
    }
}

impl AtomicSwap {
    fn start_process(
        context: Context,
    ) -> Result<(MsgHandler, RequestHandler, JoinHandle<()>), String> {
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
                                    (_, MessageFromHost::NewInterval(elapsed)) => {
                                        on_new_intervel(context.clone(), elapsed);
                                    }
                                    (_, MessageFromHost::NewOtx(otx)) => {
                                        log::info!("{} receivers msg NewOtx hash: {:?}",
                                            context.plugin_name,
                                            otx.get_tx_hash().expect("get otx tx hash").to_string());
                                        on_new_open_tx(context.clone(), otx);
                                    }
                                    (_, MessageFromHost::CommitOtx(otx_hashes)) => {
                                        on_commit_open_tx(context.clone(), otx_hashes);
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

fn on_new_open_tx(context: Context, otx: OpenTransaction) {
    log::info!("on_new_open_tx, index otxs count: {:?}", context.otxs.len());
    if let Ok(aggregate_count) = otx.get_aggregate_count() {
        log::info!("aggregate count: {:?}", aggregate_count);
        if aggregate_count > 1 {
            return;
        }
    }
    let payment_amount = if let Ok(payment_amount) = otx.get_payment_amount() {
        log::info!("payment: {:?}", payment_amount);
        if payment_amount.capacity <= 0
            || payment_amount.capacity > MIN_PAYMENT as i128
            || payment_amount.x_udt_amount.len() != 2
            || !payment_amount.s_udt_amount.is_empty()
        {
            return;
        }
        payment_amount
    } else {
        return;
    };

    let mut order_key = OrderKey::default();
    for (type_script, udt_amount) in payment_amount.x_udt_amount {
        if udt_amount > 0 {
            order_key.sell_udt = type_script;
            order_key.sell_amount = udt_amount as u128;
        } else {
            order_key.buy_udt = type_script;
            order_key.buy_amount = (-udt_amount) as u128;
        }
    }
    if order_key.sell_amount == 0 || order_key.buy_amount == 0 {
        return;
    }

    let otx_hash = otx.get_tx_hash().expect("get otx tx hash");
    if let Some(item) = context.orders.get(&order_key.pair_order()) {
        if let Some(pair_tx_hash) = item.value().iter().next() {
            log::info!("matched tx: {:#x}", pair_tx_hash);
            let pair_otx = context.otxs.get(pair_tx_hash).unwrap().value().clone();

            // merge_otx
            let otx_list = vec![otx, pair_otx];
            let aggregator = OtxAggregator::new(
                context.sign_info.secp_address(),
                context.sign_info.privkey(),
                context.ckb_config.clone(),
                context.script_config,
            );
            let merged_otx = if let Ok(merged_otx) = aggregator.merge_otxs(otx_list) {
                log::debug!("otxs merge successfully.");
                merged_otx
            } else {
                log::info!("{} failed to merge otxs.", context.plugin_name);
                return;
            };

            // to final tx
            let tx = if let Ok(tx) = merged_otx.try_into() {
                tx
            } else {
                log::info!("Failed to generate final tx.");
                return;
            };

            // send_ckb
            let committer = Committer::new(context.ckb_config.get_ckb_uri());
            let tx_hash = if let Ok(tx_hash) = committer.send_tx(tx) {
                tx_hash
            } else {
                log::error!("failed to send final tx.");
                return;
            };
            log::info!("commit final Ckb tx: {:?}", tx_hash.to_string());

            // call host service
            let message = MessageFromPlugin::SendCkbTxWithOtxs((
                tx_hash,
                vec![pair_tx_hash.to_owned(), otx_hash],
            ));
            if let Some(MessageFromHost::Ok) = Request::call(&context.service_handler, message) {
                context.otxs.remove(pair_tx_hash);
                context.orders.retain(|_, hashes| {
                    hashes.remove(pair_tx_hash);
                    !hashes.is_empty()
                });
            }
        }
    } else {
        context.otxs.insert(otx_hash.clone(), otx);
        context
            .orders
            .entry(order_key)
            .or_insert_with(HashSet::new)
            .insert(otx_hash);
    }
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
}
