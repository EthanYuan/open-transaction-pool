use anyhow::Result;
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{rpc::CkbRpcClient, unlock::OmniLockConfig, Address, ScriptId};
use ckb_types::{
    packed::{Byte32, CellDep, OutPoint, Script},
    prelude::*,
    H256,
};
use serde::{Deserialize, Serialize};

pub struct ScriptInfo {
    pub type_hash: H256,
    pub script_id: ScriptId,
    pub cell_dep: CellDep,
}

pub struct MultiSigArgs {
    /// Require first n signatures of corresponding pubkey
    pub require_first_n: u8,

    /// Multisig threshold
    pub threshold: u8,

    /// Normal sighash address
    pub sighash_address: Vec<Address>,
}

#[derive(Serialize, Deserialize)]
pub struct TxInfo {
    pub tx: json_types::TransactionView,
    pub omnilock_config: OmniLockConfig,
}

pub fn build_cell_dep(
    ckb_client: &mut CkbRpcClient,
    tx_hash: &H256,
    index: u32,
) -> Result<ScriptInfo> {
    let out_point_json = ckb_jsonrpc_types::OutPoint {
        tx_hash: tx_hash.clone(),
        index: ckb_jsonrpc_types::Uint32::from(index),
    };
    let cell_status = ckb_client.get_live_cell(out_point_json, false)?;
    let script = Script::from(cell_status.cell.unwrap().output.type_.unwrap());

    let type_hash = script.calc_script_hash();
    let out_point = OutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, index);

    let cell_dep = CellDep::new_builder().out_point(out_point).build();
    Ok(ScriptInfo {
        type_hash: H256::from_slice(type_hash.as_slice())?,
        script_id: ScriptId::new_type(type_hash.unpack()),
        cell_dep,
    })
}
