use anyhow::Result;
use ckb_sdk::{Address, AddressPayload, NetworkType};
use ckb_types::{core::ScriptHashType, packed, prelude::*, H256};

pub fn build_acp_address(
    secp_address: &Address,
    anyone_cap_pay_code_hash: H256,
) -> Result<Address> {
    let secp_script: packed::Script = secp_address.payload().into();
    let acp_code_hash =
        packed::Byte32::from_slice(anyone_cap_pay_code_hash.as_bytes()).expect("impossible:");
    let payload = AddressPayload::new_full(
        ScriptHashType::Type,
        acp_code_hash,
        secp_script.args().raw_data(),
    );
    Ok(Address::new(NetworkType::Dev, payload, true))
}
