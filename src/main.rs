use otx_pool::{
    built_in_plugin::{AtomicSwap, DustCollector, P2PRelayer, Signer},
    cli::print_logo,
    notify::{NotifyController, NotifyService},
    plugin::host_service::HostServiceProvider,
    plugin::manager::PluginManager,
    pool::OtxPool,
    rpc::{OtxPoolRpc, OtxPoolRpcImpl},
};
use utils::config::{parse, AppConfig, ConfigFile};

use anyhow::{anyhow, Result};
use ckb_async_runtime::{new_global_runtime, Handle};
use clap::Parser;
use jsonrpc_core::IoHandler;
use jsonrpc_http_server::ServerBuilder;
use jsonrpc_server_utils::cors::AccessControlAllowOrigin;
use jsonrpc_server_utils::hosts::DomainsValidation;
use tokio::time::{self, Duration, Instant};

use std::sync::Arc;
use std::{net::SocketAddr, path::Path};

const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const INTERVAL: Duration = Duration::from_secs(2);
pub const PLUGINS_DIRNAME: &str = "plugins";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, env = "OTXP_CONFIG_PATH")]
    config_path: String,
}

fn main() -> Result<()> {
    std::panic::set_hook(Box::new(move |info| {
        println!("panic occurred {:?}", info);
        log::error!("panic occurred {:?}", info);
        std::process::exit(-1);
    }));

    if std::env::var("RUST_LOG").is_err() {
        // should recognize RUST_LOG_STYLE environment variable
        env_logger::Builder::from_default_env()
            .filter(None, log::LevelFilter::Info)
            .init();
    } else {
        env_logger::init();
    }

    let config = read_cli_args()?;

    start(config)
}

fn read_cli_args() -> Result<AppConfig> {
    let args = Args::parse();
    let config: ConfigFile = parse(args.config_path)?;
    Ok(config.into())
}

pub fn start(config: AppConfig) -> Result<()> {
    // runtime handle
    let (runtime_handle, runtime) = new_global_runtime();

    // bind address
    let network_config = config.get_network_config();
    let bind: Vec<&str> = network_config.get_listen_uri().split("//").collect();
    let bind_addr: SocketAddr = bind[1].parse()?;

    // init notify service
    let notify_service = NotifyService::new();
    let notify_ctrl = notify_service.start(runtime_handle.clone());

    // interval loop
    let notifier = notify_ctrl.clone();
    let interval_handler = runtime_handle.spawn(async move {
        let mut now = Instant::now().elapsed().as_secs();
        let mut interval = time::interval(INTERVAL);
        loop {
            now += INTERVAL.as_secs();
            interval.tick().await;
            notifier.notify_interval(now);
        }
    });

    // otx pool
    let otx_pool = Arc::new(OtxPool::new(notify_ctrl.clone()));

    // init host service
    let service_provider = HostServiceProvider::start(notify_ctrl.clone(), otx_pool.clone())
        .map_err(|err| anyhow!(err))?;

    // init plugins
    let plugin_manager = init_plugins(&service_provider, &config, &runtime_handle, &notify_ctrl)?;

    // display all names of plugins
    let plugins = plugin_manager.plugin_configs();
    log::info!("actived plugins count: {:?}", plugins.len());
    plugins
        .iter()
        .for_each(|(_, plugin)| log::info!("plugin name: {:?}", plugin.1.name));

    // init otx pool rpc
    let rpc_impl = OtxPoolRpcImpl::new(otx_pool);
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
    log::info!(
        "jsonrpc server started: {}",
        config.get_network_config().get_listen_uri()
    );

    print_logo();

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

fn init_plugins(
    service_provider: &HostServiceProvider,
    config: &AppConfig,
    runtime_handle: &Handle,
    notify_ctrl: &NotifyController,
) -> Result<PluginManager> {
    // create plugin manager
    let mut plugin_manager =
        PluginManager::new(Path::new("./free-space"), service_provider.handler());

    // init built-in plugins
    if config.get_dust_collector_config().is_enabled() {
        let dust_collector = DustCollector::new(
            service_provider.handler(),
            config.get_dust_collector_config(),
            config.get_ckb_config(),
            config.get_script_config(),
        )
        .map_err(|err| anyhow!(err))?;
        plugin_manager.register_built_in_plugins(Box::new(dust_collector));
    }

    // init built-in plugins
    if config.get_atomic_swap_config().is_enabled() {
        let atomic_swap = AtomicSwap::new(
            service_provider.handler(),
            config.get_ckb_config(),
            config.get_script_config(),
        )
        .map_err(|err| anyhow!(err))?;
        plugin_manager.register_built_in_plugins(Box::new(atomic_swap));
    }

    // init built-in plugins
    if config.get_signer_config().is_enabled() {
        let signer = Signer::new(
            service_provider.handler(),
            config.get_signer_config(),
            config.get_ckb_config(),
            config.get_script_config(),
        )
        .map_err(|err| anyhow!(err))?;
        plugin_manager.register_built_in_plugins(Box::new(signer));
    }

    // init built-in plugins
    if config.get_p2p_relayer_config().is_enabled() {
        let p2p_relayer = P2PRelayer::new(
            runtime_handle,
            service_provider.handler(),
            config.get_p2p_relayer_config(),
        )
        .map_err(|err| anyhow!(err))?;
        plugin_manager.register_built_in_plugins(Box::new(p2p_relayer));
    }

    // init third-party plugins
    plugin_manager
        .load_third_party_plugins(runtime_handle, service_provider)
        .map_err(|e| anyhow!(e))?;

    // subscribe events
    plugin_manager.subscribe_events(notify_ctrl, runtime_handle);

    Ok(plugin_manager)
}
