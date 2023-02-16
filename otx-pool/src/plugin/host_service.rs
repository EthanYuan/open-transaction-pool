use otx_plugin_protocol::{MessageFromHost, MessageFromPlugin};

use ckb_types::core::service::Request;
use crossbeam_channel::{bounded, Sender};

use std::thread::{self, JoinHandle};

pub type ServiceHandler = Sender<Request<MessageFromPlugin, MessageFromHost>>;

#[derive(Debug)]
pub struct HostServiceProvider {
    handler: ServiceHandler,
    _thread: JoinHandle<()>,
}

impl HostServiceProvider {
    pub fn start() -> Result<HostServiceProvider, String> {
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
}
