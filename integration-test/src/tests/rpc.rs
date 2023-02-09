use crate::IntegrationTest;

use otx_format::jsonrpc_types::tx_view::{otx_to_tx_view, tx_view_to_otx};
use otx_format::types::packed;
use utils::client::ckb_cli_client::{ckb_cli_get_capacity, ckb_cli_transfer_ckb};
use utils::client::service_client::ServiceRpcClient;
use utils::const_definition::SERVICE_URI;
use utils::instruction::ckb::dump_data;
use utils::lock::omni::{MultiSigArgs, TxInfo};
use utils::wallet::{GenOpenTxArgs, Wallet};

use anyhow::Result;
use ckb_jsonrpc_types::JsonBytes;
use ckb_sdk::{unlock::IdentityFlag, HumanCapacity};
use ckb_types::prelude::Entity;

use std::str::FromStr;

inventory::submit!(IntegrationTest {
    name: "test_service_rpc",
    test_fn: test_service_rpc
});
fn test_service_rpc() {
    let service_client = ServiceRpcClient::new(SERVICE_URI.to_string());
    let ret = service_client.submit_otx(JsonBytes::default());
    assert!(ret.is_err());
    let ret = service_client.query_otx_by_id(u64::MAX);
    assert!(ret.is_ok())
}

inventory::submit!(IntegrationTest {
    name: "test_service_rpc_submit_otx",
    test_fn: test_service_rpc_submit_otx
});
fn test_service_rpc_submit_otx() {
    let tx_info = build_signed_otx().unwrap();
    let tx_view = tx_info.tx;
    let otx = tx_view_to_otx(tx_view.clone()).unwrap();
    let otx: packed::OpenTransaction = otx.into();

    let service_client = ServiceRpcClient::new(SERVICE_URI.to_string());
    let id = service_client
        .submit_otx(JsonBytes::from_bytes(otx.as_bytes()))
        .unwrap();
    let otx = service_client.query_otx_by_id(id).unwrap().unwrap();
    let tx_view_rebuilt = otx_to_tx_view(otx).unwrap();
    assert_eq!(tx_view, tx_view_rebuilt);
}

pub fn build_signed_otx() -> Result<TxInfo> {
    // 1. init Alice's wallet instance
    let alice_wallet = Wallet::init_account();
    let alice_omni_address = alice_wallet.get_omni_otx_address();

    // 2. transfer capacity to alice omni address
    let _tx_hash = ckb_cli_transfer_ckb(alice_omni_address, 151).unwrap();
    let capacity = ckb_cli_get_capacity(alice_omni_address).unwrap();
    assert_eq!(151f64, capacity);

    // 3. alice generate open transaction, pay 51 CKB
    let gen_open_tx_args = GenOpenTxArgs {
        omni_identity_flag: IdentityFlag::PubkeyHash,
        multis_args: MultiSigArgs {
            require_first_n: 1,
            threshold: 1,
            sighash_address: vec![],
        },
        receiver: alice_omni_address.to_owned(),
        capacity_with_open: Some((
            HumanCapacity::from_str("100.0").unwrap(),
            HumanCapacity::from_str("51.0").unwrap(),
        )),
        udt_amount_with_open: None,
        fee_rate: 0,
    };
    let open_tx = alice_wallet.gen_open_tx(&gen_open_tx_args).unwrap();
    let file = "./free-space/usercase_alice_otx_unsigned.json";
    dump_data(&open_tx, file).unwrap();

    // 4. alice sign the otx
    let open_tx = alice_wallet.sign_open_tx(open_tx).unwrap();
    dump_data(&open_tx, "./free-space/usercase_alice_otx_signed.json").unwrap();

    Ok(open_tx)
}
