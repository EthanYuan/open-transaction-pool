pub mod rpc;

use crate::plugin::host_service::ServiceHandler;
use crate::plugin::plugin_proxy::{MsgHandler, PluginState, RequestHandler};
use crate::plugin::Plugin;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{MessageFromHost, MessageFromPlugin, PluginInfo};
use utils::aggregator::{Committer, SignInfo};
use utils::config::{built_in_plugins::SignerConfig, CkbConfig, ScriptConfig};

use anyhow::{anyhow, Result};
use ckb_sdk_open_tx::types::Address;
use ckb_types::core::service::Request;
use ckb_types::H256;
use crossbeam_channel::{bounded, select, unbounded};
use dashmap::DashMap;

use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

#[derive(Clone)]
struct Context {
    plugin_name: String,
    otxs: DashMap<H256, OpenTransaction>,
    sign_info: SignInfo,
    ckb_config: CkbConfig,
    _script_config: ScriptConfig,
    service_handler: ServiceHandler,
}

impl Context {
    fn new(
        plugin_name: &str,
        sign_info: SignInfo,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
        service_handler: ServiceHandler,
    ) -> Self {
        Context {
            plugin_name: plugin_name.to_owned(),
            otxs: DashMap::new(),
            sign_info,
            ckb_config,
            _script_config: script_config,
            service_handler,
        }
    }
}

pub struct Signer {
    state: PluginState,
    info: PluginInfo,

    /// Send request to plugin thread, and expect a response.
    request_handler: RequestHandler,

    /// Send notifaction/response to plugin thread.
    msg_handler: MsgHandler,

    _thread: JoinHandle<()>,
}

impl Plugin for Arc<Signer> {
    fn get_name(&self) -> String {
        self.info.name.clone()
    }

    fn msg_handler(&self) -> MsgHandler {
        self.msg_handler.clone()
    }

    fn request_handler(&self) -> RequestHandler {
        self.request_handler.clone()
    }

    fn get_info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn get_state(&self) -> PluginState {
        self.state.clone()
    }
}

impl Signer {
    pub fn new(
        service_handler: ServiceHandler,
        config: SignerConfig,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
    ) -> Result<Signer> {
        let name = "singer";
        let state = PluginState::new(PathBuf::default(), true, true);
        let info = PluginInfo::new(
            name,
            "This plugin indexes OTXs that are waiting to be signed and enables them to be signed using a hosted private key.",
            "1.0",
        );
        let key = env::var(config.get_env_key_name())?.parse::<H256>()?;
        let address = env::var(config.get_env_default_address())?
            .parse::<Address>()
            .map_err(|e| anyhow!(e))?;

        let context = Context::new(
            name,
            SignInfo::new(&address, &key, ckb_config.clone()),
            ckb_config,
            script_config,
            service_handler,
        );
        let (msg_handler, request_handler, thread) = Signer::start_process(Arc::new(context))?;
        Ok(Signer {
            state,
            info,
            msg_handler,
            request_handler,
            _thread: thread,
        })
    }
}

impl Signer {
    fn start_process(
        context: Arc<Context>,
    ) -> Result<(MsgHandler, RequestHandler, JoinHandle<()>)> {
        // the host request channel receives request from host to plugin
        let (host_request_sender, host_request_receiver) = bounded(1);
        // the channel sends notifications or responses from the host to plugin
        let (host_msg_sender, host_msg_receiver) = unbounded();

        let plugin_name = context.plugin_name.to_owned();
        // this thread processes information from host to plugin
        let thread = thread::spawn(move || {
            let do_select = || -> Result<bool, String> {
                select! {
                    // request from host to plugin
                    recv(host_request_receiver) -> msg => {
                        match msg {
                            Ok(Request { responder, arguments }) => {
                                log::debug!("{} receives request arguments: {:?}",
                                    context.plugin_name, arguments);
                                // handle
                                let response = (0, MessageFromPlugin::Ok);
                                responder.send(response).map_err(|err| err.to_string())?;
                                Ok(false)
                            }
                            Err(err) => Err(err.to_string())
                        }
                    }
                    // repsonse/notification from host to plugin
                    recv(host_msg_receiver) -> msg => {
                        match msg {
                            Ok(msg) => {
                                match msg {
                                    (_, MessageFromHost::NewInterval(_)) => {
                                    }
                                    (_, MessageFromHost::NewOtx(otx)) => {
                                        log::info!("{} receivers msg NewOtx hash: {:?}",
                                            context.plugin_name,
                                            otx.get_tx_hash().expect("get tx hash"));
                                        on_new_open_tx(context.clone(), otx);
                                    }
                                    (_, MessageFromHost::CommitOtx(otx_hashes)) => {
                                        on_commit_open_tx(context.clone(), otx_hashes);
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
                        log::error!("plugin {} error: {}", plugin_name, err);
                        break;
                    }
                }
            }
        });

        Ok((host_msg_sender, host_request_sender, thread))
    }
}

fn on_new_open_tx(context: Arc<Context>, otx: OpenTransaction) {
    log::info!("on_new_open_tx, index otxs count: {:?}", context.otxs.len());
    if let Ok(aggregate_count) = otx.get_aggregate_count() {
        log::info!("aggregate count: {:?}", aggregate_count);
        if aggregate_count == 1 {
            return;
        }
    }
    let otx_hash = otx.get_tx_hash().expect("get tx hash");
    context.otxs.insert(otx_hash, otx.clone());

    let ckb_tx = if let Ok(tx) = otx.try_into() {
        tx
    } else {
        log::error!("open tx converts to Ckb tx failed.");
        return;
    };

    // sign
    let signer = SignInfo::new(
        context.sign_info.secp_address(),
        context.sign_info.privkey(),
        context.ckb_config.clone(),
    );
    let signed_ckb_tx = signer.sign_ckb_tx(ckb_tx).unwrap();

    // send_ckb
    let committer = Committer::new(context.ckb_config.get_ckb_uri());
    let tx_hash = if let Ok(tx_hash) = committer.send_tx(signed_ckb_tx) {
        tx_hash
    } else {
        log::error!("failed to send final tx.");
        return;
    };
    log::info!("commit final Ckb tx: {:?}", tx_hash.to_string());

    // call host service to notify the host that the final tx has been sent
    let message = MessageFromPlugin::SentToCkb(tx_hash);
    if let Some(MessageFromHost::Ok) = Request::call(&context.service_handler, message) {
        context.otxs.clear();
    }
}

fn on_commit_open_tx(context: Arc<Context>, otx_hashes: Vec<H256>) {
    log::info!(
        "{} on commit open tx remove committed otx: {:?}",
        context.plugin_name,
        otx_hashes
            .iter()
            .map(|hash| hash.to_string())
            .collect::<Vec<String>>()
    );
    otx_hashes.iter().for_each(|otx_hash| {
        context.otxs.remove(otx_hash);
    })
}
