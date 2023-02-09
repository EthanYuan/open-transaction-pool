use crate::const_definition::{CELL_BASE_MATURE_EPOCH, CKB_URI, GENESIS_EPOCH_LENGTH};

use crate::client::ckb_client::CkbRpcClient;

use anyhow::Result;
use ckb_jsonrpc_types::{OutputsValidator, Transaction};
use ckb_types::H256;
use serde::Serialize;

pub fn unlock_frozen_capacity_in_genesis() {
    let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
    let epoch_view = ckb_client.get_current_epoch().expect("get_current_epoch");
    let current_epoch_number = epoch_view.number.value();
    if current_epoch_number < CELL_BASE_MATURE_EPOCH {
        for _ in 0..=(CELL_BASE_MATURE_EPOCH - current_epoch_number) * GENESIS_EPOCH_LENGTH {
            let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
            let block_hash = ckb_client.generate_block().expect("generate block");
            println!("generate new block: {:?}", block_hash.to_string());
        }
    }
}

pub fn fast_forward_epochs(number: usize) -> Result<()> {
    generate_blocks(GENESIS_EPOCH_LENGTH as usize * number + 1)
}

pub fn generate_blocks(number: usize) -> Result<()> {
    let ckb_rpc_client = CkbRpcClient::new(CKB_URI.to_string());
    for _ in 0..number {
        let block_hash = ckb_rpc_client.generate_block()?;
        println!("generate new block: {:?}", block_hash.to_string());
    }
    Ok(())
}

pub fn aggregate_transactions_into_blocks() -> Result<()> {
    generate_blocks(3)?;
    generate_blocks(3)?;
    Ok(())
}

pub fn send_transaction_to_ckb(tx: Transaction) -> Result<H256> {
    let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
    let tx_hash = ckb_client.send_transaction(tx, OutputsValidator::Passthrough)?;
    println!("send tx: 0x{}", tx_hash);
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
