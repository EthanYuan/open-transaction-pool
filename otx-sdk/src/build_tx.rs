use config::{CkbConfig, ScriptConfig, ScriptInfo};
use otx_format::jsonrpc_types::{OpenTransaction, OtxBuilder};

use anyhow::{anyhow, Result};
use ckb_types::core::TransactionBuilder;
use ckb_types::packed::{self, CellOutput, OutPoint};
use ckb_types::prelude::*;

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
        .build();
    let otx_builder = OtxBuilder::new(script_config);
    let otx = otx_builder
        .tx_view_to_otx(tx.into(), 1, ckb_config.get_ckb_uri())
        .map_err(|err| anyhow!(err.to_string()))?;
    Ok(otx)
}
