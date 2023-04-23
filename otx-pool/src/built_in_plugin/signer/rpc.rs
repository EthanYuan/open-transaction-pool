use super::Signer;
use crate::error::{OtxPoolError, OtxRpcError};
use crate::plugin::Plugin;

use ckb_sdk::Address;
use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::PluginInfo;

use jsonrpc_core::Error;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

use std::{str::FromStr, sync::Arc};

#[rpc(server)]
pub trait SignerRpc {
    #[rpc(name = "get_signer_info")]
    fn get_signer_info(&self) -> RpcResult<PluginInfo>;

    #[rpc(name = "get_pending_sign_otxs")]
    fn get_pending_sign_otxs(&self, address: String) -> RpcResult<Vec<OpenTransaction>>;
}

impl SignerRpc for Arc<Signer> {
    fn get_signer_info(&self) -> RpcResult<PluginInfo> {
        let plugin_info = self.get_info();
        Ok(plugin_info)
    }

    fn get_pending_sign_otxs(&self, address: String) -> RpcResult<Vec<OpenTransaction>> {
        let address = Address::from_str(&address)
            .map_err(OtxPoolError::RpcParamParseError)
            .map_err(Into::<OtxRpcError>::into)
            .map_err(Into::<Error>::into)?;
        Ok(self.get_index_sign_otxs(address))
    }
}
