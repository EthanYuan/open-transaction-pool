pub mod devnet;
use devnet::{
    ANYONE_CAN_PAY_DEVNET_TYPE_HASH, OMNI_LOCK_DEVNET_TYPE_HASH, OMNI_OPENTX_TX_HASH,
    OMNI_OPENTX_TX_IDX, SECP_DATA_TX_HASH, SECP_DATA_TX_IDX, SIGHASH_TYPE_HASH,
    XUDT_DEVNET_TYPE_HASH, XUDT_TX_HASH, XUDT_TX_IDX,
};

use ckb_types::H256;
use once_cell::sync::OnceCell;

pub static CKB_URI: OnceCell<String> = OnceCell::new();

// script code hash
pub static SECP256K1_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static XUDT_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static OMNI_CODE_HASH: OnceCell<H256> = OnceCell::new();
pub static ANYONE_CAN_PAY_CODE_HASH: OnceCell<H256> = OnceCell::new();

// cell deps
pub static XUDT_CELL_DEP_TX_HASH: OnceCell<H256> = OnceCell::new();
pub static XUDT_CELL_DEP_TX_IDX: OnceCell<usize> = OnceCell::new();
pub static OMNI_OPENTX_CELL_DEP_TX_HASH: OnceCell<H256> = OnceCell::new();
pub static OMNI_OPENTX_CELL_DEP_TX_IDX: OnceCell<usize> = OnceCell::new();
pub static SECP_DATA_CELL_DEP_TX_HASH: OnceCell<H256> = OnceCell::new();
pub static SECP_DATA_CELL_DEP_TX_IDX: OnceCell<usize> = OnceCell::new();

pub fn load_code_hash() {
    let _ = SECP256K1_CODE_HASH.set(SIGHASH_TYPE_HASH);
    let _ = XUDT_CODE_HASH.set(XUDT_DEVNET_TYPE_HASH);
    let _ = OMNI_CODE_HASH.set(OMNI_LOCK_DEVNET_TYPE_HASH);
    let _ = ANYONE_CAN_PAY_CODE_HASH.set(ANYONE_CAN_PAY_DEVNET_TYPE_HASH);

    let _ = XUDT_CELL_DEP_TX_HASH.set(XUDT_TX_HASH);
    let _ = XUDT_CELL_DEP_TX_IDX.set(XUDT_TX_IDX);
    let _ = SECP_DATA_CELL_DEP_TX_HASH.set(SECP_DATA_TX_HASH);
    let _ = SECP_DATA_CELL_DEP_TX_IDX.set(SECP_DATA_TX_IDX);
    let _ = OMNI_OPENTX_CELL_DEP_TX_HASH.set(OMNI_OPENTX_TX_HASH);
    let _ = OMNI_OPENTX_CELL_DEP_TX_IDX.set(OMNI_OPENTX_TX_IDX);
}
