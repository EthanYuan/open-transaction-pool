mod helper;

use helper::{OutputAmount, TxBuilder};

use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{
    HostServiceHandler, MessageFromHost, MessageFromPlugin, Plugin, PluginInfo, PluginMeta,
};
use otx_pool_config::built_in_plugins::DustCollectorConfig;
use otx_pool_config::{CkbConfig, ScriptConfig};
use otx_sdk::build_tx::OtxBuilder;

use anyhow::{anyhow, Result};
use ckb_sdk::rpc::ckb_indexer::{Order, ScriptType, SearchKey};
use ckb_sdk::rpc::IndexerRpcClient;
use ckb_sdk::types::{Address, HumanCapacity};
use ckb_types::core::service::Request;
use ckb_types::packed::Script;
use ckb_types::{packed, H256};
use dashmap::DashMap;

use std::env;
use std::path::PathBuf;

pub const EVERY_INTERVALS: usize = 10;
pub const MIN_PAYMENT: usize = 1_0000_0000;
pub const DEFAULT_FEE: usize = 1000_0000;

#[derive(Clone)]
struct Context {
    plugin_name: String,
    otxs: DashMap<H256, OpenTransaction>,
    default_address: Address,
    ckb_config: CkbConfig,
    script_config: ScriptConfig,
    service_handler: HostServiceHandler,
}

impl Context {
    fn new(
        plugin_name: &str,
        default_address: Address,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
        service_handler: HostServiceHandler,
    ) -> Self {
        Context {
            plugin_name: plugin_name.to_owned(),
            otxs: DashMap::new(),
            default_address,
            ckb_config,
            script_config,
            service_handler,
        }
    }
}

pub struct DustCollector {
    meta: PluginMeta,
    info: PluginInfo,
    context: Context,
}

impl DustCollector {
    pub fn new(
        service_handler: HostServiceHandler,
        config: DustCollectorConfig,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
    ) -> Result<DustCollector> {
        let name = "dust collector";
        let state = PluginMeta::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(
            name,
            "Collect micropayment otx and aggregate them into ckb tx.",
            "1.0",
        );
        let address = env::var(config.get_env_default_address())?
            .parse::<Address>()
            .map_err(|e| anyhow!(e))?;
        let context = Context::new(name, address, ckb_config, script_config, service_handler);
        Ok(DustCollector {
            meta: state,
            info,
            context,
        })
    }
}

impl Plugin for DustCollector {
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
        self.context.otxs.insert(otx_hash, otx);
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

        // merge_otx
        let otx_list: Vec<OpenTransaction> =
            self.context.otxs.iter().map(|otx| otx.clone()).collect();
        let otx_builder = OtxBuilder::new(
            self.context.script_config.clone(),
            self.context.ckb_config.clone(),
        );
        let merged_otx = if let Ok(merged_otx) = otx_builder.merge_otxs_single_acp(otx_list) {
            log::debug!("otxs merge successfully.");
            merged_otx
        } else {
            log::info!(
                "Failed to merge otxs, all otxs staged by {} itself will be cleared.",
                self.context.plugin_name
            );
            self.context.otxs.clear();
            return;
        };

        // find a input cell to receive assets
        let mut indexer = IndexerRpcClient::new(self.context.ckb_config.get_ckb_uri());
        let lock_script: packed::Script = (&self.context.default_address).into();
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
        let aggregator = TxBuilder::new(
            self.context.ckb_config.clone(),
            self.context.script_config.clone(),
        );
        let unsigned_otx = if let Ok(ckb_tx) = aggregator.add_input_and_output(
            merged_otx,
            cell.out_point,
            &self.context.default_address,
            output_amount,
            Script::default(),
            DEFAULT_FEE as u64,
        ) {
            ckb_tx
        } else {
            log::error!("failed to assemble final tx.");
            return;
        };

        // call host service
        let hashes: Vec<H256> = self
            .context
            .otxs
            .iter()
            .map(|otx| otx.get_tx_hash().expect("get tx hash"))
            .collect();
        let message = MessageFromPlugin::NewMergedOtx((unsigned_otx, hashes));
        if let Some(MessageFromHost::Ok) = Request::call(&self.context.service_handler, message) {
            self.context.otxs.clear();
        }
    }
}
