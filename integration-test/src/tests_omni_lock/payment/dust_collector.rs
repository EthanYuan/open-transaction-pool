use crate::const_definition::{CKB_URI, MERCURY_URI, OTX_POOL_URI, SCRIPT_CONFIG};
use crate::help::start_otx_pool;
use crate::tests_omni_lock::helper::build_pay_ckb_signed_otx;
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::ckb::aggregate_transactions_into_blocks;
use crate::utils::instruction::mercury::{prepare_ckb_capacity, prepare_udt_1};
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use config::CkbConfig;
use dust_collector::DEFAULT_FEE;
use otx_format::jsonrpc_types::tx_view::tx_view_to_otx;
use otx_format::types::{packed, OpenTxStatus};
use otx_sdk::client::OtxPoolRpcClient;

use anyhow::Result;
use core_rpc_types::{GetBalancePayload, JsonItem};

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

inventory::submit!(IntegrationTest {
    name: "test_payment_dust_collect_ckb",
    test_fn: test_payment_dust_collect_ckb
});
fn test_payment_dust_collect_ckb() {
    // run otx pool
    let (dust_collector_address, pk) = generate_rand_secp_address_pk_pair();
    prepare_ckb_capacity(&dust_collector_address, 200_0000_0000u64).unwrap();
    prepare_udt_1(200u128, &dust_collector_address).unwrap();
    start_otx_pool(dust_collector_address.clone(), pk);

    // check dust collector assets
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
    let alice_otx_id = service_client.submit_otx(alice_otx.into()).unwrap();
    let bob_otx_id = service_client.submit_otx(bob_otx.into()).unwrap();

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

    // check dust collector assets
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 2);
    assert_eq!(
        200_0000_0000u128 + 53_0000_0000u128 - DEFAULT_FEE as u128,
        response.balances[0].free.into()
    );
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
    let otx = tx_view_to_otx(
        tx_view,
        1,
        CkbConfig::new("ckb_dev", CKB_URI),
        SCRIPT_CONFIG.get().unwrap().to_owned(),
    )
    .unwrap();
    Ok(otx.into())
}
