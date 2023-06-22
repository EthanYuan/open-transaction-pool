use otx_pool_config::ScriptInfo;

use anyhow::Result;
use ckb_sdk::{Address, AddressPayload};
use ckb_types::core::ScriptHashType;

pub fn build_otx_address_from_secp_address(
    secp_address: &Address,
    otx_script_info: &ScriptInfo,
) -> Result<Address> {
    let address_payload = AddressPayload::new_full(
        ScriptHashType::try_from(otx_script_info.script.hash_type())?,
        otx_script_info.script.code_hash(),
        secp_address.payload().args(),
    );
    let address = Address::new(secp_address.network(), address_payload, true);
    Ok(address)
}
