use crate::error::InnerResult;
use crate::notify::NotifyController;

use otx_format::jsonrpc_types::tx_view::otx_to_tx_view;
use otx_format::{
    jsonrpc_types::OpenTransaction,
    types::{packed, OpenTxWithStatus},
};

use ckb_jsonrpc_types::JsonBytes;
use ckb_types::{prelude::Entity, H256};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;

use std::sync::Arc;

pub struct OtxPool {
    raw_otxs: Arc<DashMap<H256, OpenTxWithStatus>>,
    _sent_txs: Arc<DashMap<H256, Vec<H256>>>,
    notify_ctrl: NotifyController,
}

impl OtxPool {
    pub fn new(
        raw_otxs: Arc<DashMap<H256, OpenTxWithStatus>>,
        sent_txs: Arc<DashMap<H256, Vec<H256>>>,
        notify_ctrl: NotifyController,
    ) -> Self {
        OtxPool {
            raw_otxs,
            _sent_txs: sent_txs,
            notify_ctrl,
        }
    }

    pub fn insert(&self, otx: JsonBytes) -> InnerResult<H256> {
        let otx = parse_otx(otx)?;
        let tx_hash = {
            let tx_view = otx_to_tx_view(otx.clone())?;
            tx_view.hash
        };
        match self.raw_otxs.entry(tx_hash.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(OpenTxWithStatus::new(otx.clone()));
                self.notify_ctrl.notify_new_open_tx(otx)
            }
            Entry::Occupied(_) => {}
        };
        Ok(tx_hash)
    }

    pub fn get_otx_by_id(&self, id: H256) -> Option<OpenTxWithStatus> {
        self.raw_otxs.get(&id).map(|pair| pair.value().clone())
    }
}

fn parse_otx(otx: JsonBytes) -> InnerResult<OpenTransaction> {
    let r = packed::OpenTransaction::from_slice(otx.as_bytes());
    r.map(Into::into).map_err(Into::into)
}
