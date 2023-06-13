use crate::const_definition::{OTX_POOL_URI, SCRIPT_CONFIG};
use crate::help::start_otx_pool;
use crate::tests::atomic_swap_udt_to_udt::build_signed_otx;
use crate::utils::instruction::ckb::{aggregate_transactions_into_blocks, dump_data};
use crate::utils::instruction::mercury::prepare_ckb_capacity;
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use client::OtxPoolRpcClient;
use otx_sdk::address::build_otx_address_from_secp_address;

use std::thread::sleep;
use std::time::Duration;

inventory::submit!(IntegrationTest {
    name: "test_atomic_swap_rpc_get_info",
    test_fn: test_atomic_swap_rpc_get_info
});
fn test_atomic_swap_rpc_get_info() {
    // run otx pool
    let (address, pk) = generate_rand_secp_address_pk_pair();
    prepare_ckb_capacity(&address, 200_0000_0000u64).unwrap();
    start_otx_pool(address, pk);

    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let ret = service_client.get_atomic_swap_info().unwrap();
    assert_eq!(ret.name, "atomic swap");
}

inventory::submit!(IntegrationTest {
    name: "test_atomic_swap_rpc_get_all_swap_proposals",
    test_fn: test_atomic_swap_rpc_get_all_swap_proposals
});
fn test_atomic_swap_rpc_get_all_swap_proposals() {
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
        "alice",
        &otx_address,
        (&address, &pk),
        12,
        5,
        201_0000_0000,
        2,
        15,
        200_0000_0000,
        vec![otx_lock_script_info],
    )
    .unwrap();

    let service_client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    let id = service_client.submit_otx(otx).unwrap();
    log::debug!("id: {:?}", id);

    sleep(Duration::from_secs(5));
    aggregate_transactions_into_blocks().unwrap();

    let proposals = service_client.get_all_swap_proposals().unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].otx_id, id);
    assert_eq!(proposals[0].swap_proposal.buy_amount, 10);
    assert_eq!(proposals[0].swap_proposal.sell_amount, 10);
    assert_eq!(proposals[0].swap_proposal.pay_fee, 1_0000_0000);

    dump_data(&proposals, "./free-space/proposals.json").unwrap();
}
