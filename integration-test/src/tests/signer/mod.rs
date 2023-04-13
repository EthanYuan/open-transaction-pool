use crate::const_definition::OTX_POOL_URI;
use crate::help::start_otx_pool;
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use utils::client::otx_pool_client::OtxPoolRpcClient;

inventory::submit!(IntegrationTest {
    name: "test_plugin_rpc_get_plugin_info",
    test_fn: test_plugin_rpc_get_plugin_info
});
fn test_plugin_rpc_get_plugin_info() {
    let (address, pk) = generate_rand_secp_address_pk_pair();
    start_otx_pool(address, pk);

    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let plugin_info = service_client.get_signer_info().unwrap();
    assert_eq!(plugin_info.name, "singer");
}
