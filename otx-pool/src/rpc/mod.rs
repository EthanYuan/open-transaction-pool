mod r#impl;

use super::pool::{Id, OtxPool};
use crate::notify::NotifyController;

use otx_format::jsonrpc_types::OpenTransaction;

use ckb_jsonrpc_types::JsonBytes;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

#[rpc(server)]
pub trait OtxPoolRpc {
    #[rpc(name = "submit_otx")]
    fn submit_otx(&self, otx: JsonBytes) -> RpcResult<Id>;

    #[rpc(name = "query_otx_by_id")]
    fn query_otx_by_id(&self, id: Id) -> RpcResult<Option<OpenTransaction>>;
}

pub struct OtxPoolRpcImpl {
    otx_pool: OtxPool,
}

impl OtxPoolRpcImpl {
    pub fn new(notify_ctrl: NotifyController) -> Self {
        let otx_pool = OtxPool::new(notify_ctrl);
        OtxPoolRpcImpl { otx_pool }
    }
}
