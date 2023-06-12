use super::{AtomicSwap, SwapProposalWithOtxId};

use otx_plugin_protocol::Plugin;
use otx_plugin_protocol::PluginInfo;

use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

use std::sync::Arc;

#[rpc(server)]
pub trait AtomicSwapRpc {
    #[rpc(name = "get_atomic_swap_info")]
    fn get_atomic_swap_info(&self) -> RpcResult<PluginInfo>;

    #[rpc(name = "get_all_swap_proposals")]
    fn get_all_swap_proposals(&self) -> RpcResult<Vec<SwapProposalWithOtxId>>;
}

impl AtomicSwapRpc for Arc<AtomicSwap> {
    fn get_atomic_swap_info(&self) -> RpcResult<PluginInfo> {
        let plugin_info = self.get_info();
        Ok(plugin_info)
    }

    fn get_all_swap_proposals(&self) -> RpcResult<Vec<SwapProposalWithOtxId>> {
        let proposals = self
            .context
            .otxs
            .iter()
            .map(|item| {
                let otx_id = item.key().to_owned();
                let swap_proposal = item.value().1.to_owned();
                SwapProposalWithOtxId {
                    otx_id,
                    swap_proposal,
                }
            })
            .collect();
        Ok(proposals)
    }
}
