use ckb_sdk::Address;
use ckb_types::H256;
use once_cell::sync::OnceCell;

pub static OTX_POOL_AGENT_ADDRESS: OnceCell<Address> = OnceCell::new();
pub static OTX_POOL_AGENT_PK: OnceCell<H256> = OnceCell::new();
