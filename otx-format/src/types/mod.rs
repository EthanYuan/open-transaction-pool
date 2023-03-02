pub use crate::generated::packed;
use crate::jsonrpc_types::OpenTransaction;

use ckb_jsonrpc_types::{Deserialize, Serialize};
use ckb_types::H256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenTxStatus {
    /// Status "pending". The open transaction is in the pool, and not proposed yet.
    Pending,
    /// Status "proposed". The open transaction is in the pool and has been proposed by some agent.
    Proposed(String),
    /// Status "committed". The open transaction has been committed to the canonical chain.
    Committed(H256),
    /// Status "unknown". The pool has not seen the transaction,
    /// or it should be rejected but was cleared due to storage limitations.
    Unknown,
    // Status "rejected". The open transaction has been recently removed from the pool.
    /// Due to storage limitations, the pool can only hold the most recently removed transactions.
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

#[cfg(test)]
mod test {
    use crate::types::packed::OpenTransaction;

    use super::*;
    use ckb_jsonrpc_types::JsonBytes;
    use molecule::prelude::*;
    use packed::OpenTransactionBuilder;

    #[test]
    fn test_serialize() {
        let builder = OpenTransactionBuilder::default();
        let opentx = builder.build();
        let opentx_bytes = opentx.as_bytes();
        let json_rpc_format = JsonBytes::from_bytes(opentx_bytes);
        println!("{:?}", opentx);
        println!("{:?}", json_rpc_format);

        let opentx_bytes = json_rpc_format.as_bytes();
        println!("{:?}", opentx_bytes);
        let opentx_rebuild = OpenTransaction::from_slice(opentx_bytes).unwrap();

        assert_eq!(opentx.as_bytes(), opentx_rebuild.as_bytes());
    }
}
