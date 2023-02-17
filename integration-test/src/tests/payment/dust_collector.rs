use crate::tests::helper::{alice_build_signed_otx, bob_build_signed_otx};
use crate::IntegrationTest;

use otx_format::jsonrpc_types::tx_view::tx_view_to_otx;
use otx_format::types::packed;
use otx_pool::types::OpenTxStatus;
use utils::client::service_client::OtxPoolRpcClient;
use utils::const_definition::SERVICE_URI;

use anyhow::Result;
use ckb_jsonrpc_types::JsonBytes;
use ckb_types::prelude::Entity;

use std::thread::sleep;
use std::time::Duration;

inventory::submit!(IntegrationTest {
    name: "test_payment_dust_collect",
    test_fn: test_payment_dust_collect
});
fn test_payment_dust_collect() {
    let alice_otx = alice_build_otx().unwrap();
    let bob_otx = bob_build_otx().unwrap();

    let service_client = OtxPoolRpcClient::new(SERVICE_URI.to_string());
    let alice_otx_id = service_client
        .submit_otx(JsonBytes::from_bytes(alice_otx.as_bytes()))
        .unwrap();
    let alice_otx_with_status = service_client
        .query_otx_by_id(alice_otx_id)
        .unwrap()
        .unwrap();
    assert_eq!(alice_otx_with_status.status, OpenTxStatus::Pending);

    let bob_otx_id = service_client
        .submit_otx(JsonBytes::from_bytes(bob_otx.as_bytes()))
        .unwrap();
    let bob_otx_with_status = service_client.query_otx_by_id(bob_otx_id.clone()).unwrap().unwrap();
    assert_eq!(bob_otx_with_status.status, OpenTxStatus::Pending);

    sleep(Duration::from_secs(12));

    let bob_otx_with_status = service_client.query_otx_by_id(bob_otx_id).unwrap().unwrap();

    assert!(matches!(
        bob_otx_with_status.status,
        OpenTxStatus::Committed(_)
    ));
}

fn alice_build_otx() -> Result<packed::OpenTransaction> {
    let tx_info = alice_build_signed_otx().unwrap();
    let tx_view = tx_info.tx;
    let otx = tx_view_to_otx(tx_view).unwrap();
    Ok(otx.into())
}

fn bob_build_otx() -> Result<packed::OpenTransaction> {
    let tx_info = bob_build_signed_otx().unwrap();
    let tx_view = tx_info.tx;
    let otx = tx_view_to_otx(tx_view).unwrap();
    Ok(otx.into())
}
