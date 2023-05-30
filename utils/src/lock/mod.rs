pub mod acp;
pub mod omni;

use anyhow::Result;
use ckb_hash::blake2b_256;
use ckb_sdk_otx::Address;
use ckb_types::{core::ScriptHashType, packed, prelude::*, H160, H256};

use rand::Rng;

use std::str::FromStr;

// for testing only
pub fn generate_rand_private_key() -> H256 {
    H256(rand::thread_rng().gen::<[u8; 32]>())
}

pub fn generate_secp_args_from_pk(pk: &H256) -> Result<H160> {
    let secret_key = secp256k1::SecretKey::from_str(&pk.to_string())
        .expect("impossible: fail to build secret key");
    let secp256k1: secp256k1::Secp256k1<secp256k1::All> = secp256k1::Secp256k1::new();
    let pubkey = secp256k1::PublicKey::from_secret_key(&secp256k1, &secret_key);

    // pubkey hash
    let pubkey = &pubkey.serialize()[..];
    let pubkey_hash = blake2b_256(pubkey);

    // generate args by pubkey hash
    H160::from_slice(&pubkey_hash[0..20]).map_err(Into::into)
}

pub fn get_udt_hash_by_owner(owner_address: &Address, xudt_code_hash: H256) -> Result<H256> {
    let owner_script: packed::Script = owner_address.payload().into();
    let sudt_type_script = packed::ScriptBuilder::default()
        .code_hash(xudt_code_hash.pack())
        .args(owner_script.calc_script_hash().raw_data().pack())
        .hash_type(ScriptHashType::Type.into())
        .build();
    Ok(sudt_type_script.calc_script_hash().unpack())
}
