use crate::error::InnerResult;
use crate::notify::NotifyController;

use otx_format::{jsonrpc_types::OpenTransaction, types::packed};

use ckb_jsonrpc_types::JsonBytes;
use ckb_types::prelude::Entity;
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub type Id = u64;

pub struct OtxPool {
    raw_otxs: DashMap<Id, OpenTransaction>,
    notify_ctrl: NotifyController,
}

impl OtxPool {
    pub fn new(notify_ctrl: NotifyController) -> Self {
        OtxPool {
            raw_otxs: DashMap::new(),
            notify_ctrl,
        }
    }

    pub fn insert(&self, otx: JsonBytes) -> InnerResult<Id> {
        let id = {
            let mut s = DefaultHasher::new();
            otx.hash(&mut s);
            s.finish()
        };
        let otx = parse_otx(otx)?;
        match self.raw_otxs.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(otx.clone());
                self.notify_ctrl.notify_new_open_tx(otx)
            }
            Entry::Occupied(_) => {}
        };
        Ok(id)
    }

    pub fn get_otx_by_id(&self, id: Id) -> Option<OpenTransaction> {
        self.raw_otxs.get(&id).map(|pair| pair.value().clone())
    }
}

fn parse_otx(otx: JsonBytes) -> InnerResult<OpenTransaction> {
    let r = packed::OpenTransaction::from_slice(otx.as_bytes());
    r.map(Into::into).map_err(Into::into)
}
