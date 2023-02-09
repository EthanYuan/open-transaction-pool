use super::{request, RpcClient};

use anyhow::Result;
use ckb_jsonrpc_types::{EpochView, LocalNode, OutputsValidator, Transaction};
use ckb_types::H256;

pub struct CkbRpcClient {
    client: RpcClient,
}

impl CkbRpcClient {
    pub fn new(uri: String) -> Self {
        let client = RpcClient::new(uri);
        CkbRpcClient { client }
    }

    pub fn local_node_info(&self) -> Result<LocalNode> {
        request(&self.client, "local_node_info", ())
    }

    pub fn get_current_epoch(&self) -> Result<EpochView> {
        request(&self.client, "get_current_epoch", ())
    }

    pub fn generate_block(&self) -> Result<H256> {
        request(&self.client, "generate_block", ())
    }

    pub fn send_transaction(
        &self,
        tx: Transaction,
        outputs_validator: OutputsValidator,
    ) -> Result<H256> {
        request(&self.client, "send_transaction", (tx, outputs_validator))
    }
}
