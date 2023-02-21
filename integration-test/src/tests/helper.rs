use crate::const_definition::{CKB_URI, MERCURY_URI, UDT_1_HOLDER_SECP_ADDRESS};
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::ckb::aggregate_transactions_into_blocks;
use crate::utils::instruction::ckb::dump_data;
use crate::utils::instruction::mercury::prepare_udt;

use utils::client::ckb_cli_client::{ckb_cli_get_capacity, ckb_cli_transfer_ckb};
use utils::const_definition::XUDT_DEVNET_TYPE_HASH;
use utils::lock::omni::{MultiSigArgs, TxInfo};
use utils::wallet::{GenOpenTxArgs, Wallet};

use anyhow::Result;
use ckb_sdk::{unlock::IdentityFlag, HumanCapacity};
use ckb_types::{
    bytes::Bytes,
    core::{capacity_bytes, Capacity, ScriptHashType},
    packed::{Byte32, CellOutput, OutPoint, Script},
    prelude::*,
};
use core_rpc_types::{GetBalancePayload, JsonItem};

use std::collections::HashSet;
use std::str::FromStr;

use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;

pub fn alice_build_signed_otx() -> Result<TxInfo> {
    // 1. init wallet instance
    let (address, pk) = generate_rand_secp_address_pk_pair();
    let alice_wallet = Wallet::init_account(address, pk, CKB_URI);
    let alice_omni_address = alice_wallet.get_omni_otx_address();

    // 2. transfer capacity to alice omni address
    let _tx_hash = ckb_cli_transfer_ckb(alice_omni_address, 151).unwrap();
    aggregate_transactions_into_blocks()?;

    let capacity = ckb_cli_get_capacity(alice_omni_address).unwrap();
    assert_eq!(151f64, capacity);

    // 3. generate open transaction, pay 51 CKB
    let gen_open_tx_args = GenOpenTxArgs {
        omni_identity_flag: IdentityFlag::PubkeyHash,
        multis_args: MultiSigArgs {
            require_first_n: 1,
            threshold: 1,
            sighash_address: vec![],
        },
        receiver: alice_omni_address.to_owned(),
        capacity_with_open: Some((
            HumanCapacity::from_str("100.0").unwrap(),
            HumanCapacity::from_str("51.0").unwrap(),
        )),
        udt_amount_with_open: None,
        fee_rate: 0,
    };
    let open_tx = alice_wallet.gen_open_tx(&gen_open_tx_args).unwrap();
    let file = "./free-space/usercase_alice_otx_unsigned.json";
    dump_data(&open_tx, file).unwrap();

    // 4. sign the otx
    let open_tx = alice_wallet.sign_open_tx(open_tx).unwrap();
    dump_data(&open_tx, "./free-space/usercase_alice_otx_signed.json").unwrap();

    Ok(open_tx)
}

pub fn bob_build_signed_otx() -> Result<TxInfo> {
    // 1. init bob's wallet
    let (address, pk) = generate_rand_secp_address_pk_pair();
    let bob_wallet = Wallet::init_account(address, pk, CKB_URI);
    let bob_otx_address = bob_wallet.get_omni_otx_address();
    let bob_omni_otx_script: Script = bob_otx_address.into();

    // 2. transfer udt to bob omni address
    let tx_hash = prepare_udt(51u128, bob_otx_address).unwrap();
    let out_point = OutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, 0u32);
    let balance_payload = GetBalancePayload {
        item: JsonItem::OutPoint(out_point.clone().into()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 2);
    assert_eq!(balance.balances[0].occupied, 144_0000_0000u128.into());
    assert_eq!(balance.balances[1].free, 51u128.into());

    // 3. bob generate open transaction, pay 51 UDT
    let udt_issuer_script: Script = UDT_1_HOLDER_SECP_ADDRESS.get().unwrap().into();
    let xudt_type_script = Script::new_builder()
        .code_hash(Byte32::from_slice(XUDT_DEVNET_TYPE_HASH.as_bytes()).unwrap())
        .hash_type(ScriptHashType::Type.into())
        .args(udt_issuer_script.calc_script_hash().raw_data().pack())
        .build();
    let xudt_output = CellOutput::new_builder()
        .capacity(capacity_bytes!(144).pack())
        .lock(bob_omni_otx_script)
        .type_(Some(xudt_type_script).pack())
        .build();
    let xudt_data = Bytes::from(0u128.to_le_bytes().to_vec());
    let open_tx = bob_wallet
        .gen_open_tx_pay_udt(vec![out_point], vec![xudt_output], vec![xudt_data.pack()])
        .unwrap();
    let file = "./free-space/usercase_bob_otx_unsigned.json";
    dump_data(&open_tx, file).unwrap();

    // 4. bob sign the otx
    let open_tx = bob_wallet.sign_open_tx(open_tx).unwrap();
    dump_data(&open_tx, "./free-space/usercase_bob_otx_signed.json").unwrap();

    Ok(open_tx)
}
