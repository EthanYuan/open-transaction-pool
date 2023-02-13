mod r#impl;

use super::pool::OtxPool;
use crate::{notify::NotifyController, pool::types::OpenTxWithStatus};

use ckb_jsonrpc_types::JsonBytes;
use ckb_types::H256;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

#[rpc(server)]
pub trait OtxPoolRpc {
    #[rpc(name = "submit_otx")]
    fn submit_otx(&self, otx: JsonBytes) -> RpcResult<H256>;

    #[rpc(name = "query_otx_by_id")]
    fn query_otx_by_id(&self, id: H256) -> RpcResult<Option<OpenTxWithStatus>>;
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
