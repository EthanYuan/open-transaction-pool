use super::{request, RpcClient};

use otx_format::jsonrpc_types::OpenTransaction;
use otx_format::types::OpenTxStatus;
use otx_plugin_protocol::PluginInfo;

use anyhow::Result;
use ckb_jsonrpc_types::JsonBytes;
use ckb_types::H256;

pub struct OtxPoolRpcClient {
    client: RpcClient,
}

impl OtxPoolRpcClient {
    pub fn new(uri: String) -> Self {
        let client = RpcClient::new(uri);
        OtxPoolRpcClient { client }
    }

    pub fn submit_otx(&self, otx: JsonBytes) -> Result<H256> {
        request(&self.client, "submit_otx", vec![otx])
    }

    pub fn query_otx_status_by_id(&self, otx: H256) -> Result<Option<OpenTxStatus>> {
        request(&self.client, "query_otx_status_by_id", vec![otx])
    }

    pub fn get_signer_info(&self) -> Result<PluginInfo> {
        request(&self.client, "get_signer_info", ())
    }

    pub fn get_pending_sign_otxs(&self, address: String) -> Result<Vec<OpenTransaction>> {
        request(&self.client, "get_pending_sign_otxs", vec![address])
    }
}
