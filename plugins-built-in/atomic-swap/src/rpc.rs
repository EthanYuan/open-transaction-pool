use super::{AtomicSwap, SwapProposalWithCount};

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
    fn get_all_swap_proposals(&self) -> RpcResult<Vec<SwapProposalWithCount>>;
}

impl AtomicSwapRpc for Arc<AtomicSwap> {
    fn get_atomic_swap_info(&self) -> RpcResult<PluginInfo> {
        let plugin_info = self.get_info();
        Ok(plugin_info)
    }

    fn get_all_swap_proposals(&self) -> RpcResult<Vec<SwapProposalWithCount>> {
        let proposals = self
            .context
            .proposals
            .iter()
            .map(|p| SwapProposalWithCount::new(p.key().clone(), p.value().len()))
            .collect();
        Ok(proposals)
    }
}
