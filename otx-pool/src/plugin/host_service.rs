use crate::notify::NotifyController;

use otx_format::types::{OpenTxStatus, OpenTxWithStatus};
use otx_plugin_protocol::{MessageFromHost, MessageFromPlugin};

use ckb_types::core::service::Request;
use ckb_types::H256;
use crossbeam_channel::{bounded, Sender};
use dashmap::DashMap;

use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub type ServiceHandler = Sender<Request<MessageFromPlugin, MessageFromHost>>;

#[derive(Debug)]
pub struct HostServiceProvider {
    handler: ServiceHandler,
    _thread: JoinHandle<()>,
}

impl HostServiceProvider {
    pub fn start(
        notify_ctrl: NotifyController,
        raw_otxs: Arc<DashMap<H256, OpenTxWithStatus>>,
        sent_txs: Arc<DashMap<H256, Vec<H256>>>,
    ) -> Result<HostServiceProvider, String> {
        let (sender, receiver) = bounded(5);

        let handle = thread::spawn(move || loop {
            match receiver.recv() {
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
                        MessageFromPlugin::SendCkbTx((tx_hash, otx_hashes)) => {
                            Self::on_send_ckb_tx(
                                tx_hash,
                                otx_hashes,
                                notify_ctrl.clone(),
                                raw_otxs.clone(),
                                sent_txs.clone(),
                            );
                        }
                        _ => unreachable!(),
                    }
                }
            }
        });

        Ok(HostServiceProvider {
            _thread: handle,
            handler: sender,
        })
    }

    pub fn handler(&self) -> ServiceHandler {
        self.handler.clone()
    }

    pub fn on_send_ckb_tx(
        tx_hash: H256,
        otx_hashes: Vec<H256>,
        notify_ctrl: NotifyController,
        raw_otxs: Arc<DashMap<H256, OpenTxWithStatus>>,
        sent_txs: Arc<DashMap<H256, Vec<H256>>>,
    ) {
        log::info!(
            "on send ckb tx: {:?}, includes otxs: {:?}",
            tx_hash.to_string(),
            otx_hashes
                .iter()
                .map(|hash| hash.to_string())
                .collect::<Vec<String>>()
        );

        for otx_hash in otx_hashes.iter() {
            raw_otxs.get_mut(otx_hash).unwrap().status = OpenTxStatus::Committed(tx_hash.clone());
        }
        notify_ctrl.notify_commit_open_tx(otx_hashes.clone());
        sent_txs.insert(tx_hash, otx_hashes);
    }
}
