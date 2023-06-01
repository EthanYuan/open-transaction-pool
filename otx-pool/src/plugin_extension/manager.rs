use crate::notify::{NotifyController, RuntimeHandle};
use crate::plugin_extension::host_service::HostServiceProvider;
use crate::plugin_extension::plugin_proxy::PluginProxy;

use ckb_async_runtime::Handle;
use otx_plugin_protocol::{HostServiceHandler, Plugin, PluginInfo, PluginMeta};
use tokio::task::{block_in_place, JoinHandle};

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const PLUGINS_DIRNAME: &str = "plugins";
pub const INACTIVE_DIRNAME: &str = "plugins_inactive";

type PluginList = Vec<(String, Arc<Mutex<Box<dyn Plugin + Send>>>)>;

pub struct PluginManager {
    _plugin_dir: PathBuf,
    inactive_plugin_dir: PathBuf,

    // information about all plugins, including inactive ones
    plugin_configs: HashMap<String, (PluginMeta, PluginInfo)>,

    // proxies for activated plugin processes
    plugins: HashMap<String, Arc<Mutex<Box<dyn Plugin + Send>>>>,

    service_provider: HostServiceHandler,
    _event_thread: Option<JoinHandle<()>>,
}

impl PluginManager {
    pub fn new(host_dir: &Path, service_provider: HostServiceHandler) -> Self {
        let plugin_configs: HashMap<String, (PluginMeta, PluginInfo)> = HashMap::new();
        let plugins: HashMap<String, Arc<Mutex<Box<dyn Plugin + Send>>>> = HashMap::new();

        PluginManager {
            _plugin_dir: host_dir.join(PLUGINS_DIRNAME),
            inactive_plugin_dir: host_dir.join(INACTIVE_DIRNAME),
            plugin_configs,
            plugins,
            service_provider,
            _event_thread: None,
        }
    }

    pub fn register_built_in_plugins(&mut self, plugin: Box<dyn Plugin + Send>) {
        let plugin_info = plugin.get_info();
        let plugin_state = plugin.get_meta();
        self.plugin_configs
            .insert(plugin.get_name(), (plugin_state, plugin_info));
        self.plugins
            .insert(plugin.get_name(), Arc::new(Mutex::new(plugin)));
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
                self.plugins.insert(
                    plugin_name.to_owned(),
                    Arc::new(Mutex::new(Box::new(plugin_proxy))),
                );
            }
        }

        Ok(())
    }

    pub fn subscribe_events(&mut self, notify_ctrl: &NotifyController, runtime_handle: &Handle) {
        let plugins: PluginList = self
            .plugins
            .iter()
            .map(|(name, p)| (name.to_owned(), p.clone()))
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
                        plugins.iter().for_each(| (_, plugin) | {
                            let plugin = plugin.lock().expect("plugin lock should not fail");
                            block_in_place(|| plugin.on_new_intervel(elapsed));
                        })
                    }
                    Some(open_tx) = new_otx_event_receiver.recv() => {
                        plugins.iter().for_each(|(_, plugin) | {
                            let plugin = plugin.lock().expect("plugin lock should not fail");
                            block_in_place(|| plugin.on_new_otx(open_tx.clone()));
                        })
                    }
                    Some(otx_hash) = commit_otx_event_receiver.recv() => {
                        plugins.iter().for_each(|(_, plugin) | {
                            let plugin = plugin.lock().expect("plugin lock should not fail");
                            block_in_place(|| plugin.on_commit_otx(otx_hash.clone()));
                        })
                    }
                }
            }
        });
        self._event_thread = Some(event_listening_thread);
    }

    pub fn plugin_configs(&self) -> &HashMap<String, (PluginMeta, PluginInfo)> {
        &self.plugin_configs
    }

    pub fn service_handler(&self) -> HostServiceHandler {
        self.service_provider.clone()
    }

    fn load_plugin_configs(&self) -> Result<HashMap<String, (PluginMeta, PluginInfo)>, io::Error> {
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
                    let plugin_state = PluginMeta::new(path.clone(), *is_active, false);
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
