use crate::const_definition::{OTX_POOL_URI, SCRIPT_CONFIG};
use crate::help::start_otx_pool;
use crate::tests::small_blank_check::build_signed_otx;
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use client::OtxPoolRpcClient;
use otx_format::jsonrpc_types::OpenTransaction;
use otx_format::types::OpenTxStatus;
use otx_sdk::address::build_otx_address_from_secp_address;

use ckb_types::H256;

inventory::submit!(IntegrationTest {
    name: "test_service_rpc",
    test_fn: test_service_rpc
});
fn test_service_rpc() {
    let (address, pk) = generate_rand_secp_address_pk_pair();
    start_otx_pool(address, pk);

    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let otx = OpenTransaction::default();
    let id = otx.get_tx_hash().unwrap();
    let ret = service_client.submit_otx(otx);
    assert!(ret.is_ok());
    let ret = service_client.query_otx_status_by_id(id);
    assert!(ret.is_ok());
}

inventory::submit!(IntegrationTest {
    name: "test_service_rpc_submit_otx",
    test_fn: test_service_rpc_submit_otx
});
fn test_service_rpc_submit_otx() {
    let (address, pk) = generate_rand_secp_address_pk_pair();
    start_otx_pool(address, pk);

    // get otx lock script info
    let script_config = SCRIPT_CONFIG.get().unwrap().clone();
    let otx_lock_script_info = script_config.get_script_info("otx-sighash-lock").unwrap();

    // build otxs
    // pay 51 CKB
    let (address, pk) = generate_rand_secp_address_pk_pair();
    let otx_address = build_otx_address_from_secp_address(&address, &otx_lock_script_info).unwrap();
    let otx = build_signed_otx(
        "payer",
        &otx_address,
        (&address, &pk),
        151_0000_0000,
        100_0000_0000,
        vec![otx_lock_script_info],
    )
    .unwrap();

    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let id = service_client.submit_otx(otx).unwrap();
    log::debug!("id: {:?}", id);
    let status = service_client.query_otx_status_by_id(id).unwrap().unwrap();
    assert_eq!(status, OpenTxStatus::Pending);

    let ret = service_client
        .query_otx_status_by_id(H256::default())
        .unwrap();
    assert!(ret.is_none());
}
