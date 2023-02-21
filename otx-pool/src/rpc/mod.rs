mod r#impl;

use super::pool::OtxPool;
use crate::notify::NotifyController;

use otx_format::types::OpenTxWithStatus;

use ckb_jsonrpc_types::JsonBytes;
use ckb_types::H256;
use dashmap::DashMap;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;

use std::sync::Arc;

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
    pub fn new(
        raw_otxs: Arc<DashMap<H256, OpenTxWithStatus>>,
        sent_txs: Arc<DashMap<H256, Vec<H256>>>,
        notify_ctrl: NotifyController,
    ) -> Self {
        let otx_pool = OtxPool::new(raw_otxs, sent_txs, notify_ctrl);
        OtxPoolRpcImpl { otx_pool }
    }
}
