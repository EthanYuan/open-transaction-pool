use utils::config::ScriptInfo;

use anyhow::Result;
use ckb_sdk_otx::Address;

pub fn build_otx_address_from_secp_address(
    _secp_address: &Address,
    _otx_script_info: &ScriptInfo,
) -> Result<Address> {
    todo!()
}
