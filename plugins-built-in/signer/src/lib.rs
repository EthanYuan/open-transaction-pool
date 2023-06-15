mod helper;

use helper::SignInfo;

use otx_format::jsonrpc_types::OpenTransaction;
use otx_plugin_protocol::{
    HostServiceHandler, MessageFromHost, MessageFromPlugin, Plugin, PluginInfo, PluginMeta,
};
use otx_pool_config::{built_in_plugins::SignerConfig, CkbConfig, ScriptConfig};
use otx_sdk::build_tx::send_tx;

use anyhow::{anyhow, Result};
use ckb_sdk::types::Address;
use ckb_types::core::service::Request;
use ckb_types::H256;
use dashmap::DashMap;

use std::env;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
struct Context {
    plugin_name: String,
    otxs: Arc<DashMap<H256, OpenTransaction>>,
    sign_info: SignInfo,
    ckb_config: CkbConfig,
    _script_config: ScriptConfig,
    service_handler: HostServiceHandler,
}

impl Context {
    fn new(
        plugin_name: &str,
        sign_info: SignInfo,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
        service_handler: HostServiceHandler,
    ) -> Self {
        Context {
            plugin_name: plugin_name.to_owned(),
            otxs: Arc::new(DashMap::new()),
            sign_info,
            ckb_config,
            _script_config: script_config,
            service_handler,
        }
    }
}

pub struct Signer {
    meta: PluginMeta,
    info: PluginInfo,
    context: Context,
}

impl Signer {
    pub fn new(
        service_handler: HostServiceHandler,
        config: SignerConfig,
        ckb_config: CkbConfig,
        script_config: ScriptConfig,
    ) -> Result<Signer> {
        let name = "singer";
        let state = PluginMeta::new(PathBuf::default(), true, true);
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
        Ok(Signer {
            meta: state,
            info,
            context,
        })
    }
}

impl Plugin for Signer {
    fn get_name(&self) -> String {
        self.info.name.clone()
    }

    fn get_info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn get_meta(&self) -> PluginMeta {
        self.meta.clone()
    }

    fn on_new_otx(&self, otx: OpenTransaction) {
        log::info!(
            "on_new_open_tx, index otxs count: {:?}",
            self.context.otxs.len()
        );
        if let Ok(aggregate_count) = otx.get_aggregate_count() {
            log::info!("aggregate count: {:?}", aggregate_count);
            if aggregate_count == 1 {
                return;
            }
        }
        let otx_hash = otx.get_tx_hash().expect("get tx hash");
        self.context.otxs.insert(otx_hash, otx.clone());

        let ckb_tx = if let Ok(tx) = otx.try_into() {
            tx
        } else {
            log::error!("open tx converts to Ckb tx failed.");
            return;
        };

        // sign
        let signer = SignInfo::new(
            self.context.sign_info.secp_address(),
            self.context.sign_info.privkey(),
            self.context.ckb_config.clone(),
        );
        let signed_ckb_tx = signer.sign_ckb_tx(ckb_tx).unwrap();

        // send_ckb
        let tx_hash =
            if let Ok(tx_hash) = send_tx(self.context.ckb_config.get_ckb_uri(), signed_ckb_tx) {
                tx_hash
            } else {
                log::error!("failed to send final tx.");
                return;
            };
        log::info!("commit final Ckb tx: {:?}", tx_hash.to_string());

        // call host service to notify the host that the final tx has been sent
        let message = MessageFromPlugin::SentToCkb(tx_hash);
        if let Some(MessageFromHost::Ok) = Request::call(&self.context.service_handler, message) {
            self.context.otxs.clear();
        }
    }

    fn on_commit_otx(&self, otx_hashes: Vec<H256>) {
        log::info!(
            "{} on commit open tx remove committed otx: {:?}",
            self.context.plugin_name,
            otx_hashes
                .iter()
                .map(|hash| hash.to_string())
                .collect::<Vec<String>>()
        );
        otx_hashes.iter().for_each(|otx_hash| {
            self.context.otxs.remove(otx_hash);
        })
    }
}
