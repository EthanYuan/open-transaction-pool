use super::helper::alice_build_signed_otx;
use crate::IntegrationTest;

use otx_format::jsonrpc_types::tx_view::{otx_to_tx_view, tx_view_to_otx};
use otx_format::types::packed;
use utils::client::service_client::OtxPoolRpcClient;
use utils::const_definition::SERVICE_URI;

use ckb_jsonrpc_types::JsonBytes;
use ckb_types::{prelude::Entity, H256};

inventory::submit!(IntegrationTest {
    name: "test_service_rpc",
    test_fn: test_service_rpc
});
fn test_service_rpc() {
    let service_client = OtxPoolRpcClient::new(SERVICE_URI.to_string());
    let ret = service_client.submit_otx(JsonBytes::default());
    assert!(ret.is_err());
    let ret = service_client.query_otx_by_id(H256::default());
    assert!(ret.is_ok())
}

inventory::submit!(IntegrationTest {
    name: "test_service_rpc_submit_otx",
    test_fn: test_service_rpc_submit_otx
});
fn test_service_rpc_submit_otx() {
    let tx_info = alice_build_signed_otx().unwrap();
    let tx_view = tx_info.tx;
    let otx = tx_view_to_otx(tx_view.clone()).unwrap();
    let otx: packed::OpenTransaction = otx.into();

    let service_client = OtxPoolRpcClient::new(SERVICE_URI.to_string());
    let id = service_client
        .submit_otx(JsonBytes::from_bytes(otx.as_bytes()))
        .unwrap();
    let otx = service_client.query_otx_by_id(id).unwrap().unwrap();
    let tx_view_rebuilt = otx_to_tx_view(otx.otx).unwrap();
    assert_eq!(tx_view, tx_view_rebuilt);
}
