use crate::pool::types::OpenTxWithStatus;

use super::{OtxPoolRpc, OtxPoolRpcImpl};

use ckb_jsonrpc_types::JsonBytes;
use ckb_types::H256;
use jsonrpc_core::Result as RpcResult;

impl OtxPoolRpc for OtxPoolRpcImpl {
    fn submit_otx(&self, otx: JsonBytes) -> RpcResult<H256> {
        self.otx_pool.insert(otx).map_err(Into::into)
    }

    fn query_otx_by_id(&self, id: H256) -> RpcResult<Option<OpenTxWithStatus>> {
        Ok(self.otx_pool.get_otx_by_id(id))
    }
}
