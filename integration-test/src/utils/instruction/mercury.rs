use crate::const_definition::{
    GENESIS_BUILT_IN_ADDRESS_1, GENESIS_BUILT_IN_ADDRESS_1_PRIVATE_KEY, MERCURY_URI, SCRIPT_CONFIG,
    UDT_1_HASH, UDT_1_HOLDER_ACP_ADDRESS, UDT_1_HOLDER_PK, UDT_1_HOLDER_SECP_ADDRESS, UDT_2_HASH,
    UDT_2_HOLDER_ACP_ADDRESS, UDT_2_HOLDER_PK, UDT_2_HOLDER_SECP_ADDRESS,
};
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::ckb::send_transaction_to_ckb;
use crate::utils::lock::secp::prepare_secp_address_with_ckb_capacity;
use crate::utils::signer::sign_transaction;

use utils::lock::acp::build_acp_address;
use utils::lock::get_udt_hash_by_owner;

use anyhow::Result;
use ckb_jsonrpc_types::OutPoint;
use ckb_sdk::Address;
use ckb_types::H256;

use core_rpc_types::{
    AssetInfo, AssetType, IOType, JsonItem, OutputCapacityProvider, SudtIssuePayload, ToInfo,
    TransferPayload,
};

pub fn issue_udt_1() -> Result<()> {
    log::info!("issue udt 1 for test");

    if UDT_1_HASH.get().is_some() {
        return Ok(());
    }

    // issue udt
    let (owner_address, owner_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(5000_0000_0000_0000)?;
    let udt_hash = get_udt_hash_by_owner(
        &owner_address,
        SCRIPT_CONFIG.get().unwrap().get_xudt_rce_code_hash(),
    )?;
    let _tx_hash = issue_udt_with_acp(&owner_address, &owner_address_pk, 20_000_000_000u128)?;
    let acp_address = build_acp_address(&owner_address)?;

    UDT_1_HASH.set(udt_hash).expect("init UDT_HASH_1");
    UDT_1_HOLDER_SECP_ADDRESS
        .set(owner_address)
        .expect("init UDT_1_HOLDER_ACP_ADDRESS");
    UDT_1_HOLDER_ACP_ADDRESS
        .set(acp_address)
        .expect("init UDT_1_HOLDER_ACP_ADDRESS");
    UDT_1_HOLDER_PK
        .set(owner_address_pk)
        .expect("init UDT_1_HOLDER_ACP_ADDRESS_PK");
    Ok(())
}

pub fn issue_udt_2() -> Result<()> {
    log::info!("issue udt 2 for test");

    if UDT_2_HASH.get().is_some() {
        return Ok(());
    }

    // issue udt
    let (owner_address, owner_address_pk, _) =
        prepare_secp_address_with_ckb_capacity(5000_0000_0000_0000)?;
    let udt_hash = get_udt_hash_by_owner(
        &owner_address,
        SCRIPT_CONFIG.get().unwrap().get_xudt_rce_code_hash(),
    )?;
    let _tx_hash = issue_udt_with_acp(&owner_address, &owner_address_pk, 20_000_000_000u128)?;
    let acp_address = build_acp_address(&owner_address)?;

    UDT_2_HASH.set(udt_hash).expect("init UDT_HASH_2");
    UDT_2_HOLDER_SECP_ADDRESS
        .set(owner_address)
        .expect("init UDT_2_HOLDER_ACP_ADDRESS");
    UDT_2_HOLDER_ACP_ADDRESS
        .set(acp_address)
        .expect("init UDT_2_HOLDER_ACP_ADDRESS");
    UDT_2_HOLDER_PK
        .set(owner_address_pk)
        .expect("init UDT_2_HOLDER_ACP_ADDRESS_PK");
    Ok(())
}

pub(crate) fn prepare_ckb_capacity(address: &Address, capacity: u64) -> Result<OutPoint> {
    let payload = TransferPayload {
        asset_info: AssetInfo::new_ckb(),
        from: vec![JsonItem::Address(GENESIS_BUILT_IN_ADDRESS_1.to_string())],
        to: vec![ToInfo {
            address: address.to_string(),
            amount: (capacity as u128).into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload)?;
    let tx = sign_transaction(tx, &[GENESIS_BUILT_IN_ADDRESS_1_PRIVATE_KEY])?;

    // send tx to ckb node
    let tx_hash = send_transaction_to_ckb(tx)?;
    let tx_info = mercury_client
        .get_transaction_info(tx_hash)?
        .transaction
        .expect("get transaction info");
    let out_point = &tx_info
        .records
        .into_iter()
        .find(|record| {
            record.asset_info.asset_type == AssetType::CKB
                && record.amount == (capacity as u128).into()
                && record.io_type == IOType::Output
        })
        .expect("find record")
        .out_point;

    Ok(out_point.to_owned())
}

pub(crate) fn issue_udt_with_acp(
    owner_address: &Address,
    owner_pk: &H256,
    udt_amount: u128,
) -> Result<H256> {
    let acp_address = build_acp_address(owner_address).expect("build acp address");
    let payload = SudtIssuePayload {
        owner: owner_address.to_string(),
        from: vec![JsonItem::Address(owner_address.to_string())],
        to: vec![ToInfo {
            address: acp_address.to_string(),
            amount: udt_amount.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        fee_rate: None,
        since: None,
    };

    // build tx
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_sudt_issue_transaction(payload)?;
    let tx = sign_transaction(tx, &[owner_pk.to_owned()])?;

    // send tx to ckb node
    send_transaction_to_ckb(tx)
}

pub fn prepare_udt_1(amount: u128, to_address: &Address) -> Result<H256> {
    issue_udt_1().unwrap();
    let udt_hash = UDT_1_HASH.get().unwrap();
    let payer_acp_address = UDT_1_HOLDER_ACP_ADDRESS.get().unwrap();
    let payer_acp_address_pk = UDT_1_HOLDER_PK.get().unwrap();
    prepare_udt(
        udt_hash,
        payer_acp_address,
        payer_acp_address_pk,
        amount,
        to_address,
    )
}

pub fn prepare_udt_2(amount: u128, to_address: &Address) -> Result<H256> {
    issue_udt_2().unwrap();
    let udt_hash = UDT_2_HASH.get().unwrap();
    let payer_acp_address = UDT_2_HOLDER_ACP_ADDRESS.get().unwrap();
    let payer_acp_address_pk = UDT_2_HOLDER_PK.get().unwrap();
    prepare_udt(
        udt_hash,
        payer_acp_address,
        payer_acp_address_pk,
        amount,
        to_address,
    )
}

fn prepare_udt(
    udt_hash: &H256,
    payer_acp_address: &Address,
    payer_acp_address_pk: &H256,
    amount: u128,
    to_address: &Address,
) -> Result<H256> {
    let payload = TransferPayload {
        asset_info: AssetInfo::new_udt(udt_hash.to_owned()),
        from: vec![
            JsonItem::Address(payer_acp_address.to_string()),
            JsonItem::Address(GENESIS_BUILT_IN_ADDRESS_1.to_string()),
        ],
        to: vec![ToInfo {
            address: to_address.to_string(),
            amount: amount.into(),
        }],
        output_capacity_provider: Some(OutputCapacityProvider::From),
        pay_fee: None,
        fee_rate: None,
        since: None,
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    let tx = mercury_client.build_transfer_transaction(payload).unwrap();
    let tx = sign_transaction(
        tx,
        &[
            payer_acp_address_pk.to_owned(),
            GENESIS_BUILT_IN_ADDRESS_1_PRIVATE_KEY.to_owned(),
        ],
    )
    .unwrap();
    send_transaction_to_ckb(tx)
}
