use super::Signer;
use crate::plugin::Plugin;

use otx_plugin_protocol::PluginInfo;

use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

use std::sync::Arc;

#[rpc(server)]
pub trait SignerRpc {
    #[rpc(name = "get_signer_info")]
    fn get_signer_info(&self) -> RpcResult<PluginInfo>;
}

impl SignerRpc for Arc<Signer> {
    fn get_signer_info(&self) -> RpcResult<PluginInfo> {
        let plugin_info = self.get_info();
        Ok(plugin_info)
    }
}
