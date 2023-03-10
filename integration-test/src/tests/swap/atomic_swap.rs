#![allow(clippy::too_many_arguments)]

use crate::const_definition::{
    CKB_URI, MERCURY_URI, OTX_POOL_URI, UDT_1_HASH, UDT_1_HOLDER_SECP_ADDRESS, UDT_2_HASH,
    UDT_2_HOLDER_SECP_ADDRESS,
};
use crate::help::start_otx_pool;
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::ckb::aggregate_transactions_into_blocks;
use crate::utils::instruction::ckb::dump_data;
use crate::utils::instruction::mercury::{prepare_ckb_capacity, prepare_udt_1, prepare_udt_2};
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use otx_format::jsonrpc_types::tx_view::tx_view_to_otx;
use otx_format::types::{packed, OpenTxStatus};
use utils::client::ckb_cli_client::ckb_cli_transfer_ckb;
use utils::client::service_client::OtxPoolRpcClient;
use utils::const_definition::XUDT_CODE_HASH;
use utils::lock::omni::build_otx_omnilock_addr_from_secp;
use utils::wallet::Wallet;

use anyhow::Result;
use ckb_jsonrpc_types::JsonBytes;
use ckb_sdk::Address;
use ckb_types::prelude::Entity;
use ckb_types::{
    bytes::Bytes,
    core::{capacity_bytes, Capacity, ScriptHashType},
    packed::{Byte32, CellOutput, OutPoint, Script},
    prelude::*,
    H256,
};
use core_rpc_types::{AssetInfo, GetBalancePayload, JsonItem};

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

inventory::submit!(IntegrationTest {
    name: "test_swap_udt_to_udt",
    test_fn: test_swap_udt_to_udt
});
fn test_swap_udt_to_udt() {
    // run otx pool
    let (address, pk) = generate_rand_secp_address_pk_pair();
    prepare_ckb_capacity(&address, 200_0000_0000u64).unwrap();
    start_otx_pool(address, pk);

    // alice build otxs
    // pay 10 UDT-1, get 10 UDT-2, pay fee 1 CKB
    let (alice_address, alice_pk) = generate_rand_secp_address_pk_pair();
    let alice_otx = build_signed_otx(
        "alice",
        alice_address.clone(),
        alice_pk,
        12,
        5,
        201_0000_0000,
        2,
        15,
        200_0000_0000,
    )
    .unwrap();

    // bob build otxs
    // pay 10 UDT-2, get 10 UDT-1, pay fee 1 CKB
    let (bob_address, bob_pk) = generate_rand_secp_address_pk_pair();
    let bob_otx = build_signed_otx(
        "bob",
        bob_address.clone(),
        bob_pk,
        10,
        100,
        201_0000_0000,
        20,
        90,
        200_0000_0000,
    )
    .unwrap();

    // submit alice otxs
    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let alice_otx_id = service_client
        .submit_otx(JsonBytes::from_bytes(alice_otx.as_bytes()))
        .unwrap();

    // query alice otxs
    let alice_otx_with_status = service_client
        .query_otx_by_id(alice_otx_id.clone())
        .unwrap()
        .unwrap();
    assert_eq!(alice_otx_with_status.status, OpenTxStatus::Pending);

    // submit bob otxs
    let bob_otx_id = service_client
        .submit_otx(JsonBytes::from_bytes(bob_otx.as_bytes()))
        .unwrap();

    sleep(Duration::from_secs(5));
    aggregate_transactions_into_blocks().unwrap();

    // query otxs
    let alice_otx_with_status = service_client
        .query_otx_by_id(alice_otx_id)
        .unwrap()
        .unwrap();
    let bob_otx_with_status = service_client.query_otx_by_id(bob_otx_id).unwrap().unwrap();
    assert!(matches!(
        alice_otx_with_status.status,
        OpenTxStatus::Committed(_)
    ));
    assert!(matches!(
        bob_otx_with_status.status,
        OpenTxStatus::Committed(_)
    ));
    assert_eq!(alice_otx_with_status.status, bob_otx_with_status.status);

    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());

    // check alice assets
    let mut asset_infos = HashSet::new();
    asset_infos.insert(AssetInfo::new_udt(UDT_1_HASH.get().unwrap().clone()));
    let alice_omni_otx_address =
        build_otx_omnilock_addr_from_secp(&alice_address, CKB_URI).unwrap();
    let payload = GetBalancePayload {
        item: JsonItem::Address(alice_omni_otx_address.to_string()),
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
        item: JsonItem::Address(alice_omni_otx_address.to_string()),
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
    let bob_omni_otx_address = build_otx_omnilock_addr_from_secp(&bob_address, CKB_URI).unwrap();
    let payload = GetBalancePayload {
        item: JsonItem::Address(bob_omni_otx_address.to_string()),
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
        item: JsonItem::Address(bob_omni_otx_address.to_string()),
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
    secp_address: Address,
    pk: H256,
    prepare_udt_1_amount: u128,
    prepare_udt_2_amount: u128,
    prepare_capacity: usize,
    remain_udt_1: u128,
    remain_udt_2: u128,
    remain_capacity: usize,
) -> Result<packed::OpenTransaction> {
    // 1. init wallet
    let wallet = Wallet::init_account(secp_address, pk, CKB_URI);
    let otx_address = wallet.get_omni_otx_address();
    let omni_otx_script: Script = otx_address.into();

    // 2. transfer udt-1 to omni address
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
    assert_eq!(balance.balances[0].occupied, 144_0000_0000u128.into());
    assert_eq!(balance.balances[1].free, prepare_udt_1_amount.into());

    // 3. transfer udt-2 to omni address
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
    assert_eq!(balance.balances[0].occupied, 144_0000_0000u128.into());
    assert_eq!(balance.balances[1].free, prepare_udt_2_amount.into());

    // 4. transfer capacity to omni address
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
        (prepare_capacity as u128 - 63_0000_0000u128).into()
    );
    assert_eq!(balance.balances[0].occupied, 63_0000_0000u128.into());

    // 5. generate open transaction, pay UDT-1, get UDT-2, pay fee
    let xudt_1_issuer_script: Script = UDT_1_HOLDER_SECP_ADDRESS.get().unwrap().into();
    let xudt_1_type_script = Script::new_builder()
        .code_hash(Byte32::from_slice(XUDT_CODE_HASH.get().unwrap().as_bytes()).unwrap())
        .hash_type(ScriptHashType::Type.into())
        .args(xudt_1_issuer_script.calc_script_hash().raw_data().pack())
        .build();
    let xudt_1_output = CellOutput::new_builder()
        .capacity(capacity_bytes!(144).pack())
        .lock(omni_otx_script.clone())
        .type_(Some(xudt_1_type_script).pack())
        .build();
    let xudt_1_data = Bytes::from(remain_udt_1.to_le_bytes().to_vec());

    let xudt_2_issuer_script: Script = UDT_2_HOLDER_SECP_ADDRESS.get().unwrap().into();
    let xudt_2_type_script = Script::new_builder()
        .code_hash(Byte32::from_slice(XUDT_CODE_HASH.get().unwrap().as_bytes()).unwrap())
        .hash_type(ScriptHashType::Type.into())
        .args(xudt_2_issuer_script.calc_script_hash().raw_data().pack())
        .build();
    let xudt_2_output = CellOutput::new_builder()
        .capacity(capacity_bytes!(144).pack())
        .lock(omni_otx_script.clone())
        .type_(Some(xudt_2_type_script).pack())
        .build();
    let xudt_2_data = Bytes::from(remain_udt_2.to_le_bytes().to_vec());

    let omni_output = CellOutput::new_builder()
        .capacity((remain_capacity as u64).pack())
        .lock(omni_otx_script)
        .build();
    let data = Bytes::default();

    let open_tx = wallet
        .gen_open_tx_pay_udt(
            vec![out_point_1, out_point_2, out_point_3],
            vec![xudt_1_output, xudt_2_output, omni_output],
            vec![xudt_1_data.pack(), xudt_2_data.pack(), data.pack()],
        )
        .unwrap();
    let file = format!("./free-space/swap_{}_otx_unsigned.json", payer);
    dump_data(&open_tx, &file).unwrap();

    let open_tx = wallet.sign_open_tx(open_tx).unwrap();
    dump_data(
        &open_tx,
        &format!("./free-space/swap_{}_otx_signed.json", payer),
    )
    .unwrap();

    let tx_view = open_tx.tx;
    let otx = tx_view_to_otx(
        tx_view,
        None,
        None,
        XUDT_CODE_HASH.get().unwrap().to_owned(),
        H256::default(),
        CKB_URI,
    )
    .unwrap();

    dump_data(
        &otx,
        &format!("./free-space/swap_{}_otx_format_unsigned.json", payer),
    )
    .unwrap();

    Ok(otx.into())
}
