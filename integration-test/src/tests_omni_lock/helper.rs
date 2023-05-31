use crate::const_definition::{CKB_URI, MERCURY_URI, SCRIPT_CONFIG, UDT_1_HOLDER_SECP_ADDRESS};
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::ckb::aggregate_transactions_into_blocks;
use crate::utils::instruction::ckb::dump_data;
use crate::utils::instruction::mercury::prepare_udt_1;
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;

use config::CkbConfig;
use utils::client::ckb_cli_client::{ckb_cli_get_capacity, ckb_cli_transfer_ckb};
use utils::lock::omni::{MultiSigArgs, TxInfo};
use utils::wallet::{GenOpenTxArgs, Wallet};

use anyhow::Result;
use ckb_sdk_otx::{unlock::IdentityFlag, HumanCapacity};
use ckb_types::{
    bytes::Bytes,
    core::{capacity_bytes, Capacity, ScriptHashType},
    packed::{Byte32, CellOutput, OutPoint, Script},
    prelude::*,
};
use core_rpc_types::{GetBalancePayload, JsonItem};

use std::collections::HashSet;
use std::str::FromStr;

pub fn build_pay_ckb_signed_otx(
    payer: &str,
    prepare_capacity: usize,
    remain_capacity: usize,
    open_capacity: usize,
) -> Result<TxInfo> {
    // 1. init wallet instance
    let (address, pk) = generate_rand_secp_address_pk_pair();
    let wallet = Wallet::new(
        address,
        pk,
        CkbConfig::new("ckb_dev", CKB_URI),
        SCRIPT_CONFIG.get().unwrap().clone(),
    )
    .unwrap();
    let omni_address = wallet.get_omni_otx_address()?;

    // 2. transfer capacity to omni address
    let capacity = prepare_capacity;
    log::info!("{} prepare wallet: {:?} CKB", payer, capacity);
    let _tx_hash = ckb_cli_transfer_ckb(&omni_address, capacity).unwrap();
    aggregate_transactions_into_blocks()?;

    let capacity = ckb_cli_get_capacity(&omni_address).unwrap();
    assert_eq!(prepare_capacity as f64, capacity);

    // 3. generate open transaction
    let gen_open_tx_args = GenOpenTxArgs {
        omni_identity_flag: IdentityFlag::PubkeyHash,
        multis_args: MultiSigArgs {
            require_first_n: 1,
            threshold: 1,
            sighash_address: vec![],
        },
        receiver: omni_address,
        capacity_with_open: Some((
            HumanCapacity::from_str(&remain_capacity.to_string()).unwrap(),
            HumanCapacity::from_str(&open_capacity.to_string()).unwrap(),
        )),
        udt_amount_with_open: None,
        fee_rate: 0,
    };
    let open_tx = wallet.gen_open_tx(&gen_open_tx_args).unwrap();
    let file = format!("./free-space/dust_collector_{}_otx_unsigned.json", payer);
    dump_data(&open_tx, &file).unwrap();

    // 4. sign the otx
    let open_tx = wallet.sign_open_tx(open_tx).unwrap();
    dump_data(
        &open_tx,
        &format!("./free-space/dust_collector_{}_otx_signed.json", payer),
    )
    .unwrap();

    Ok(open_tx)
}

pub fn _bob_build_signed_otx() -> Result<TxInfo> {
    // 1. init bob's wallet
    let (address, pk) = generate_rand_secp_address_pk_pair();
    let bob_wallet = Wallet::new(
        address,
        pk,
        CkbConfig::new("ckb_dev", CKB_URI),
        SCRIPT_CONFIG.get().unwrap().clone(),
    )
    .unwrap();
    let bob_otx_address = bob_wallet.get_omni_otx_address()?;
    let bob_omni_otx_script: Script = (&bob_otx_address).into();

    // 2. transfer udt to bob omni address
    let udt_amount = 51u128;
    log::info!("prepare bob wallet: {:?} UDT", udt_amount);
    let tx_hash = prepare_udt_1(udt_amount, &bob_otx_address).unwrap();
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
        .code_hash(
            Byte32::from_slice(
                SCRIPT_CONFIG
                    .get()
                    .unwrap()
                    .get_xudt_rce_code_hash()
                    .as_bytes(),
            )
            .unwrap(),
        )
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
