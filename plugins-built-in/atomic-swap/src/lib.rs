pub mod rpc;

use config::{CkbConfig, ScriptConfig};
use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{
    HostServiceHandler, MessageFromHost, MessageFromPlugin, Plugin, PluginInfo, PluginMeta,
};
use otx_sdk::build_tx::send_tx;
use otx_sdk::build_tx::OtxBuilder;

use ckb_jsonrpc_types::Script;
use ckb_types::core::service::Request;
use ckb_types::H256;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use std::collections::HashSet;
use std::path::PathBuf;

pub const EVERY_INTERVALS: usize = 10;
pub const MIN_PAYMENT: usize = 1_0000_0000;

#[derive(Clone)]
struct Context {
    plugin_name: String,
    ckb_config: CkbConfig,
    script_config: ScriptConfig,
    service_handler: HostServiceHandler,

    otxs: DashMap<H256, OpenTransaction>,
    proposals: DashMap<SwapProposal, HashSet<H256>>,
}

impl Context {
    fn new(
        plugin_name: &str,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
        service_handler: HostServiceHandler,
    ) -> Self {
        Context {
            plugin_name: plugin_name.to_owned(),
            ckb_config,
            script_config,
            service_handler,
            otxs: DashMap::new(),
            proposals: DashMap::new(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone, Serialize, Deserialize)]
pub struct SwapProposal {
    sell_udt: Script,
    sell_amount: u128,
    buy_udt: Script,
    buy_amount: u128,
}

impl SwapProposal {
    fn pair_order(&self) -> SwapProposal {
        SwapProposal {
            sell_udt: self.buy_udt.clone(),
            sell_amount: self.buy_amount,
            buy_udt: self.sell_udt.clone(),
            buy_amount: self.sell_amount,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone, Serialize, Deserialize)]
pub struct SwapProposalWithCount {
    swap_proposal: SwapProposal,
    count: usize,
}

impl SwapProposalWithCount {
    pub fn new(swap_proposal: SwapProposal, count: usize) -> Self {
        Self {
            swap_proposal,
            count,
        }
    }
}

pub struct AtomicSwap {
    meta: PluginMeta,
    info: PluginInfo,
    context: Context,
}

impl AtomicSwap {
    pub fn new(
        service_handler: HostServiceHandler,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
    ) -> Result<AtomicSwap, String> {
        let name = "atomic swap";
        let state = PluginMeta::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(
            name,
            "One kind of UDT can be used to swap another kind of UDT.",
            "1.0",
        );
        let context = Context::new(name, ckb_config, script_config, service_handler);
        Ok(AtomicSwap {
            meta: state,
            info,
            context,
        })
    }
}

impl Plugin for AtomicSwap {
    fn get_name(&self) -> String {
        self.info.name.clone()
    }

    fn get_info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn get_meta(&self) -> PluginMeta {
        self.meta.clone()
    }

    fn on_new_otx(&self, otx: OpenTransaction) {
        log::info!(
            "on_new_open_tx, index otxs count: {:?}",
            self.context.otxs.len()
        );
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
                || payment_amount.s_udt_amount.len() != 2
                || !payment_amount.x_udt_amount.is_empty()
            {
                return;
            }
            payment_amount
        } else {
            return;
        };

        let mut order_key = SwapProposal::default();
        for (type_script, udt_amount) in payment_amount.s_udt_amount {
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
        if let Some(item) = self.context.proposals.get(&order_key.pair_order()) {
            if let Some(pair_tx_hash) = item.value().iter().next() {
                log::info!("matched tx: {:#x}", pair_tx_hash);
                let pair_otx = self.context.otxs.get(pair_tx_hash).unwrap().value().clone();

                // merge_otx
                let builder = OtxBuilder::new(
                    self.context.script_config.clone(),
                    self.context.ckb_config.clone(),
                );
                let otx_list = vec![otx, pair_otx];
                let merged_otx = if let Ok(merged_otx) = builder.merge_otxs_single_acp(otx_list) {
                    log::debug!("otxs merge successfully.");
                    merged_otx
                } else {
                    log::info!("{} failed to merge otxs.", self.context.plugin_name);
                    return;
                };

                // to final tx
                let tx = if let Ok(tx) = merged_otx.try_into() {
                    tx
                } else {
                    log::info!("failed to generate final tx.");
                    return;
                };

                // send_ckb
                let tx_hash = match send_tx(self.context.ckb_config.get_ckb_uri(), tx) {
                    Ok(tx_hash) => tx_hash,
                    Err(err) => {
                        log::error!("failed to send final tx: {}", err);
                        return;
                    }
                };
                log::info!("commit final Ckb tx: {:?}", tx_hash.to_string());

                // call host service
                let message = MessageFromPlugin::MergeOtxsAndSentToCkb((
                    vec![pair_tx_hash.to_owned(), otx_hash],
                    tx_hash,
                ));
                if let Some(MessageFromHost::Ok) =
                    Request::call(&self.context.service_handler, message)
                {
                    self.context.otxs.remove(pair_tx_hash);
                    self.context.proposals.retain(|_, hashes| {
                        hashes.remove(pair_tx_hash);
                        !hashes.is_empty()
                    });
                }
            }
        } else {
            self.context.otxs.insert(otx_hash.clone(), otx);
            self.context
                .proposals
                .entry(order_key)
                .or_insert_with(HashSet::new)
                .insert(otx_hash);
        }
    }

    fn on_commit_otx(&self, otx_hashes: Vec<H256>) {
        log::info!(
            "{} on commit open tx remove committed otx: {:?}",
            self.context.plugin_name,
            otx_hashes
                .iter()
                .map(|hash| hash.to_string())
                .collect::<Vec<String>>()
        );
        otx_hashes.iter().for_each(|otx_hash| {
            self.context.otxs.remove(otx_hash);
            self.context.proposals.retain(|_, hashes| {
                hashes.remove(otx_hash);
                !hashes.is_empty()
            });
        })
    }

    fn on_new_intervel(&self, elapsed: u64) {
        if elapsed % EVERY_INTERVALS as u64 != 0 || self.context.otxs.len() <= 1 {
            return;
        }

        log::info!(
            "on new {} intervals otx set len: {:?}",
            EVERY_INTERVALS,
            self.context.otxs.len()
        );
    }
}
