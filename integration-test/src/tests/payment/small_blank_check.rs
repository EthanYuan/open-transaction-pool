use crate::const_definition::{CKB_URI, MERCURY_URI, OTX_POOL_URI, SCRIPT_CONFIG};
use crate::help::start_otx_pool;
use crate::utils::client::ckb_cli_client::ckb_cli_transfer_ckb;
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::ckb::aggregate_transactions_into_blocks;
use crate::utils::instruction::ckb::dump_data;
use crate::utils::instruction::mercury::prepare_ckb_capacity;
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_format::types::OpenTxStatus;
use otx_pool_client::OtxPoolRpcClient;
use otx_pool_config::{CkbConfig, ScriptInfo};
use otx_pool_plugin_dust_collector::DEFAULT_FEE;
use otx_sdk::address::build_otx_address_from_secp_address;
use otx_sdk::build_tx::OtxBuilder;
use otx_sdk::signer::{SighashMode, Signer};

use core_rpc_types::{GetBalancePayload, JsonItem};

use anyhow::{Ok, Result};
use ckb_sdk::Address;
use ckb_types::prelude::Entity;
use ckb_types::{
    bytes::Bytes,
    packed::{Byte32, CellOutput, OutPoint, Script},
    prelude::*,
    H256,
};

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

inventory::submit!(IntegrationTest {
    name: "test_payment_small_blank_check",
    test_fn: test_payment_small_blank_check
});
fn test_payment_small_blank_check() {
    // run otx pool
    let (payee_address, pk) = generate_rand_secp_address_pk_pair();
    prepare_ckb_capacity(&payee_address, 200_0000_0000u64).unwrap();
    start_otx_pool(payee_address.clone(), pk);

    // get otx lock script info
    let script_config = SCRIPT_CONFIG.get().unwrap().clone();
    let otx_lock_script_info = script_config.get_script_info("otx-sighash-lock").unwrap();

    // check dust collector assets
    let payload = GetBalancePayload {
        item: JsonItem::Address(payee_address.to_string()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let response = mercury_client.get_balance(payload.clone()).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(200_0000_0000u128, response.balances[0].free.into());

    // alice build otxs
    // pay 51 CKB
    let (alice_address, alice_pk) = generate_rand_secp_address_pk_pair();
    let alice_otx_address =
        build_otx_address_from_secp_address(&alice_address, &otx_lock_script_info).unwrap();
    let alice_otx = build_signed_otx(
        "alice",
        &alice_otx_address,
        (&alice_address, &alice_pk),
        151_0000_0000,
        100_0000_0000,
        vec![otx_lock_script_info.to_owned()],
    )
    .unwrap();

    // bob build otxs
    // pay 2 CKB
    let (bob_address, bob_pk) = generate_rand_secp_address_pk_pair();
    let bob_otx_address =
        build_otx_address_from_secp_address(&bob_address, &otx_lock_script_info).unwrap();
    let bob_otx = build_signed_otx(
        "bob",
        &bob_otx_address,
        (&bob_address, &bob_pk),
        202_0000_0000,
        200_0000_0000,
        vec![otx_lock_script_info],
    )
    .unwrap();

    // submit otxs
    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let alice_otx_id = service_client.submit_otx(alice_otx).unwrap();
    let bob_otx_id = service_client.submit_otx(bob_otx).unwrap();

    // query otxs immediately
    let alice_otx_status = service_client
        .query_otx_status_by_id(alice_otx_id.clone())
        .unwrap()
        .unwrap();
    assert_eq!(alice_otx_status, OpenTxStatus::Pending);
    let bob_otx_status = service_client
        .query_otx_status_by_id(bob_otx_id.clone())
        .unwrap()
        .unwrap();
    assert_eq!(bob_otx_status, OpenTxStatus::Pending);

    sleep(Duration::from_secs(12));
    aggregate_transactions_into_blocks().unwrap();

    // query otxs after a few secs
    let alice_otx_status = service_client
        .query_otx_status_by_id(alice_otx_id)
        .unwrap()
        .unwrap();
    let bob_otx_status = service_client
        .query_otx_status_by_id(bob_otx_id)
        .unwrap()
        .unwrap();
    assert!(matches!(alice_otx_status, OpenTxStatus::Committed(_)));
    assert!(matches!(bob_otx_status, OpenTxStatus::Committed(_)));
    assert_eq!(alice_otx_status, bob_otx_status);
    if let OpenTxStatus::Committed(tx_hash) = alice_otx_status {
        let merged_otx_status = service_client
            .query_otx_status_by_id(tx_hash)
            .unwrap()
            .unwrap();
        assert!(matches!(merged_otx_status, OpenTxStatus::Committed(_)));
    } else {
        panic!()
    }

    // check payee assets
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 1);
    assert_eq!(
        200_0000_0000u128 + 53_0000_0000u128 - DEFAULT_FEE as u128,
        response.balances[0].free.into()
    );
}

pub(crate) fn build_signed_otx(
    payer: &str,
    otx_address: &Address,
    (_secp_addr, pk): (&Address, &H256),
    prepare_capacity: u64,
    remain_capacity: u64,
    script_infos: Vec<ScriptInfo>,
) -> Result<OpenTransaction> {
    // get udt script info
    let script_config = SCRIPT_CONFIG.get().unwrap().clone();

    // get ckb config
    let ckb_config = CkbConfig::new("ckb_dev", CKB_URI);

    // 1. init address
    let otx_script: Script = (otx_address).into();

    // 2. transfer capacity to otx-sighash-lock address
    let tx_hash = ckb_cli_transfer_ckb(otx_address, prepare_capacity / 1_0000_0000).unwrap();
    aggregate_transactions_into_blocks().unwrap();
    let out_point = OutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, 0u32);
    let balance_payload = GetBalancePayload {
        item: JsonItem::OutPoint(out_point.clone().into()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let balance = mercury_client.get_balance(balance_payload).unwrap();
    assert_eq!(balance.balances.len(), 1);
    assert_eq!(
        balance.balances[0].free,
        (prepare_capacity as u128 - 61_0000_0000u128).into()
    );
    assert_eq!(balance.balances[0].occupied, 61_0000_0000u128.into());

    // 5. generate open transaction, pay ckb
    let capacity_output = CellOutput::new_builder()
        .capacity(remain_capacity.pack())
        .lock(otx_script)
        .build();
    let data = Bytes::default();

    let otx_builder = OtxBuilder::new(script_config.to_owned(), ckb_config.to_owned());
    let open_tx = otx_builder
        .build_otx(
            vec![out_point],
            vec![capacity_output],
            vec![data.pack()],
            script_infos,
            prepare_capacity - remain_capacity,
        )
        .unwrap();
    let file = format!("./free-space/payment_{}_otx_unsigned.json", payer);
    dump_data(&open_tx, &file).unwrap();

    let signer = Signer::new(pk.to_owned(), script_config, ckb_config);
    let open_tx = signer
        .partial_sign(open_tx, SighashMode::SingleAnyoneCanPay, vec![0])
        .unwrap();
    dump_data(
        &open_tx,
        &format!("./free-space/payment_{}_otx_signed.json", payer),
    )
    .unwrap();

    Ok(open_tx)
}
