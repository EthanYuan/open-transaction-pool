use crate::const_definition::{OMNI_OPENTX_TX_HASH, OMNI_OPENTX_TX_IDX};

use anyhow::Result;
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    rpc::CkbRpcClient, types::NetworkType, unlock::OmniLockConfig, Address, AddressPayload,
    ScriptId,
};
use ckb_types::{
    core::ScriptHashType,
    packed::{Byte32, CellDep, OutPoint, Script},
    prelude::*,
    H160, H256,
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

pub fn build_otx_omnilock_addr_from_secp(address: &Address, ckb_uri: &str) -> Result<Address> {
    let mut ckb_client = CkbRpcClient::new(ckb_uri);
    let cell = build_cell_dep(&mut ckb_client, &OMNI_OPENTX_TX_HASH, OMNI_OPENTX_TX_IDX)?;
    let mut config = {
        let arg = H160::from_slice(&address.payload().args()).unwrap();
        OmniLockConfig::new_pubkey_hash(arg)
    };
    config.set_opentx_mode();
    let address_payload = {
        let args = config.build_args();
        AddressPayload::new_full(ScriptHashType::Type, cell.type_hash.pack(), args)
    };
    let lock_script = Script::from(&address_payload);
    let address = Address::new(NetworkType::Testnet, address_payload.clone(), true);
    let resp = serde_json::json!({
        "testnet": address.to_string(),
        "lock-arg": format!("0x{}", hex_string(address_payload.args().as_ref())),
        "lock-hash": format!("{:#x}", lock_script.calc_script_hash())
    });
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(address)
}

pub fn build_cell_dep(
    ckb_client: &mut CkbRpcClient,
    tx_hash: &H256,
    index: usize,
) -> Result<ScriptInfo> {
    let out_point_json = ckb_jsonrpc_types::OutPoint {
        tx_hash: tx_hash.clone(),
        index: ckb_jsonrpc_types::Uint32::from(index as u32),
    };
    let cell_status = ckb_client.get_live_cell(out_point_json, false)?;
    let script = Script::from(cell_status.cell.unwrap().output.type_.unwrap());

    let type_hash = script.calc_script_hash();
    let out_point = OutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, index as u32);

    let cell_dep = CellDep::new_builder().out_point(out_point).build();
    Ok(ScriptInfo {
        type_hash: H256::from_slice(type_hash.as_slice())?,
        script_id: ScriptId::new_type(type_hash.unpack()),
        cell_dep,
    })
}
