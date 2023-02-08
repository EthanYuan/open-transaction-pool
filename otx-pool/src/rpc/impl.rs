use super::{OtxPoolRpc, OtxPoolRpcImpl};
use crate::pool::Id;

use otx_format::jsonrpc_types::OpenTransaction;

use ckb_jsonrpc_types::JsonBytes;
use jsonrpc_core::Result as RpcResult;

impl OtxPoolRpc for OtxPoolRpcImpl {
    fn submit_otx(&self, otx: JsonBytes) -> RpcResult<Id> {
        self.otx_pool.insert(otx).map_err(Into::into)
    }

    fn query_otx_by_id(&self, id: Id) -> RpcResult<Option<OpenTransaction>> {
        Ok(self.otx_pool.get_otx_by_id(id))
    }
}
