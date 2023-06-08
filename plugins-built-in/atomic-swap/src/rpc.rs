use otx_plugin_protocol::Plugin;

use otx_plugin_protocol::PluginInfo;

use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

use std::sync::Arc;

#[rpc(server)]
pub trait AtomicSwapRpc {
    #[rpc(name = "get_atomic_swap_info")]
    fn get_atomic_swap_info(&self) -> RpcResult<PluginInfo>;
}

impl AtomicSwapRpc for Arc<Box<dyn Plugin + Send>> {
    fn get_atomic_swap_info(&self) -> RpcResult<PluginInfo> {
        let plugin_info = self.get_info();
        Ok(plugin_info)
    }
}
