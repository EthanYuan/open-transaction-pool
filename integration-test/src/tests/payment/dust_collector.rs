use crate::const_definition::{MERCURY_URI, OTX_POOL_AGENT_ADDRESS, OTX_POOL_URI};
use crate::tests::helper::{_bob_build_signed_otx, build_pay_ckb_signed_otx};
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::ckb::aggregate_transactions_into_blocks;
use crate::IntegrationTest;

use otx_format::jsonrpc_types::tx_view::tx_view_to_basic_otx;
use otx_format::types::{packed, OpenTxStatus};
use utils::client::service_client::OtxPoolRpcClient;

use anyhow::Result;
use ckb_jsonrpc_types::JsonBytes;
use ckb_types::prelude::Entity;
use core_rpc_types::{GetBalancePayload, JsonItem};

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

inventory::submit!(IntegrationTest {
    name: "test_payment_dust_collect_ckb",
    test_fn: test_payment_dust_collect_ckb
});
fn test_payment_dust_collect_ckb() {
    // check dust collector assets
    let dust_collector_address = OTX_POOL_AGENT_ADDRESS.get().unwrap();
    let payload = GetBalancePayload {
        item: JsonItem::Address(dust_collector_address.to_string()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let response = mercury_client.get_balance(payload.clone()).unwrap();
    assert_eq!(response.balances.len(), 2);
    assert_eq!(200_0000_0000u128, response.balances[0].free.into());
    assert_eq!(142_0000_0000u128, response.balances[0].occupied.into());
    assert_eq!(200u128, response.balances[1].free.into());

    // build otxs
    let alice_otx = build_pay_ckb_otx("alice", 151, 100, 51).unwrap();
    let bob_otx = build_pay_ckb_otx("bob", 202, 200, 2).unwrap();

    // submit otxs
    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let alice_otx_id = service_client
        .submit_otx(JsonBytes::from_bytes(alice_otx.as_bytes()))
        .unwrap();
    let bob_otx_id = service_client
        .submit_otx(JsonBytes::from_bytes(bob_otx.as_bytes()))
        .unwrap();

    // query otxs immediately
    let alice_otx_with_status = service_client
        .query_otx_by_id(alice_otx_id.clone())
        .unwrap()
        .unwrap();
    assert_eq!(alice_otx_with_status.status, OpenTxStatus::Pending);
    let bob_otx_with_status = service_client
        .query_otx_by_id(bob_otx_id.clone())
        .unwrap()
        .unwrap();
    assert_eq!(bob_otx_with_status.status, OpenTxStatus::Pending);

    sleep(Duration::from_secs(12));
    aggregate_transactions_into_blocks().unwrap();

    // query otxs after a few secs
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

    // check dust collector assets
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 2);
    assert_eq!(200_0000_0000u128, response.balances[0].free.into());
    assert_eq!(142_0000_0000u128, response.balances[0].occupied.into());
    assert_eq!(200u128, response.balances[1].free.into());
}

fn build_pay_ckb_otx(
    payer: &str,
    prepare_capacity: usize,
    remain_capacity: usize,
    open_capacity: usize,
) -> Result<packed::OpenTransaction> {
    let tx_info =
        build_pay_ckb_signed_otx(payer, prepare_capacity, remain_capacity, open_capacity).unwrap();
    let tx_view = tx_info.tx;
    let otx = tx_view_to_basic_otx(tx_view).unwrap();
    Ok(otx.into())
}

fn _bob_build_otx() -> Result<packed::OpenTransaction> {
    let tx_info = _bob_build_signed_otx().unwrap();
    let tx_view = tx_info.tx;
    let otx = tx_view_to_basic_otx(tx_view).unwrap();
    Ok(otx.into())
}
