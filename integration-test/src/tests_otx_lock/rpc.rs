use crate::const_definition::OTX_POOL_URI;
use crate::help::start_otx_pool;
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_sdk::client::OtxPoolRpcClient;

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
