use config::{CkbConfig, ScriptConfig, ScriptInfo};
use otx_format::jsonrpc_types::{OpenTransaction, OtxBuilder};

use anyhow::{anyhow, Result};
use ckb_jsonrpc_types::{OutputsValidator, TransactionView};
use ckb_sdk_otx::CkbRpcClient;
use ckb_types::core::TransactionBuilder;
use ckb_types::packed::{self, CellOutput, OutPoint};
use ckb_types::prelude::*;
use ckb_types::H256;
use serde::Serialize;

pub fn build_otx(
    inputs: Vec<OutPoint>,
    outputs: Vec<CellOutput>,
    outputs_data: Vec<packed::Bytes>,
    script_infos: Vec<ScriptInfo>,
    script_config: ScriptConfig,
    ckb_config: CkbConfig,
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
    let otx_builder = OtxBuilder::new(script_config);
    let otx = otx_builder
        .tx_view_to_otx(tx, 1, ckb_config.get_ckb_uri())
        .map_err(|err| anyhow!(err.to_string()))?;
    Ok(otx)
}

pub fn merge_otxs(_otxs: Vec<OpenTransaction>) -> Result<OpenTransaction> {
    todo!()
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
