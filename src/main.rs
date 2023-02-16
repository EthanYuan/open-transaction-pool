use otx_pool::{
    built_in_plugin::DustCollector,
    notify::NotifyService,
    plugin::manager::PluginManager,
    rpc::{OtxPoolRpc, OtxPoolRpcImpl},
};

use anyhow::{anyhow, Result};
use ckb_async_runtime::new_global_runtime;
use jsonrpc_core::IoHandler;
use jsonrpc_http_server::ServerBuilder;
use jsonrpc_server_utils::cors::AccessControlAllowOrigin;
use jsonrpc_server_utils::hosts::DomainsValidation;
use tokio::time::{self, Duration};

use std::{net::SocketAddr, path::Path};

pub const MESSAGE_CHANNEL_SIZE: usize = 1024;
const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
pub const PLUGINS_DIRNAME: &str = "plugins";
pub const SERVICE_URI: &str = "http://127.0.0.1:8118";

fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        // should recognize RUST_LOG_STYLE environment variable
        env_logger::Builder::from_default_env()
            .filter(None, log::LevelFilter::Info)
            .init();
    } else {
        env_logger::init();
    }

    start()
}

pub fn start() -> Result<()> {
    // runtime handle
    let (runtime_handle, runtime) = new_global_runtime();

    // bind address
    let bind: Vec<&str> = SERVICE_URI.split("//").collect();
    let bind_addr: SocketAddr = bind[1].parse()?;

    // start notify service
    let notify_service = NotifyService::new();
    let notify_ctrl = notify_service.start(runtime_handle.clone());

    // interval loop
    let notifier = notify_ctrl.clone();
    let interval_handler = runtime_handle.spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            notifier.notify_interval();
        }
    });

    // init plugins
    let mut plugin_manager = PluginManager::init(
        runtime_handle.clone(),
        notify_ctrl.clone(),
        Path::new("./free-space"),
    )
    .unwrap();

    // init built-in plugins
    let dust_collector = DustCollector::new(runtime_handle, plugin_manager.service_handler())
        .map_err(|err| anyhow!(err))?;
    plugin_manager
        .add_built_in_plugin(Box::new(dust_collector))
        .map_err(|err| anyhow!(err))?;
    let plugins = plugin_manager.plugin_configs();

    // display all names of plugins
    log::info!("actived plugins count: {:?}", plugins.len());
    plugins
        .iter()
        .for_each(|(_, plugin)| log::info!("plugin name: {:?}", plugin.1.name));

    // init otx pool rpc
    let rpc_impl = OtxPoolRpcImpl::new(notify_ctrl);
    let mut io_handler = IoHandler::new();
    io_handler.extend_with(rpc_impl.to_delegate());

    // start rpc server
    let server = ServerBuilder::new(io_handler)
        .cors(DomainsValidation::AllowOnly(vec![
            AccessControlAllowOrigin::Null,
            AccessControlAllowOrigin::Any,
        ]))
        .health_api(("/ping", "ping"))
        .start_http(&bind_addr)
        .expect("Start Jsonrpc HTTP service");
    log::info!("jsonrpc server started: {}", SERVICE_URI);

    // stop
    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || tx.send(()).unwrap()).unwrap();
    log::info!("Waiting for Ctrl-C...");
    rx.recv().expect("Receive Ctrl-C from channel.");

    interval_handler.abort();
    server.close();
    runtime.shutdown_timeout(RUNTIME_SHUTDOWN_TIMEOUT);

    log::info!("Closing!");

    Ok(())
}
