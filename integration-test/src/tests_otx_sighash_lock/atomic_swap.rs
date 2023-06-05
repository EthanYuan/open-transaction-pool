#![allow(clippy::too_many_arguments)]

use crate::const_definition::{
    CKB_URI, MERCURY_URI, OTX_POOL_URI, SCRIPT_CONFIG, UDT_1_HASH, UDT_1_HOLDER_SECP_ADDRESS,
    UDT_2_HASH, UDT_2_HOLDER_SECP_ADDRESS,
};
use crate::help::start_otx_pool;
use crate::utils::client::ckb_cli_client::ckb_cli_transfer_ckb;
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::ckb::aggregate_transactions_into_blocks;
use crate::utils::instruction::ckb::dump_data;
use crate::utils::instruction::mercury::{prepare_ckb_capacity, prepare_udt_1, prepare_udt_2};
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use config::{CkbConfig, ScriptInfo};
use otx_format::jsonrpc_types::OpenTransaction;
use otx_format::types::OpenTxStatus;
use otx_sdk::address::build_otx_address_from_secp_address;
use otx_sdk::build_tx::OtxBuilder;
use otx_sdk::client::OtxPoolRpcClient;
use otx_sdk::signer::{SighashMode, Signer};

use core_rpc_types::{AssetInfo, GetBalancePayload, JsonItem};

use anyhow::{Ok, Result};
use ckb_sdk::Address;
use ckb_types::prelude::Entity;
use ckb_types::{
    bytes::Bytes,
    core::{capacity_bytes, Capacity, ScriptHashType},
    packed::{Byte32, CellOutput, OutPoint, Script},
    prelude::*,
    H256,
};

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

inventory::submit!(IntegrationTest {
    name: "test_otx_swap_udt_to_udt",
    test_fn: test_otx_swap_udt_to_udt
});
fn test_otx_swap_udt_to_udt() {
    // run otx pool
    let (address, pk) = generate_rand_secp_address_pk_pair();
    prepare_ckb_capacity(&address, 200_0000_0000u64).unwrap();
    start_otx_pool(address, pk);

    // get otx lock script info
    let script_config = SCRIPT_CONFIG.get().unwrap().clone();
    let otx_lock_script_info = script_config.get_script_info("otx-sighash-lock").unwrap();
    let udt_type_script_info = script_config.get_script_info("sudt").unwrap();

    // alice build otxs
    // pay 10 UDT-1, get 10 UDT-2, pay fee 1 CKB
    let (alice_address, alice_pk) = generate_rand_secp_address_pk_pair();
    let alice_otx_address =
        build_otx_address_from_secp_address(&alice_address, &otx_lock_script_info).unwrap();
    let alice_otx = build_signed_otx(
        "alice",
        &alice_otx_address,
        (&alice_address, &alice_pk),
        12,
        5,
        201_0000_0000,
        2,
        15,
        200_0000_0000,
        vec![
            otx_lock_script_info.to_owned(),
            udt_type_script_info.to_owned(),
        ],
    )
    .unwrap();

    // bob build otxs
    // pay 10 UDT-2, get 10 UDT-1, pay fee 1 CKB
    let (bob_address, bob_pk) = generate_rand_secp_address_pk_pair();
    let bob_otx_address =
        build_otx_address_from_secp_address(&bob_address, &otx_lock_script_info).unwrap();
    let bob_otx = build_signed_otx(
        "bob",
        &bob_otx_address,
        (&bob_address, &bob_pk),
        10,
        100,
        201_0000_0000,
        20,
        90,
        200_0000_0000,
        vec![otx_lock_script_info, udt_type_script_info],
    )
    .unwrap();

    // submit alice otxs
    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let alice_otx_id = service_client.submit_otx(alice_otx).unwrap();

    // query alice otxs
    let alice_otx_with_status = service_client
        .query_otx_status_by_id(alice_otx_id.clone())
        .unwrap()
        .unwrap();
    assert_eq!(alice_otx_with_status, OpenTxStatus::Pending);

    // submit bob otxs
    let bob_otx_id = service_client.submit_otx(bob_otx).unwrap();

    sleep(Duration::from_secs(5));
    aggregate_transactions_into_blocks().unwrap();

    // query otxs
    let alice_otx_with_status = service_client
        .query_otx_status_by_id(alice_otx_id)
        .unwrap()
        .unwrap();
    let bob_otx_status = service_client
        .query_otx_status_by_id(bob_otx_id)
        .unwrap()
        .unwrap();
    assert!(matches!(alice_otx_with_status, OpenTxStatus::Committed(_)));
    assert!(matches!(bob_otx_status, OpenTxStatus::Committed(_)));
    assert_eq!(alice_otx_with_status, bob_otx_status);

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // check alice assets
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(UDT_1_HASH.get().unwrap().clone()));
    let payload = GetBalancePayload {
        item: JsonItem::Address(alice_otx_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(2u128, response.balances[0].free.into());

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(UDT_2_HASH.get().unwrap().clone()));
    let payload = GetBalancePayload {
        item: JsonItem::Address(alice_otx_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(15u128, response.balances[0].free.into());

    // check bob assets
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(UDT_1_HASH.get().unwrap().clone()));
    let payload = GetBalancePayload {
        item: JsonItem::Address(bob_otx_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(20u128, response.balances[0].free.into());

    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(UDT_2_HASH.get().unwrap().clone()));
    let payload = GetBalancePayload {
        item: JsonItem::Address(bob_otx_address.to_string()),
        asset_infos,
        extra: None,
        tip_block_number: None,
    };
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(90u128, response.balances[0].free.into());
}

fn build_signed_otx(
    payer: &str,
    otx_address: &Address,
    (_secp_addr, pk): (&Address, &H256),
    prepare_udt_1_amount: u128,
    prepare_udt_2_amount: u128,
    prepare_capacity: usize,
    remain_udt_1: u128,
    remain_udt_2: u128,
    remain_capacity: usize,
    script_infos: Vec<ScriptInfo>,
) -> Result<OpenTransaction> {
    // get udt script info
    let script_config = SCRIPT_CONFIG.get().unwrap().clone();
    let udt_script_code_hash = script_config.get_sudt_code_hash();

    // get ckb config
    let ckb_config = CkbConfig::new("ckb_dev", CKB_URI);

    // 1. init address
    let otx_script: Script = (otx_address).into();

    // 2. transfer udt-1 to otx-sighash-lock address
    let tx_hash = prepare_udt_1(prepare_udt_1_amount, otx_address).unwrap();
    let out_point_1 = OutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, 0u32);
    let balance_payload = GetBalancePayload {
        item: JsonItem::OutPoint(out_point_1.clone().into()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 2);
    assert_eq!(balance.balances[0].occupied, 142_0000_0000u128.into());
    assert_eq!(balance.balances[0].free, 0_0000_0000u128.into());
    assert_eq!(balance.balances[1].free, prepare_udt_1_amount.into());

    // 3. transfer udt-2 to otx-sighash-lock address
    let tx_hash = prepare_udt_2(prepare_udt_2_amount, otx_address).unwrap();
    let out_point_2 = OutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, 0u32);
    let balance_payload = GetBalancePayload {
        item: JsonItem::OutPoint(out_point_2.clone().into()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 2);
    assert_eq!(balance.balances[0].occupied, 142_0000_0000u128.into());
    assert_eq!(balance.balances[0].free, 0_0000_0000u128.into());
    assert_eq!(balance.balances[1].free, prepare_udt_2_amount.into());

    // 4. transfer capacity to otx-sighash-lock address
    let tx_hash = ckb_cli_transfer_ckb(otx_address, prepare_capacity / 1_0000_0000).unwrap();
    aggregate_transactions_into_blocks().unwrap();
    let out_point_3 = OutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, 0u32);
    let balance_payload = GetBalancePayload {
        item: JsonItem::OutPoint(out_point_3.clone().into()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(
        balance.balances[0].free,
        (prepare_capacity as u128 - 61_0000_0000u128).into()
    );
    assert_eq!(balance.balances[0].occupied, 61_0000_0000u128.into());

    // 5. generate open transaction, pay UDT-1, get UDT-2, pay fee
    let udt_1_issuer_script: Script = UDT_1_HOLDER_SECP_ADDRESS.get().unwrap().into();
    let udt_1_type_script = Script::new_builder()
        .code_hash(Byte32::from_slice(udt_script_code_hash.as_bytes()).unwrap())
        .hash_type(ScriptHashType::Type.into())
        .args(udt_1_issuer_script.calc_script_hash().raw_data().pack())
        .build();
    let udt_1_output = CellOutput::new_builder()
        .capacity(capacity_bytes!(142).pack())
        .lock(otx_script.clone())
        .type_(Some(udt_1_type_script).pack())
        .build();
    let udt_1_data = Bytes::from(remain_udt_1.to_le_bytes().to_vec());

    let udt_2_issuer_script: Script = UDT_2_HOLDER_SECP_ADDRESS.get().unwrap().into();
    let udt_2_type_script = Script::new_builder()
        .code_hash(Byte32::from_slice(udt_script_code_hash.as_bytes()).unwrap())
        .hash_type(ScriptHashType::Type.into())
        .args(udt_2_issuer_script.calc_script_hash().raw_data().pack())
        .build();
    let udt_2_output = CellOutput::new_builder()
        .capacity(capacity_bytes!(142).pack())
        .lock(otx_script.clone())
        .type_(Some(udt_2_type_script).pack())
        .build();
    let udt_2_data = Bytes::from(remain_udt_2.to_le_bytes().to_vec());

    let capacity_output = CellOutput::new_builder()
        .capacity((remain_capacity as u64).pack())
        .lock(otx_script)
        .build();
    let data = Bytes::default();

    let otx_builder = OtxBuilder::new(script_config.to_owned(), ckb_config.to_owned());
    let open_tx = otx_builder
        .build_otx(
            vec![out_point_1, out_point_2, out_point_3],
            vec![udt_1_output, udt_2_output, capacity_output],
            vec![udt_1_data.pack(), udt_2_data.pack(), data.pack()],
            script_infos,
        )
        .unwrap();
    let file = format!("./free-space/swap_{}_otx_unsigned.json", payer);
    dump_data(&open_tx, &file).unwrap();

    let signer = Signer::new(pk.to_owned(), script_config, ckb_config);
    let open_tx = signer
        .partial_sign(open_tx, SighashMode::SingleAnyoneCanPay, vec![0, 1, 2])
        .unwrap();
    dump_data(
        &open_tx,
        &format!("./free-space/swap_{}_otx_signed.json", payer),
    )
    .unwrap();

    Ok(open_tx)
}
