use super::host_service::{HostServiceProvider, ServiceHandler};
use super::plugin_proxy::MsgHandler;
use super::plugin_proxy::{PluginProxy, PluginState};
use crate::built_in_plugin::DustCollector;
use crate::notify::{NotifyController, RuntimeHandle};
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

    // information about all plugins, including inactive ones
    plugin_configs: HashMap<String, (PluginState, PluginInfo)>,

    // proxies for activated plugin processes
    _plugins: HashMap<String, Box<dyn Plugin>>,

    service_provider: HostServiceProvider,
    _event_thread: JoinHandle<()>,
}

impl PluginManager {
    pub fn init(
        runtime_handle: RuntimeHandle,
        notify_ctrl: NotifyController,
        host_dir: &Path,
    ) -> Result<PluginManager, String> {
        let mut plugin_configs: HashMap<String, (PluginState, PluginInfo)> = HashMap::new();
        let mut plugins: HashMap<String, Box<dyn Plugin>> = HashMap::new();

        // Make sure ServiceProvider start before all daemon processes
        let service_provider = HostServiceProvider::start()?;

        // load plugins
        log::info!("load plugins");
        for (plugin_name, (plugin_state, plugin_info)) in
            load_plugin_configs(host_dir).map_err(|err| err.to_string())?
        {
            plugin_configs.insert(
                plugin_name.clone(),
                (plugin_state.to_owned(), plugin_info.to_owned()),
            );
            if plugin_state.is_active {
                let plugin_proxy = PluginProxy::start_process(
                    runtime_handle.clone(),
                    plugin_state,
                    plugin_info,
                    service_provider.handler().clone(),
                )?;
                plugins.insert(plugin_name.to_owned(), Box::new(plugin_proxy));
            }
        }

        // init built-in plugins
        log::info!("init built-in plugins");
        let dust_collector =
            DustCollector::new(runtime_handle.clone(), service_provider.handler())?;
        add_built_in_plugin(Box::new(dust_collector), &mut plugin_configs, &mut plugins);

        // subscribe events
        let event_listening_thread = subscribe_events(
            notify_ctrl,
            plugins
                .iter()
                .map(|(name, p)| (name.to_owned(), p.msg_handler()))
                .collect(),
            runtime_handle,
        );

        Ok(PluginManager {
            _plugin_dir: host_dir.join(PLUGINS_DIRNAME),
            plugin_configs,
            _plugins: plugins,
            service_provider,
            _event_thread: event_listening_thread,
        })
    }

    pub fn plugin_configs(&self) -> &HashMap<String, (PluginState, PluginInfo)> {
        &self.plugin_configs
    }

    pub fn service_handler(&self) -> ServiceHandler {
        self.service_provider.handler()
    }
}

fn load_plugin_configs(
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

fn add_built_in_plugin(
    plugin: Box<dyn Plugin>,
    plugin_configs: &mut HashMap<String, (PluginState, PluginInfo)>,
    plugins: &mut HashMap<String, Box<dyn Plugin>>,
) {
    let plugin_info = plugin.get_info();
    let plugin_state = plugin.get_state();
    plugin_configs.insert(plugin.get_name(), (plugin_state, plugin_info));
    plugins.insert(plugin.get_name(), plugin);
}

fn subscribe_events(
    notify_ctrl: NotifyController,
    plugin_msg_handlers: Vec<(String, MsgHandler)>,
    runtime_handle: Handle,
) -> JoinHandle<()> {
    let mut interval_event_receiver =
        runtime_handle.block_on(notify_ctrl.subscribe_interval("plugin manager"));
    let mut new_otx_event_receiver =
        runtime_handle.block_on(notify_ctrl.subscribe_new_open_tx("plugin manager"));
    runtime_handle.spawn(async move {
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
            }
        }
    })
}
