use super::plugin_proxy::MsgHandler;
use super::plugin_proxy::{PluginProxy, PluginState};
use super::service::ServiceProvider;
use crate::notify::NotifyController;

use otx_plugin_protocol::MessageFromHost;

use ckb_async_runtime::Handle;
use otx_plugin_protocol::PluginInfo;
use tokio::task::JoinHandle;

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const PLUGINS_DIRNAME: &str = "plugins";
pub const INACTIVE_DIRNAME: &str = "plugins_inactive";

pub struct PluginManager {
    _plugin_dir: PathBuf,

    // information about all plugins, including inactive ones
    plugin_configs: HashMap<String, (PluginState, PluginInfo)>,

    // proxies for activated plugin processes
    _plugin_proxies: HashMap<String, PluginProxy>,

    _service_provider: ServiceProvider,
    _notify_thread: JoinHandle<()>,
}

impl PluginManager {
    pub fn load_plugin_configs(
        host_dir: &Path,
    ) -> Result<HashMap<String, (PluginState, PluginInfo)>, io::Error> {
        let plugin_dir = host_dir.join(PLUGINS_DIRNAME);
        if !plugin_dir.exists() {
            fs::create_dir_all(&plugin_dir)?;
        }
        let inactive_plugin_dir = host_dir.join(INACTIVE_DIRNAME);
        if !inactive_plugin_dir.exists() {
            fs::create_dir_all(&inactive_plugin_dir)?;
        }

        let mut plugin_configs = HashMap::new();
        for (dir, is_active) in &[(&plugin_dir, true), (&inactive_plugin_dir, false)] {
            for entry in fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_file() {
                    let plugin_state = PluginState::new(path.clone(), *is_active);
                    match PluginProxy::get_plugin_info(path.clone()) {
                        Ok(plugin_info) => {
                            log::info!("Loaded plugin: {}", plugin_info.name);
                            plugin_configs.insert(
                                plugin_info.clone().name,
                                (plugin_state, plugin_info.clone()),
                            );
                        }
                        Err(err) => {
                            log::warn!("get_config error: {}, path: {:?}", err, path);
                        }
                    }
                }
            }
        }
        Ok(plugin_configs)
    }

    pub fn init(
        handle: Handle,
        notify_ctrl: NotifyController,
        host_dir: &Path,
    ) -> Result<PluginManager, String> {
        let plugin_dir = host_dir.join(PLUGINS_DIRNAME);
        let plugin_configs = Self::load_plugin_configs(host_dir).map_err(|err| err.to_string())?;

        let mut plugin_proxies = HashMap::new();

        // Make sure ServiceProvider start before all daemon processes
        let service_provider = ServiceProvider::start()?;

        for (plugin_name, (plugin_state, plugin_info)) in plugin_configs.iter() {
            if plugin_state.is_active {
                let plugin_proxy = PluginProxy::start_process(
                    handle.clone(),
                    plugin_state.to_owned(),
                    plugin_info.to_owned(),
                    service_provider.handler().clone(),
                )?;
                plugin_proxies.insert(plugin_name.to_owned(), plugin_proxy);
            }
        }

        let plugins: Vec<(String, MsgHandler)> = plugin_proxies
            .iter()
            .map(|(name, p)| (name.to_owned(), p.msg_handler()))
            .collect();

        // subscribe pool event
        let mut interval_receiver =
            handle.block_on(notify_ctrl.subscribe_interval("plugin manager"));
        let notify_thread = handle.spawn(async move {
            loop {
                tokio::select! {
                    Some(()) = interval_receiver.recv() => {
                        plugins.iter().for_each(|(_, notify_handler)| {
                            let _ = notify_handler.send((0, MessageFromHost::NewInterval));
                        })
                    }
                }
            }
        });

        Ok(PluginManager {
            _plugin_dir: plugin_dir,
            plugin_configs,
            _plugin_proxies: plugin_proxies,
            _service_provider: service_provider,
            _notify_thread: notify_thread,
        })
    }

    pub fn plugin_configs(&self) -> &HashMap<String, (PluginState, PluginInfo)> {
        &self.plugin_configs
    }
}
