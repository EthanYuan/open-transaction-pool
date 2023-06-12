use super::{AtomicSwap, SwapProposalWithOtxs};

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
    fn get_all_swap_proposals(&self) -> RpcResult<Vec<SwapProposalWithOtxs>>;
}

impl AtomicSwapRpc for Arc<AtomicSwap> {
    fn get_atomic_swap_info(&self) -> RpcResult<PluginInfo> {
        let plugin_info = self.get_info();
        Ok(plugin_info)
    }

    fn get_all_swap_proposals(&self) -> RpcResult<Vec<SwapProposalWithOtxs>> {
        let proposals = self
            .context
            .proposals
            .iter()
            .map(|p| {
                SwapProposalWithOtxs::new(
                    p.key().clone(),
                    p.value().iter().map(|id| id.to_owned()).collect(),
                )
            })
            .collect();
        Ok(proposals)
    }
}
