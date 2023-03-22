use super::plugin_proxy::MsgHandler;
use super::plugin_proxy::{PluginProxy, PluginState};
use crate::notify::{NotifyController, RuntimeHandle};
use crate::plugin::host_service::{HostServiceProvider, ServiceHandler};
use crate::plugin::Plugin;

use ckb_async_runtime::Handle;
use otx_plugin_protocol::{MessageFromHost, PluginInfo};
use tokio::task::JoinHandle;

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const PLUGINS_DIRNAME: &str = "plugins";
pub const INACTIVE_DIRNAME: &str = "plugins_inactive";

pub struct PluginManager {
    _plugin_dir: PathBuf,
    inactive_plugin_dir: PathBuf,

    // information about all plugins, including inactive ones
    plugin_configs: HashMap<String, (PluginState, PluginInfo)>,

    // proxies for activated plugin processes
    _plugins: HashMap<String, Box<dyn Plugin>>,

    service_provider: ServiceHandler,
    _event_thread: Option<JoinHandle<()>>,
}

impl PluginManager {
    pub fn new(host_dir: &Path, service_provider: ServiceHandler) -> Self {
        let plugin_configs: HashMap<String, (PluginState, PluginInfo)> = HashMap::new();
        let plugins: HashMap<String, Box<dyn Plugin>> = HashMap::new();

        PluginManager {
            _plugin_dir: host_dir.join(PLUGINS_DIRNAME),
            inactive_plugin_dir: host_dir.join(INACTIVE_DIRNAME),
            plugin_configs,
            _plugins: plugins,
            service_provider,
            _event_thread: None,
        }
    }

    pub fn register_built_in_plugins(&mut self, plugin: Box<dyn Plugin>) {
        let plugin_info = plugin.get_info();
        let plugin_state = plugin.get_state();
        self.plugin_configs
            .insert(plugin.get_name(), (plugin_state, plugin_info));
        self._plugins.insert(plugin.get_name(), plugin);
    }

    pub fn load_third_party_plugins(
        &mut self,
        runtime_handle: &RuntimeHandle,
        service_provider: &HostServiceProvider,
    ) -> Result<(), String> {
        // load plugins
        log::info!("load third-party plugins");
        for (plugin_name, (plugin_state, plugin_info)) in
            self.load_plugin_configs().map_err(|err| err.to_string())?
        {
            self.plugin_configs.insert(
                plugin_name.clone(),
                (plugin_state.to_owned(), plugin_info.to_owned()),
            );
            if plugin_state.is_active {
                let plugin_proxy = PluginProxy::start_process(
                    runtime_handle.clone(),
                    plugin_state,
                    plugin_info,
                    service_provider.handler(),
                )?;
                self._plugins
                    .insert(plugin_name.to_owned(), Box::new(plugin_proxy));
            }
        }

        Ok(())
    }

    pub fn subscribe_events(&mut self, notify_ctrl: &NotifyController, runtime_handle: &Handle) {
        let plugin_msg_handlers: Vec<(String, MsgHandler)> = self
            ._plugins
            .iter()
            .map(|(name, p)| (name.to_owned(), p.msg_handler()))
            .collect();

        let mut interval_event_receiver =
            runtime_handle.block_on(notify_ctrl.subscribe_interval("plugin manager"));
        let mut new_otx_event_receiver =
            runtime_handle.block_on(notify_ctrl.subscribe_new_open_tx("plugin manager"));
        let mut commit_otx_event_receiver =
            runtime_handle.block_on(notify_ctrl.subscribe_commit_open_tx("plugin manager"));
        let event_listening_thread = runtime_handle.spawn(async move {
                loop {
                    tokio::select! {
                        Some(elapsed) = interval_event_receiver.recv() => {
                            plugin_msg_handlers.iter().for_each(|(_, msg_handler)| {
                                let _ = msg_handler.send((0, MessageFromHost::NewInterval(elapsed)));
                            })
                        }
                        Some(open_tx) = new_otx_event_receiver.recv() => {
                            plugin_msg_handlers.iter().for_each(|(_, msg_handler)| {
                                let _ = msg_handler.send((0, MessageFromHost::NewOtx(open_tx.clone())));
                            })
                        }
                        Some(otx_hash) = commit_otx_event_receiver.recv() => {
                            plugin_msg_handlers.iter().for_each(|(_, msg_handler)| {
                                let _ = msg_handler.send((0, MessageFromHost::CommitOtx(otx_hash.clone())));
                            })
                        }
                    }
                }
            });
        self._event_thread = Some(event_listening_thread);
    }

    pub fn plugin_configs(&self) -> &HashMap<String, (PluginState, PluginInfo)> {
        &self.plugin_configs
    }

    pub fn service_handler(&self) -> ServiceHandler {
        self.service_provider.clone()
    }

    fn load_plugin_configs(&self) -> Result<HashMap<String, (PluginState, PluginInfo)>, io::Error> {
        if !self._plugin_dir.exists() {
            fs::create_dir_all(&self._plugin_dir)?;
        }
        if !self.inactive_plugin_dir.exists() {
            fs::create_dir_all(&self.inactive_plugin_dir)?;
        }

        let mut plugin_configs = HashMap::new();
        for (dir, is_active) in &[
            (&self._plugin_dir, true),
            (&self.inactive_plugin_dir, false),
        ] {
            for entry in fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_file() {
                    let plugin_state = PluginState::new(path.clone(), *is_active, false);
                    match PluginProxy::load_plugin_info(path.clone()) {
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
}
