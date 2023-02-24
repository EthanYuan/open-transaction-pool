use crate::error::OtxFormatError;
use crate::jsonrpc_types::constant::basic_keys::OTX_META_VERSION;
use crate::jsonrpc_types::{OpenTransaction, OtxKeyPair, OtxMap};

use anyhow::Result;
use ckb_jsonrpc_types::{CellDep, CellInput, CellOutput, JsonBytes, TransactionView, Uint32};
use ckb_sdk::{CkbRpcClient, IndexerRpcClient};
use ckb_types::constants::TX_VERSION;
use ckb_types::core::TransactionBuilder;

struct Creator {
    ckb_rpc_client: CkbRpcClient,
    indexer_rpc_client: IndexerRpcClient,
}

impl Creator {
    pub fn new(uri: &str) -> Self {
        Creator {
            ckb_rpc_client: CkbRpcClient::new(uri),
            indexer_rpc_client: IndexerRpcClient::new(uri),
        }
    }

    pub fn new_from_tx_view(&self, tx_view: TransactionView) -> OpenTransaction {
        todo!()
    }
}
