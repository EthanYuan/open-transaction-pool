use crate::const_definition::OTX_POOL_URI;
use crate::help::start_otx_pool;
use crate::tests::helper::build_pay_ckb_signed_otx;
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use otx_format::jsonrpc_types::tx_view::tx_view_to_basic_otx;
use otx_format::types::{packed, OpenTxStatus};
use utils::client::otx_pool_client::OtxPoolRpcClient;

use ckb_jsonrpc_types::JsonBytes;
use ckb_types::{prelude::Entity, H256};

inventory::submit!(IntegrationTest {
    name: "test_service_rpc",
    test_fn: test_service_rpc
});
fn test_service_rpc() {
    let (address, pk) = generate_rand_secp_address_pk_pair();
    start_otx_pool(address, pk);

    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let ret = service_client.submit_otx(JsonBytes::default());
    assert!(ret.is_err());
    let ret = service_client.query_otx_status_by_id(H256::default());
    assert!(ret.is_ok());
}

inventory::submit!(IntegrationTest {
    name: "test_service_rpc_submit_otx",
    test_fn: test_service_rpc_submit_otx
});
fn test_service_rpc_submit_otx() {
    let (address, pk) = generate_rand_secp_address_pk_pair();
    start_otx_pool(address, pk);

    let tx_info = build_pay_ckb_signed_otx("alice", 151, 100, 51).unwrap();
    let tx_view = tx_info.tx;
    let otx = tx_view_to_basic_otx(tx_view).unwrap();
    let otx: packed::OpenTransaction = otx.into();

    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let otx = JsonBytes::from_bytes(otx.as_bytes());
    log::debug!("otx: {:?}", serde_json::to_string_pretty(&otx).unwrap());
    let id = service_client.submit_otx(otx).unwrap();
    log::debug!("id: {:?}", id);
    let status = service_client.query_otx_status_by_id(id).unwrap().unwrap();
    assert_eq!(status, OpenTxStatus::Pending);

    let ret = service_client.query_otx_status_by_id(H256::default()).unwrap();
    assert!(ret.is_none());
}
