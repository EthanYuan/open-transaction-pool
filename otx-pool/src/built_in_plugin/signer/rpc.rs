use super::Signer;
use crate::error::{OtxPoolError, OtxRpcError};
use crate::plugin::Plugin;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{MessageFromHost, PluginInfo};

use ckb_sdk::Address;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

use std::{str::FromStr, sync::Arc};

#[rpc(server)]
pub trait SignerRpc {
    #[rpc(name = "get_signer_info")]
    fn get_signer_info(&self) -> RpcResult<PluginInfo>;

    #[rpc(name = "get_pending_sign_otxs")]
    fn get_pending_sign_otxs(&self, address: String) -> RpcResult<Vec<OpenTransaction>>;

    #[rpc(name = "send_signed_otx")]
    fn send_signed_otx(&self, otx: OpenTransaction) -> RpcResult<()>;
}

impl SignerRpc for Arc<Signer> {
    fn get_signer_info(&self) -> RpcResult<PluginInfo> {
        let plugin_info = self.get_info();
        Ok(plugin_info)
    }

    fn get_pending_sign_otxs(&self, address: String) -> RpcResult<Vec<OpenTransaction>> {
        let address = Address::from_str(&address)
            .map_err(OtxPoolError::RpcParamParseError)
            .map_err(Into::<OtxRpcError>::into)?;
        Ok(self.get_index_sign_otxs(address))
    }

    fn send_signed_otx(&self, otx: OpenTransaction) -> RpcResult<()> {
        let _ = self.msg_handler.send((0, MessageFromHost::SendTx(otx)));
        Ok(())
    }
}
