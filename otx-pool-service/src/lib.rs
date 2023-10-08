pub mod error;
pub mod logo;
pub mod notify;
pub mod plugin_extension;
pub mod pool;
pub mod rpc;

use notify::{NotifyController, NotifyService};
use otx_pool_config::NetworkConfig;
use otx_pool_plugin_protocol::{HostServiceHandler, Plugin, PluginInfo, PluginMeta};
use plugin_extension::host_service::HostServiceProvider;
use plugin_extension::manager::PluginManager;
use pool::OtxPool;
use rpc::{OtxPoolRpc, OtxPoolRpcImpl};

use anyhow::{anyhow, Result};
use ckb_async_runtime::{new_global_runtime, Handle, Runtime};
use jsonrpc_core::{IoDelegate, IoHandler};
use jsonrpc_http_server::{Server, ServerBuilder};
use jsonrpc_server_utils::cors::AccessControlAllowOrigin;
use jsonrpc_server_utils::hosts::DomainsValidation;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration, Instant};

use std::{net::SocketAddr, path::Path, sync::Arc};

const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const INTERVAL: Duration = Duration::from_secs(2);
const PLUGIN_ROOT: &str = "./free-space";

pub struct OtxPoolService {
    runtime_handle: Handle,
    runtime: Runtime,
    network_config: NetworkConfig,
    bind_addr: SocketAddr,
    notify_ctrl: NotifyController,
    otx_pool: Arc<OtxPool>,
    host_service_provider: HostServiceProvider,
    plugin_manager: PluginManager,

    interval_handler: Option<JoinHandle<()>>,
    io_handler: Option<IoHandler>,
    rpc_server: Option<Server>,
}

impl OtxPoolService {
    pub fn new(network_config: NetworkConfig) -> Result<Self> {
        // runtime handle
        let (runtime_handle, runtime) = new_global_runtime();

        // bind address
        let bind: Vec<&str> = network_config.get_listen_uri().split("//").collect();
        let bind_addr: SocketAddr = bind[1].parse()?;

        // init notify service
        let notify_service = NotifyService::new();
        let notify_ctrl = notify_service.start(runtime_handle.clone());

        // otx pool
        let otx_pool = Arc::new(OtxPool::new(notify_ctrl.clone()));

        // init host service
        let _service_provider = HostServiceProvider::start(notify_ctrl.clone(), otx_pool.clone())
            .map_err(|err| anyhow!(err))?;

        // create plugin manager
        let plugin_manager =
            PluginManager::new(Path::new(PLUGIN_ROOT), _service_provider.handler());

        let io_handler = Some(IoHandler::new());

        Ok(OtxPoolService {
            runtime_handle,
            runtime,
            network_config,
            bind_addr,
            notify_ctrl,
            otx_pool,
            host_service_provider: _service_provider,
            plugin_manager,
            interval_handler: None,
            rpc_server: None,
            io_handler,
        })
    }

    pub fn add_plugin(&mut self, plugin: Box<Arc<dyn Plugin + Send>>) {
        self.plugin_manager.register_built_in_plugins(plugin)
    }

    pub fn extended_rpc_with<T: Send + Sync>(&mut self, delegate: IoDelegate<T>) {
        self.io_handler
            .as_mut()
            .expect("extended_rpc before start")
            .extend_with(delegate);
    }

    pub fn load_third_party_plugins(&mut self) -> Result<()> {
        self.plugin_manager
            .load_third_party_plugins(&self.runtime_handle, &self.host_service_provider)
            .map_err(|e| anyhow!(e))
    }

    pub fn get_host_service_handler(&self) -> HostServiceHandler {
        self.host_service_provider.handler()
    }

    pub fn get_plugin_configs(
        &self,
    ) -> &std::collections::HashMap<String, (PluginMeta, PluginInfo)> {
        self.plugin_manager.plugin_configs()
    }

    pub fn start(&mut self) {
        // start interval loop
        let notifier = self.notify_ctrl.clone();
        self.interval_handler = Some(self.runtime_handle.spawn(async move {
            let mut now = Instant::now().elapsed().as_secs();
            let mut interval = time::interval(INTERVAL);
            loop {
                now += INTERVAL.as_secs();
                interval.tick().await;
                notifier.notify_interval(now);
            }
        }));

        // subscribe events
        self.plugin_manager
            .subscribe_events(&self.notify_ctrl, &self.runtime_handle);

        // init otx pool rpc
        let rpc_impl = OtxPoolRpcImpl::new(self.otx_pool.clone());
        let mut io_handler = self.io_handler.take().expect("io_handler");
        io_handler.extend_with(rpc_impl.to_delegate());

        // start rpc server
        let server = ServerBuilder::new(io_handler)
            .cors(DomainsValidation::AllowOnly(vec![
                AccessControlAllowOrigin::Null,
                AccessControlAllowOrigin::Any,
            ]))
            .health_api(("/ping", "ping"))
            .start_http(&self.bind_addr)
            .expect("Start Jsonrpc HTTP service");
        self.rpc_server = Some(server);
        log::info!(
            "jsonrpc server started: {}",
            self.network_config.get_listen_uri()
        );
    }

    pub fn stop(self) {
        if let Some(interval_handler) = self.interval_handler {
            interval_handler.abort();
        }
        if let Some(rpc_server) = self.rpc_server {
            rpc_server.close();
        }
        self.runtime.shutdown_timeout(RUNTIME_SHUTDOWN_TIMEOUT);
    }
}
