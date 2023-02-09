use super::{request, RpcClient};

use otx_format::jsonrpc_types::OpenTransaction;

use anyhow::Result;
use ckb_jsonrpc_types::JsonBytes;
use ckb_types::H256;

pub struct ServiceRpcClient {
    client: RpcClient,
}

impl ServiceRpcClient {
    pub fn new(uri: String) -> Self {
        let client = RpcClient::new(uri);
        ServiceRpcClient { client }
    }

    pub fn submit_otx(&self, otx: JsonBytes) -> Result<H256> {
        request(&self.client, "submit_otx", vec![otx])
    }

    pub fn query_otx_by_id(&self, otx: H256) -> Result<Option<OpenTransaction>> {
        request(&self.client, "query_otx_by_id", vec![otx])
    }
}
