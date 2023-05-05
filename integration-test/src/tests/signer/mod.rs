use crate::const_definition::{CKB_URI, MERCURY_URI, OTX_POOL_URI};
use crate::help::start_otx_pool;
use crate::tests::payment::dust_collector::build_pay_ckb_otx;
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::ckb::aggregate_transactions_into_blocks;
use crate::utils::instruction::mercury::{prepare_ckb_capacity, prepare_udt_1};
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use otx_format::jsonrpc_types::tx_view::tx_view_to_basic_otx;
use otx_format::types::OpenTxStatus;
use utils::aggregator::SignInfo;
use utils::client::otx_pool_client::OtxPoolRpcClient;

use ckb_jsonrpc_types::JsonBytes;
use ckb_types::prelude::Entity;
use core_rpc_types::{GetBalancePayload, JsonItem};
use utils::config::CkbConfig;

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

inventory::submit!(IntegrationTest {
    name: "test_plugin_rpc_get_plugin_info",
    test_fn: test_plugin_rpc_get_plugin_info
});
fn test_plugin_rpc_get_plugin_info() {
    let (address, pk) = generate_rand_secp_address_pk_pair();
    start_otx_pool(address, Some(pk));

    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let plugin_info = service_client.get_signer_info().unwrap();
    assert_eq!(plugin_info.name, "singer");
}

inventory::submit!(IntegrationTest {
    name: "test_plugin_rpc_get_pending_sign_otxs",
    test_fn: test_plugin_rpc_get_pending_sign_otxs
});
fn test_plugin_rpc_get_pending_sign_otxs() {
    let (address, pk) = generate_rand_secp_address_pk_pair();
    start_otx_pool(address.clone(), Some(pk));

    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let otxs = service_client
        .get_pending_sign_otxs(address.to_string())
        .unwrap();
    assert_eq!(otxs.len(), 0);
}

inventory::submit!(IntegrationTest {
    name: "test_plugin_rpc_get_pending_sign_otxs_with_one_otx",
    test_fn: test_plugin_rpc_get_pending_sign_otxs_with_one_otx
});
fn test_plugin_rpc_get_pending_sign_otxs_with_one_otx() {
    // run otx pool
    let (dust_collector_address, pk) = generate_rand_secp_address_pk_pair();
    prepare_ckb_capacity(&dust_collector_address, 200_0000_0000u64).unwrap();
    prepare_udt_1(200u128, &dust_collector_address).unwrap();
    start_otx_pool(dust_collector_address.clone(), None);

    // check dust collector assets
    let payload = GetBalancePayload {
        item: JsonItem::Address(dust_collector_address.to_string()),
        asset_infos: HashSet::new(),
        extra: None,
        tip_block_number: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let response = mercury_client.get_balance(payload).unwrap();
    assert_eq!(response.balances.len(), 2);
    assert_eq!(200_0000_0000u128, response.balances[0].free.into());
    assert_eq!(142_0000_0000u128, response.balances[0].occupied.into());
    assert_eq!(200u128, response.balances[1].free.into());

    // build otxs
    let alice_otx = build_pay_ckb_otx("alice", 151, 100, 51).unwrap();
    let bob_otx = build_pay_ckb_otx("bob", 202, 200, 2).unwrap();

    // submit otxs
    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let _alice_otx_id = service_client
        .submit_otx(JsonBytes::from_bytes(alice_otx.as_bytes()))
        .unwrap();
    let _bob_otx_id = service_client
        .submit_otx(JsonBytes::from_bytes(bob_otx.as_bytes()))
        .unwrap();

    // query otx after a few secs
    sleep(Duration::from_secs(12));
    aggregate_transactions_into_blocks().unwrap();

    let otxs = service_client
        .get_pending_sign_otxs(dust_collector_address.to_string())
        .unwrap();
    assert_eq!(otxs.len(), 1);

    // sign
    let ckb_tx = if let Ok(tx) = otxs[0].clone().try_into() {
        tx
    } else {
        log::error!("open tx converts to Ckb tx failed.");
        return;
    };
    let sign_info = SignInfo::new(
        &dust_collector_address,
        &pk,
        CkbConfig::new("ckb_dev", CKB_URI),
    );
    let tx_view = sign_info.sign_ckb_tx(ckb_tx).unwrap();
    let otx = tx_view_to_basic_otx(tx_view).unwrap();

    // send signed tx to otx pool
    let ret = service_client.send_signed_otx(otx.clone());
    println!("ret: {:?}", ret);
    assert!(ret.is_ok());

    // query otx after a few secs
    sleep(Duration::from_secs(12));
    let otxs = service_client
        .get_pending_sign_otxs(dust_collector_address.to_string())
        .unwrap();
    assert_eq!(otxs.len(), 0);

    let status = service_client
        .query_otx_status_by_id(otx.get_tx_hash().unwrap())
        .unwrap()
        .unwrap();
    assert!(matches!(status, OpenTxStatus::Committed(_)));
}
