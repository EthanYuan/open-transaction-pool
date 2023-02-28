use crate::const_definition::{CKB_URI, OTX_POOL_URI};
use crate::help::start_otx_pool;
use crate::tests::helper::build_pay_ckb_signed_otx;
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use otx_format::jsonrpc_types::tx_view::{otx_to_tx_view, tx_view_to_otx};
use otx_format::types::packed;
use utils::client::service_client::OtxPoolRpcClient;
use utils::const_definition::XUDT_CODE_HASH;

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
    let ret = service_client.query_otx_by_id(H256::default());
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
    let otx = tx_view_to_otx(
        tx_view.clone(),
        None,
        None,
        XUDT_CODE_HASH.get().unwrap().to_owned(),
        CKB_URI,
    )
    .unwrap();
    let otx: packed::OpenTransaction = otx.into();

    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let id = service_client
        .submit_otx(JsonBytes::from_bytes(otx.as_bytes()))
        .unwrap();
    let otx = service_client.query_otx_by_id(id).unwrap().unwrap();
    let tx_view_rebuilt = otx_to_tx_view(otx.otx).unwrap();
    assert_eq!(tx_view, tx_view_rebuilt);
}
