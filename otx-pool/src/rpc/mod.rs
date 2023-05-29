mod r#impl;

use super::pool::OtxPool;

use otx_format::{jsonrpc_types::OpenTransaction, types::OpenTxStatus};

use ckb_types::H256;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

use std::sync::Arc;

#[rpc(server)]
pub trait OtxPoolRpc {
    #[rpc(name = "submit_otx")]
    fn submit_otx(&self, otx: OpenTransaction) -> RpcResult<H256>;

    #[rpc(name = "query_otx_status_by_id")]
    fn query_otx_status_by_id(&self, id: H256) -> RpcResult<Option<OpenTxStatus>>;
}

pub struct OtxPoolRpcImpl {
    otx_pool: Arc<OtxPool>,
}

impl OtxPoolRpcImpl {
    pub fn new(otx_pool: Arc<OtxPool>) -> Self {
        OtxPoolRpcImpl { otx_pool }
    }
}
