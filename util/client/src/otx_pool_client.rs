use super::{request, RpcClient};

use otx_format::{jsonrpc_types::OpenTransaction, types::OpenTxStatus};
use otx_pool_plugin_atomic_swap::SwapProposalWithOtxId;
use otx_pool_plugin_protocol::PluginInfo;

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

    pub fn get_atomic_swap_info(&self) -> Result<PluginInfo> {
        request(&self.client, "get_atomic_swap_info", ())
    }

    pub fn get_all_swap_proposals(&self) -> Result<Vec<SwapProposalWithOtxId>> {
        request(&self.client, "get_all_swap_proposals", ())
    }
}
