#![allow(dead_code)]

use crate::const_definition::{CELL_BASE_MATURE_EPOCH, CKB_URI, GENESIS_EPOCH_LENGTH, MERCURY_URI};
use crate::utils::client::ckb_client::CkbRpcClient;
use crate::utils::client::mercury_client::MercuryRpcClient;

use anyhow::Result;
use ckb_jsonrpc_types::{OutputsValidator, Transaction};
use ckb_types::H256;
use serde::Serialize;

pub fn unlock_frozen_capacity_in_genesis() -> Result<()> {
    let ckb_uri = CKB_URI;
    let ckb_client = CkbRpcClient::new(ckb_uri.to_string());
    let epoch_view = ckb_client.get_current_epoch().expect("get_current_epoch");
    let current_epoch_number = epoch_view.number.value();
    if current_epoch_number < CELL_BASE_MATURE_EPOCH {
        for _ in 0..=(CELL_BASE_MATURE_EPOCH - current_epoch_number) * GENESIS_EPOCH_LENGTH {
            let ckb_client = CkbRpcClient::new(ckb_uri.to_string());
            let block_hash = ckb_client.generate_block().expect("generate block");
            log::trace!("generate new block: {:?}", block_hash.to_string());
        }
    }
    Ok(())
}

pub fn fast_forward_epochs(number: usize) -> Result<()> {
    generate_blocks(GENESIS_EPOCH_LENGTH as usize * number + 1)
}

pub fn generate_blocks(number: usize) -> Result<()> {
    let ckb_uri = CKB_URI;
    let ckb_rpc_client = CkbRpcClient::new(ckb_uri.to_string());
    for _ in 0..number {
        let block_hash = ckb_rpc_client.generate_block()?;
        log::trace!("generate new block: {:?}", block_hash.to_string());
    }
    Ok(())
}

pub fn aggregate_transactions_into_blocks() -> Result<()> {
    generate_blocks(3)?;
    generate_blocks(3)?;
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    mercury_client.wait_sync();
    Ok(())
}

pub fn send_transaction_to_ckb(tx: Transaction) -> Result<H256> {
    let ckb_uri = CKB_URI;
    let ckb_client = CkbRpcClient::new(ckb_uri.to_string());
    let tx_hash = ckb_client.send_transaction(tx, OutputsValidator::Passthrough)?;
    log::info!("send tx: 0x{}", tx_hash);
    aggregate_transactions_into_blocks()?;
    Ok(tx_hash)
}

pub fn dump_data<T>(data: &T, file_name: &str) -> Result<()>
where
    T: ?Sized + Serialize,
{
    let json_string = serde_json::to_string_pretty(data)?;
    std::fs::write(file_name, json_string).map_err(Into::into)
}
