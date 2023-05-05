use super::Signer;
use crate::error::{OtxPoolError, OtxRpcError};
use crate::plugin::Plugin;

use ckb_types::H256;
use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{MessageFromPlugin, PluginInfo};
use utils::aggregator::Committer;

use ckb_jsonrpc_types::TransactionView;
use ckb_sdk::Address;
use ckb_types::core::service::Request;
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

    #[rpc(name = "submit_sent_tx_hash")]
    fn submit_sent_tx_hash(&self, tx_hash: H256) -> RpcResult<()>;
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
        // send tx to ckb
        let signed_ckb_tx: TransactionView = otx.try_into().map_err(Into::<OtxRpcError>::into)?;
        let committer = Committer::new(self.context.ckb_config.get_ckb_uri());
        let tx_hash = committer
            .send_tx(signed_ckb_tx)
            .map_err(|e| OtxPoolError::RpcParamParseError(e.to_string()))
            .map_err(Into::<OtxRpcError>::into)?;

        // call host service to notify the host that the final tx has been sent
        let message = MessageFromPlugin::SentToCkb(tx_hash);
        Request::call(&self.context.service_handler, message);
        Ok(())
    }

    fn submit_sent_tx_hash(&self, tx_hash: H256) -> RpcResult<()> {
        let message = MessageFromPlugin::SentToCkb(tx_hash);
        Request::call(&self.context.service_handler, message);
        Ok(())
    }
}
