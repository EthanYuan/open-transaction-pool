pub mod rpc;

use config::{CkbConfig, ScriptConfig};
use otx_format::jsonrpc_types::OpenTransaction;
use otx_format::types::PaymentAmount;
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
pub const MIN_FEE: u64 = 1_0000_0000;

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
    pub sell_udt: Script,
    pub sell_amount: u64,
    pub buy_udt: Script,
    pub buy_amount: u64,
    pub pay_fee: u64,
}

impl SwapProposal {
    fn pair_proposal(&self) -> SwapProposal {
        SwapProposal {
            sell_udt: self.buy_udt.clone(),
            sell_amount: self.buy_amount,
            buy_udt: self.sell_udt.clone(),
            buy_amount: self.sell_amount,
            pay_fee: 0, // no need to consider the fee when doing a pair
        }
    }

    fn cap_match(&self, swap_proposal: SwapProposal) -> bool {
        self.pay_fee + swap_proposal.pay_fee >= MIN_FEE
            && self.buy_udt == swap_proposal.sell_udt
            && self.sell_udt == swap_proposal.buy_udt
            && self.buy_amount == swap_proposal.sell_amount
            && self.sell_amount == swap_proposal.buy_amount
    }
}

impl TryFrom<PaymentAmount> for SwapProposal {
    type Error = String;
    fn try_from(payment_amount: PaymentAmount) -> Result<Self, Self::Error> {
        let asset_types_number = payment_amount.s_udt_amount.len()
            + payment_amount.x_udt_amount.len()
            + usize::from(payment_amount.capacity - payment_amount.fee as i128 != 0);
        if asset_types_number != 2 {
            return Err(format!(
                "The number of asset types must be 2, but got {}",
                asset_types_number
            ));
        }
        let mut swap_proposal = SwapProposal {
            pay_fee: payment_amount.fee,
            ..Default::default()
        };
        if payment_amount.capacity - payment_amount.fee as i128 > 0 {
            swap_proposal.sell_udt = Script::default();
            swap_proposal.sell_amount = payment_amount.capacity as u64 - payment_amount.fee;
        } else {
            swap_proposal.buy_udt = Script::default();
            swap_proposal.buy_amount = payment_amount.capacity as u64 - payment_amount.fee;
        }
        for (type_script, udt_amount) in payment_amount.s_udt_amount {
            if udt_amount > 0 {
                swap_proposal.sell_udt = type_script;
                swap_proposal.sell_amount = udt_amount as u64;
            } else {
                swap_proposal.buy_udt = type_script;
                swap_proposal.buy_amount = (-udt_amount) as u64;
            }
        }
        for (type_script, udt_amount) in payment_amount.x_udt_amount {
            if udt_amount > 0 {
                swap_proposal.sell_udt = type_script;
                swap_proposal.sell_amount = udt_amount as u64;
            } else {
                swap_proposal.buy_udt = type_script;
                swap_proposal.buy_amount = (-udt_amount) as u64;
            }
        }
        if swap_proposal.sell_amount == 0 || swap_proposal.buy_amount == 0 {
            return Err("The amount of sell and buy must be greater than 0".to_owned());
        }
        Ok(swap_proposal)
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone, Serialize, Deserialize)]
pub struct SwapProposalWithOtxs {
    pub swap_proposal: SwapProposal,
    pub otx_ids: Vec<H256>,
}

impl SwapProposalWithOtxs {
    pub fn new(swap_proposal: SwapProposal, otx_ids: Vec<H256>) -> Self {
        Self {
            swap_proposal,
            otx_ids,
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
            payment_amount
        } else {
            return;
        };
        log::info!("payment amount: {:?}", payment_amount);
        let mut swap_proposal: SwapProposal = match payment_amount.try_into() {
            Ok(swap_proposal) => swap_proposal,
            Err(err) => {
                log::error!("parse payment amount error: {:?}", err);
                return;
            }
        };
        log::info!("swap proposal {:?}", swap_proposal);

        let otx_hash = otx.get_tx_hash().expect("get otx tx hash");
        let item = match self.context.proposals.get(&swap_proposal.pair_proposal()) {
            Some(item) => item,
            None => {
                self.context.otxs.insert(otx_hash.clone(), otx);
                swap_proposal.pay_fee = 0;
                self.context
                    .proposals
                    .entry(swap_proposal)
                    .or_insert_with(HashSet::new)
                    .insert(otx_hash);
                return;
            }
        };

        for pair_otx_hash in item.value() {
            log::info!("matched tx: {:#x}", pair_otx_hash);
            let pair_otx = self
                .context
                .otxs
                .get(pair_otx_hash)
                .expect("get pair otx from otxs")
                .value()
                .clone();
            let pair_payment_amount = pair_otx.get_payment_amount().expect("get payment amount");
            let pair_swap_proposal = pair_payment_amount
                .try_into()
                .expect("parse payment amount");
            if !swap_proposal.cap_match(pair_swap_proposal) {
                continue;
            }

            // merge_otx
            let builder = OtxBuilder::new(
                self.context.script_config.clone(),
                self.context.ckb_config.clone(),
            );
            let otx_list = vec![otx.clone(), pair_otx];
            let merged_otx = if let Ok(merged_otx) = builder.merge_otxs_single_acp(otx_list) {
                log::debug!("otxs merge successfully.");
                merged_otx
            } else {
                log::info!("{} failed to merge otxs.", self.context.plugin_name);
                continue;
            };

            // to final tx
            let tx = if let Ok(tx) = merged_otx.try_into() {
                tx
            } else {
                log::info!("failed to generate final tx.");
                continue;
            };

            // send_ckb
            let tx_hash = match send_tx(self.context.ckb_config.get_ckb_uri(), tx) {
                Ok(tx_hash) => tx_hash,
                Err(err) => {
                    log::error!("failed to send final tx: {}", err);
                    continue;
                }
            };
            log::info!("commit final Ckb tx: {:?}", tx_hash.to_string());

            // call host service
            let message = MessageFromPlugin::MergeOtxsAndSentToCkb((
                vec![pair_otx_hash.to_owned(), otx_hash],
                tx_hash,
            ));
            if let Some(MessageFromHost::Ok) = Request::call(&self.context.service_handler, message)
            {
                self.context.otxs.remove(pair_otx_hash);
                self.context.proposals.retain(|_, hashes| {
                    hashes.remove(pair_otx_hash);
                    !hashes.is_empty()
                });
            }

            break;
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
