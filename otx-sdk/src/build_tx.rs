use otx_format::jsonrpc_types::tx_view::tx_view_to_otx;
use otx_format::jsonrpc_types::OpenTransaction;
use otx_pool_config::{CkbConfig, ScriptConfig, ScriptInfo};

use anyhow::{anyhow, Result};
use ckb_jsonrpc_types::{OutputsValidator, TransactionView};
use ckb_sdk::CkbRpcClient;
use ckb_types::core;
use ckb_types::core::TransactionBuilder;
use ckb_types::packed::Transaction;
use ckb_types::packed::{self, CellOutput, OutPoint};
use ckb_types::prelude::*;
use ckb_types::H256;
use serde::Serialize;

use std::collections::HashSet;

pub struct OtxBuilder {
    script_config: ScriptConfig,
    ckb_config: CkbConfig,
}

impl OtxBuilder {
    pub fn new(script_config: ScriptConfig, ckb_config: CkbConfig) -> Self {
        Self {
            script_config,
            ckb_config,
        }
    }

    pub fn build_otx(
        &self,
        inputs: Vec<OutPoint>,
        outputs: Vec<CellOutput>,
        outputs_data: Vec<packed::Bytes>,
        script_infos: Vec<ScriptInfo>,
        fee: u64,
    ) -> Result<OpenTransaction> {
        let inputs: Vec<packed::CellInput> = inputs
            .into_iter()
            .map(|out_point| {
                packed::CellInputBuilder::default()
                    .previous_output(out_point)
                    .build()
            })
            .collect();
        let cell_deps: Vec<packed::CellDep> = script_infos
            .into_iter()
            .map(|script_info| script_info.cell_dep)
            .collect();
        let tx = TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs)
            .outputs_data(outputs_data)
            .cell_deps(cell_deps)
            .build()
            .into();
        let otx = tx_view_to_otx(
            tx,
            fee,
            1,
            self.ckb_config.get_ckb_uri(),
            self.script_config.get_sudt_code_hash(),
            self.script_config.get_xudt_rce_code_hash(),
        )
        .map_err(|err| anyhow!(err.to_string()))?;
        Ok(otx)
    }

    pub fn merge_otxs_single_acp(&self, mut otxs: Vec<OpenTransaction>) -> Result<OpenTransaction> {
        if otxs.len() == 1 {
            return Ok(otxs.remove(0));
        }
        let mut txs = vec![];
        let aggregate_count = otxs.len();
        let mut fee = 0;
        for otx in otxs {
            fee += otx.get_max_fee();
            let tx: TransactionView = otx
                .try_into()
                .map_err(|_| anyhow!("otx convert to ckb tx"))?;
            let tx = Transaction::from(tx.inner.clone()).into_view();
            txs.push(tx);
        }

        let mut builder = core::TransactionView::new_advanced_builder();
        #[allow(clippy::mutable_key_type)]
        let mut cell_deps = HashSet::new();
        #[allow(clippy::mutable_key_type)]
        let mut header_deps = HashSet::new();
        for tx in txs.iter() {
            cell_deps.extend(tx.cell_deps());
            header_deps.extend(tx.header_deps());
            builder = builder.inputs(tx.inputs());
            builder = builder.outputs(tx.outputs());
            builder = builder.outputs_data(tx.outputs_data());
            builder = builder.witnesses(tx.witnesses());
        }
        let tx = builder
            .cell_deps(cell_deps)
            .header_deps(header_deps)
            .build()
            .into();
        let otx = tx_view_to_otx(
            tx,
            fee,
            aggregate_count as u32,
            self.ckb_config.get_ckb_uri(),
            self.script_config.get_sudt_code_hash(),
            self.script_config.get_xudt_rce_code_hash(),
        )
        .map_err(|err| anyhow!(err.to_string()))?;
        Ok(otx)
    }

    // Merge otxs into ckb tx to skip the input existence checking
    pub fn merge_otxs_single_acp_into_tx(
        &self,
        mut otxs: Vec<OpenTransaction>,
    ) -> Result<TransactionView> {
        if otxs.len() == 1 {
            return otxs
                .remove(0)
                .try_into()
                .map_err(|_| anyhow!("otx convert to ckb tx"));
        }
        let mut txs = vec![];
        for otx in otxs {
            let tx: TransactionView = otx
                .try_into()
                .map_err(|_| anyhow!("otx convert to ckb tx"))?;
            let tx = Transaction::from(tx.inner.clone()).into_view();
            txs.push(tx);
        }

        let mut builder = core::TransactionView::new_advanced_builder();
        #[allow(clippy::mutable_key_type)]
        let mut cell_deps = HashSet::new();
        #[allow(clippy::mutable_key_type)]
        let mut header_deps = HashSet::new();
        for tx in txs.iter() {
            cell_deps.extend(tx.cell_deps());
            header_deps.extend(tx.header_deps());
            builder = builder.inputs(tx.inputs());
            builder = builder.outputs(tx.outputs());
            builder = builder.outputs_data(tx.outputs_data());
            builder = builder.witnesses(tx.witnesses());
        }
        let tx = builder
            .cell_deps(cell_deps)
            .header_deps(header_deps)
            .build()
            .into();
        Ok(tx)
    }
}
pub fn dump_data<T>(data: &T, file_name: &str) -> Result<()>
where
    T: ?Sized + Serialize,
{
    let json_string = serde_json::to_string_pretty(data)?;
    std::fs::write(file_name, json_string).map_err(Into::into)
}

pub fn send_tx(ckb_uri: &str, tx: TransactionView) -> Result<H256> {
    let outputs_validator = Some(OutputsValidator::Passthrough);
    CkbRpcClient::new(ckb_uri)
        .send_transaction(tx.inner, outputs_validator)
        .map_err(|e| anyhow!(e.to_string()))
}
