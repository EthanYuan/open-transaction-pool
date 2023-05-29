use super::{request, RpcClient};

use otx_format::{jsonrpc_types::OpenTransaction, types::OpenTxStatus};

use anyhow::Result;
use ckb_types::H256;

pub struct OtxPoolRpcClient {
    client: RpcClient,
}

impl OtxPoolRpcClient {
    pub fn new(uri: String) -> Self {
        let client = RpcClient::new(uri);
        OtxPoolRpcClient { client }
    }

    pub fn submit_otx(&self, otx: OpenTransaction) -> Result<H256> {
        request(&self.client, "submit_otx", vec![otx])
    }

    pub fn query_otx_status_by_id(&self, otx: H256) -> Result<Option<OpenTxStatus>> {
        request(&self.client, "query_otx_status_by_id", vec![otx])
    }
}
