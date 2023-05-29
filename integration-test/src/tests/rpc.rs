use crate::const_definition::OTX_POOL_URI;
use crate::help::start_otx_pool;
use crate::tests::helper::build_pay_ckb_signed_otx;
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use otx_format::jsonrpc_types::tx_view::tx_view_to_basic_otx;
use otx_format::jsonrpc_types::OpenTransaction;
use otx_format::types::OpenTxStatus;
use utils::client::otx_pool_client::OtxPoolRpcClient;

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

    let tx_info = build_pay_ckb_signed_otx("alice", 151, 100, 51).unwrap();
    let tx_view = tx_info.tx;
    let otx = tx_view_to_basic_otx(tx_view).unwrap();

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
