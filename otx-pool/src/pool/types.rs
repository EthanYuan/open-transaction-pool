use ckb_jsonrpc_types::{Deserialize, Serialize};
use ckb_types::H256;
use otx_format::jsonrpc_types::OpenTransaction;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenTxStatus {
    /// Status "pending". The transaction is in the pool, and not proposed yet.
    Pending,
    /// Status "proposed". The transaction is in the pool and has been proposed.
    Proposed,
    /// Status "committed". The transaction has been committed to the canonical chain.
    Committed(H256),
    /// Status "unknown". The node has not seen the transaction,
    /// or it should be rejected but was cleared due to storage limitations.
    Unknown,
    // Status "rejected". The transaction has been recently removed from the pool.
    /// Due to storage limitations, the node can only hold the most recently removed transactions.
    Rejected(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenTxWithStatus {
    pub otx: OpenTransaction,
    pub status: OpenTxStatus,
}

impl OpenTxWithStatus {
    pub fn new(otx: OpenTransaction) -> Self {
        OpenTxWithStatus {
            otx,
            status: OpenTxStatus::Pending,
        }
    }
}
