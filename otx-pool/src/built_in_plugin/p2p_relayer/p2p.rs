use ckb_async_runtime::Handle as RuntimeHandle;
use ckb_types::core::service::Request;
use crossbeam_channel::{select, Receiver};
use otx_plugin_protocol::{MessageFromHost, MessageFromPlugin};
use std::str;
use tentacle::{
    async_trait,
    builder::{MetaBuilder, ServiceBuilder},
    context::{ProtocolContext, ProtocolContextMutRef, ServiceContext},
    secio::SecioKeyPair,
    service::{ProtocolHandle, ProtocolMeta, ServiceError, ServiceEvent, TargetProtocol},
    traits::{ServiceHandle, ServiceProtocol},
    ProtocolId, SessionId,
};

// Any protocol will be abstracted into a ProtocolMeta structure.
// From an implementation point of view, tentacle treats any protocol equally
fn create_relayer_meta(id: ProtocolId) -> ProtocolMeta {
    MetaBuilder::new()
        .id(id)
        .service_handle(move || {
            // All protocol use the same handle.
            // This is just an example. In the actual environment, this should be a different handle.
            let handle = Box::new(RelayerProtocol::default());
            ProtocolHandle::Callback(handle)
        })
        .build()
}

#[derive(Default)]
struct RelayerProtocol {
    connected_session_ids: Vec<SessionId>,
}

#[async_trait]
impl ServiceProtocol for RelayerProtocol {
    async fn init(&mut self, _context: &mut ProtocolContext) {}

    async fn connected(&mut self, context: ProtocolContextMutRef<'_>, version: &str) {
        let session = context.session;
        self.connected_session_ids.push(session.id);
        log::info!(
            "proto id [{}] open on session [{}], address: [{}], type: [{:?}], version: {}",
            context.proto_id,
            session.id,
            session.address,
            session.ty,
            version
        );
        log::info!("connected sessions are: {:?}", self.connected_session_ids);
    }

    async fn disconnected(&mut self, context: ProtocolContextMutRef<'_>) {
        let new_list = self
            .connected_session_ids
            .iter()
            .filter(|&id| id != &context.session.id)
            .cloned()
            .collect();
        self.connected_session_ids = new_list;

        log::info!(
            "proto id [{}] close on session [{}]",
            context.proto_id,
            context.session.id
        );
    }

    async fn received(&mut self, context: ProtocolContextMutRef<'_>, data: bytes::Bytes) {
        log::info!(
            "received from [{}]: proto [{}] data {:?}",
            context.session.id,
            context.proto_id,
            str::from_utf8(data.as_ref()).unwrap(),
        );
        // TODO(now): deserialize data
        // TODO(now): send the received otx to pool
    }
}

struct P2PServiceHandle;

#[async_trait]
impl ServiceHandle for P2PServiceHandle {
    async fn handle_error(&mut self, _context: &mut ServiceContext, error: ServiceError) {
        log::info!("service error: {:?}", error);
    }
    async fn handle_event(&mut self, _context: &mut ServiceContext, event: ServiceEvent) {
        log::info!("service event: {:?}", event);
    }
}

#[derive(Default)]
pub struct P2PBuilder {
    listen_address: Option<String>,
    dial_address: Option<String>,
}

impl P2PBuilder {
    fn new() -> Self {
        Self::default()
    }

    pub fn listen(self: &mut Self, address: &str) {
        self.listen_address = Some(address.to_owned());
    }

    pub fn dial(self: &mut Self, address: &str) {
        self.dial_address = Some(address.to_owned());
    }

    pub fn spawn(
        self: Self,
        handle: &RuntimeHandle,
        request_receiver: Receiver<Request<(u64, MessageFromHost), (u64, MessageFromPlugin)>>,
        msg_receiver: Receiver<(u64, MessageFromHost)>,
    ) {
        // TODO(later): use async channel
        handle.spawn_blocking(move || {
            let do_select = || -> Result<bool, String> {
                select! {
                    // request from host to plugin
                    recv(request_receiver) -> msg => {
                        match msg {
                            Ok(Request { responder, arguments }) => {
                                log::debug!("p2p_relayer receives request arguments: {:?}", arguments);
                                let response = (0, MessageFromPlugin::Ok);
                                responder.send(response).map_err(|err| err.to_string())?;
                                Ok(false)
                            }
                            Err(err) => Err(err.to_string())
                        }
                    }
                    // repsonse/notification from host to plugin
                    recv(msg_receiver) -> msg => {
                        match msg {
                            Ok(msg) => {
                                match msg {
                                    (_, MessageFromHost::NewInterval(_)) => {
                                    }
                                    (_, MessageFromHost::NewOtx(otx)) => {
                                        log::info!("p2p_relayer receivers msg NewOtx hash: {:?}", otx.get_tx_hash().expect("get tx hash"));
                                        // TODO(now): broadcast otx
                                    }
                                    (_, MessageFromHost::CommitOtx(_)) => {
                                    }
                                    _ => unreachable!(),
                                }
                                Ok(false)
                            }
                            Err(err) => Err(err.to_string())
                        }
                    }
                }
            };
            loop {
                match do_select() {
                    Ok(true) => {
                        break;
                    }
                    Ok(false) => (),
                    Err(err) => {
                        log::error!("plugin p2p_relayer error: {}", err);
                        break;
                    }
                }
            }
        });

        // TODO(later): graceful shutdown
        handle.spawn(async {
            let mut service = ServiceBuilder::default()
                .insert_protocol(create_relayer_meta(0.into()))
                .key_pair(SecioKeyPair::secp256k1_generated())
                .build(P2PServiceHandle);
            if let Some(address) = self.listen_address {
                service
                    .listen(address.parse().expect("valid listen address"))
                    .await
                    .expect("listen ok");
            }
            if let Some(address) = self.dial_address {
                service
                    .dial(
                        address.parse().expect("valid dial address"),
                        TargetProtocol::All,
                    )
                    .await
                    .expect("dial ok");
            }
            service.run().await
        });
    }
}

pub fn builder() -> P2PBuilder {
    P2PBuilder::new()
}
