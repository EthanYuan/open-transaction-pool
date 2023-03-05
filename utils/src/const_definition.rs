use ckb_types::{packed, prelude::*, H256};
use once_cell::sync::OnceCell;

use std::collections::HashMap;

pub static CKB_URI: OnceCell<String> = OnceCell::new();

// script code hash
pub static SECP256K1_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static XUDT_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static SUDT_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static OMNI_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static ANYONE_CAN_PAY_CODE_HASH: OnceCell<H256> = OnceCell::new();

// cell deps
pub static XUDT_CELL_DEP_TX_HASH: OnceCell<H256> = OnceCell::new();
pub static XUDT_CELL_DEP_TX_IDX: OnceCell<usize> = OnceCell::new();
pub static OMNI_OPENTX_CELL_DEP_TX_HASH: OnceCell<H256> = OnceCell::new();
pub static OMNI_OPENTX_CELL_DEP_TX_IDX: OnceCell<usize> = OnceCell::new();
pub static SECP_DATA_CELL_DEP_TX_HASH: OnceCell<H256> = OnceCell::new();
pub static SECP_DATA_CELL_DEP_TX_IDX: OnceCell<usize> = OnceCell::new();

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptInfo {
    pub script: packed::Script,
    pub cell_dep: packed::CellDep,
}

pub fn load_code_hash(script_info: HashMap<String, ScriptInfo>) {
    let _ = SECP256K1_CODE_HASH.set(
        script_info
            .get("secp256k1_blake160")
            .cloned()
            .expect("get secp256k1 code hash")
            .script
            .code_hash()
            .unpack(),
    );
    let _ = XUDT_CODE_HASH.set(
        script_info
            .get("xudt_rce")
            .cloned()
            .expect("get xudt script info")
            .script
            .code_hash()
            .unpack(),
    );
    let _ = OMNI_CODE_HASH.set(
        script_info
            .get("omni_lock")
            .cloned()
            .expect("get omni script info")
            .script
            .code_hash()
            .unpack(),
    );
    let _ = ANYONE_CAN_PAY_CODE_HASH.set(
        script_info
            .get("anyone_can_pay")
            .cloned()
            .expect("get anyone can pay script info")
            .script
            .code_hash()
            .unpack(),
    );
    let _ = SUDT_CODE_HASH.set(
        script_info
            .get("sudt")
            .cloned()
            .expect("get sudt script info")
            .script
            .code_hash()
            .unpack(),
    );

    let _ = SECP_DATA_CELL_DEP_TX_HASH.set(
        script_info
            .get("dao")
            .cloned()
            .expect("get dao script info")
            .cell_dep
            .out_point()
            .tx_hash()
            .unpack(),
    );
    let _ = SECP_DATA_CELL_DEP_TX_IDX.set(3);

    let xudt_cell_dep = script_info
        .get("xudt_rce")
        .cloned()
        .expect("get xudt script info")
        .cell_dep
        .out_point();
    let xudt_cell_dep_index: u32 = xudt_cell_dep.index().unpack();
    let _ = XUDT_CELL_DEP_TX_HASH.set(xudt_cell_dep.tx_hash().unpack());
    let _ = XUDT_CELL_DEP_TX_IDX.set(xudt_cell_dep_index as usize);

    let omni_cell_dep = script_info
        .get("omni_lock")
        .cloned()
        .expect("get omni script info")
        .cell_dep
        .out_point();
    let omni_cell_dep_index: u32 = omni_cell_dep.index().unpack();
    let _ = OMNI_OPENTX_CELL_DEP_TX_HASH.set(omni_cell_dep.tx_hash().unpack());
    let _ = OMNI_OPENTX_CELL_DEP_TX_IDX.set(omni_cell_dep_index as usize);
}
