mod verifier;

use crate::error::InnerResult;
use crate::notify::NotifyController;

use otx_format::{
    jsonrpc_types::OpenTransaction,
    types::{packed, OpenTxStatus, OpenTxWithStatus},
};

use ckb_jsonrpc_types::JsonBytes;
use ckb_types::{prelude::Entity, H256};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;

pub struct OtxPool {
    raw_otxs: DashMap<H256, OpenTxWithStatus>,
    sent_txs: DashMap<H256, Vec<H256>>,
    notify_ctrl: NotifyController,
}

impl OtxPool {
    pub fn new(notify_ctrl: NotifyController) -> Self {
        let raw_otxs = DashMap::new();
        let sent_txs = DashMap::new();
        OtxPool {
            raw_otxs,
            sent_txs,
            notify_ctrl,
        }
    }

    pub fn insert(&self, otx: JsonBytes) -> InnerResult<H256> {
        let mut otx = parse_otx(otx)?;
        let tx_hash = otx.get_or_insert_otx_id()?;
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

    pub fn update_otx_status(&self, id: &H256, status: OpenTxStatus) {
        if let Some(mut otx) = self.raw_otxs.get_mut(id) {
            otx.status = status;
        }
    }

    pub fn insert_sent_tx(&self, tx_hash: H256, otx_hashes: Vec<H256>) {
        self.sent_txs.insert(tx_hash, otx_hashes);
    }

    pub fn get_otxs_by_merged_otx_id(&self, id: &H256) -> Vec<OpenTxWithStatus> {
        self.raw_otxs
            .iter()
            .filter(|pair| {
                if let OpenTxStatus::Merged(merged_otx_id) = &pair.value().status {
                    merged_otx_id == id
                } else {
                    false
                }
            })
            .map(|pair| pair.value().clone())
            .collect()
    }
}

fn parse_otx(otx: JsonBytes) -> InnerResult<OpenTransaction> {
    let r = packed::OpenTransaction::from_slice(otx.as_bytes());
    r.map(Into::into).map_err(Into::into)
}
