use crate::notify::NotifyController;
use crate::pool::OtxPool;

use ckb_types::prelude::Entity;
use otx_format::jsonrpc_types::OpenTransaction;
use otx_format::types::{packed, OpenTxStatus};
use otx_plugin_protocol::{HostServiceHandler, MessageFromHost, MessageFromPlugin};

use anyhow::{anyhow, Result};
use ckb_jsonrpc_types::JsonBytes;
use ckb_types::core::service::Request;
use ckb_types::H256;
use crossbeam_channel::{bounded, select, Sender};

use std::sync::Arc;
use std::thread::{self, JoinHandle};

#[derive(Debug)]
pub struct HostServiceProvider {
    handler: HostServiceHandler,
    stop_handler: Sender<()>,
    _thread: Option<JoinHandle<()>>,
}

impl HostServiceProvider {
    pub fn start(
        notify_ctrl: NotifyController,
        otx_pool: Arc<OtxPool>,
    ) -> Result<HostServiceProvider, String> {
        let (sender, receiver) = bounded(5);
        let (stop_sender, stop_receiver) = bounded(1);

        let handle = thread::spawn(move || loop {
            select! {
                recv(receiver) -> request => {
                    match request {
                        Err(err) => {
                            log::warn!("ServiceProvider receive request error: {:?}", err);
                            break;
                        }
                        Ok(Request {
                            responder,
                            arguments,
                        }) => {
                            log::debug!("ServiceProvider received a request: {:?}", arguments);
                            match arguments {
                                MessageFromPlugin::DiscardOtx(_id) => {
                                    let _ = responder.send(MessageFromHost::Ok);
                                }
                                MessageFromPlugin::SentToCkb(otx_hash) => {
                                    Self::handle_sent_ckb_tx(
                                        otx_hash,
                                        notify_ctrl.clone(),
                                        otx_pool.clone(),
                                    );
                                    let _ = responder.send(MessageFromHost::Ok);
                                }
                                MessageFromPlugin::MergeOtxsAndSentToCkb((otx_hashes, tx_hash)) => {
                                    Self::handle_merge_otxs_and_sent(
                                        otx_hashes,
                                        tx_hash,
                                        notify_ctrl.clone(),
                                        otx_pool.clone(),
                                    );
                                    let _ = responder.send(MessageFromHost::Ok);
                                }
                                MessageFromPlugin::NewMergedOtx((merged_otx, otx_hashes)) => {
                                    match Self::handle_new_merged_otx(
                                        merged_otx,
                                        otx_hashes,
                                        otx_pool.clone()
                                    ) {
                                        Ok(_) => {let _ = responder.send(MessageFromHost::Ok);}
                                        Err(err) => {
                                            log::warn!("handle new merged otx error: {:?}", err);
                                            let _ = responder.send(MessageFromHost::Error(err.to_string()));
                                        }
                                    }
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }
                recv(stop_receiver) -> request => {
                    match request {
                        Err(err) => {
                            log::warn!("ServiceProvider receive stop request error: {:?}", err);
                            break;
                        }
                        Ok(_) => {
                            log::info!("ServiceProvider received stop signal");
                            break;
                        }
                    }
                }
            }
        });

        Ok(HostServiceProvider {
            handler: sender,
            stop_handler: stop_sender,
            _thread: Some(handle),
        })
    }

    pub fn handler(&self) -> HostServiceHandler {
        self.handler.clone()
    }

    fn handle_new_merged_otx(
        new_merged_otx: OpenTransaction,
        included_otx_hashes: Vec<H256>,
        otx_pool: Arc<OtxPool>,
    ) -> Result<()> {
        let merged_otx_hash = if let Ok(hash) = new_merged_otx.get_tx_hash() {
            hash
        } else {
            return Err(anyhow!("invalid merged otx"));
        };
        log::info!(
            "handle new merged otx: {:?}, includes otxs: {:?}",
            merged_otx_hash,
            included_otx_hashes
                .iter()
                .map(|hash| hash.to_string())
                .collect::<Vec<String>>()
        );
        for otx_hash in included_otx_hashes.iter() {
            otx_pool.update_otx_status(otx_hash, OpenTxStatus::Merged(merged_otx_hash.clone()));
        }
        let otx: packed::OpenTransaction = new_merged_otx.into();
        otx_pool
            .insert(JsonBytes::from_bytes(otx.as_bytes()))
            .expect("insert merged otx");
        Ok(())
    }

    fn handle_sent_ckb_tx(
        final_otx_hash: H256,
        notify_ctrl: NotifyController,
        otx_pool: Arc<OtxPool>,
    ) {
        let otx_hashes: Vec<H256> = otx_pool
            .get_otxs_by_merged_otx_id(&final_otx_hash)
            .iter_mut()
            .map(|otx| otx.otx.get_or_insert_otx_id().expect("get otx id"))
            .collect();
        log::info!(
            "handle sent ckb tx: {:?}, includes otxs: {:?}",
            final_otx_hash.to_string(),
            otx_hashes
                .iter()
                .map(|hash| hash.to_string())
                .collect::<Vec<String>>()
        );

        for otx_hash in otx_hashes.iter() {
            otx_pool.update_otx_status(otx_hash, OpenTxStatus::Committed(final_otx_hash.clone()));
        }
        otx_pool.update_otx_status(
            &final_otx_hash,
            OpenTxStatus::Committed(final_otx_hash.clone()),
        );
        notify_ctrl.notify_commit_open_tx(otx_hashes.clone());
        otx_pool.insert_sent_tx(final_otx_hash, otx_hashes);
    }

    fn handle_merge_otxs_and_sent(
        otx_hashes: Vec<H256>,
        tx_hash: H256,
        notify_ctrl: NotifyController,
        otx_pool: Arc<OtxPool>,
    ) {
        log::info!(
            "handle sent ckb tx: {:?}, includes otxs: {:?}",
            tx_hash.to_string(),
            otx_hashes
                .iter()
                .map(|hash| hash.to_string())
                .collect::<Vec<String>>()
        );

        for otx_hash in otx_hashes.iter() {
            otx_pool.update_otx_status(otx_hash, OpenTxStatus::Committed(tx_hash.clone()));
        }
        notify_ctrl.notify_commit_open_tx(otx_hashes.clone());
        otx_pool.insert_sent_tx(tx_hash, otx_hashes);
    }
}

impl Drop for HostServiceProvider {
    fn drop(&mut self) {
        log::info!("HostServiceProvider drop");
        let _ = self.stop_handler.try_send(());
    }
}
