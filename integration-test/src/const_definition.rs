use ckb_sdk::Address;
use ckb_types::H256;
use once_cell::sync::OnceCell;

pub const CKB_URI: &str = "http://127.0.0.1:8114";
pub const MERCURY_URI: &str = "http://127.0.0.1:8116";
pub const OTX_POOL_URI: &str = "http://127.0.0.1:8118";

pub static OTX_POOL_AGENT_ADDRESS: OnceCell<Address> = OnceCell::new();
pub static OTX_POOL_AGENT_PK: OnceCell<H256> = OnceCell::new();

pub static UDT_1_HASH: OnceCell<H256> = OnceCell::new();
pub static UDT_1_HOLDER_SECP_ADDRESS: OnceCell<Address> = OnceCell::new();
pub static UDT_1_HOLDER_ACP_ADDRESS: OnceCell<Address> = OnceCell::new();
pub static UDT_1_HOLDER_PK: OnceCell<H256> = OnceCell::new();
