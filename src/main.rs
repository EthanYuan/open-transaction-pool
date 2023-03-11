use ckb_sdk::Address;
use ckb_types::H256;
use otx_pool::{
    built_in_plugin::{atomic_swap::AtomicSwap, DustCollector},
    cli::{parse, print_logo, Config},
    notify::NotifyService,
    plugin::host_service::HostServiceProvider,
    plugin::manager::PluginManager,
    rpc::{OtxPoolRpc, OtxPoolRpcImpl},
};
use utils::aggregator::SignInfo;
use utils::const_definition::{load_code_hash, CKB_URI};

use anyhow::{anyhow, Result};
use ckb_async_runtime::new_global_runtime;
use clap::Parser;
use dashmap::DashMap;
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
    #[clap(short, long)]
    config_path: String,

    #[clap(short, long)]
    address: Address,

    // Although this is a demo, the pool is a reusable component. It should not have any wallet feature for the sake of security.
    // I would suggest adding a plugin for signing. The plugin can read address and private key from environment variables.
    //
    // We may have to define extra keys to make the signing plugin work. Please go ahead and give a proposal.
    #[clap(short, long)]
    key: H256,
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

    let (config, sign_info) = read_cli_args()?;

    start(config, sign_info)
}

fn read_cli_args() -> Result<(Config, SignInfo)> {
    let args = Args::parse();
    let sign_info = SignInfo::new(&args.address, &args.key);
    let config: Config = parse(args.config_path)?;
    // Global variable is a bad smell. We can avoid it by delegate all the CKB interactions in a dedicated service.
    CKB_URI
        .set(config.network_config.ckb_uri.clone())
        .map_err(|err| anyhow!(err))?;
    load_code_hash(config.to_script_map());
    Ok((config, sign_info))
}

pub fn start(config: Config, sign_info: SignInfo) -> Result<()> {
    // runtime handle
    let (runtime_handle, runtime) = new_global_runtime();

    // bind address
    let bind: Vec<&str> = config.network_config.listen_uri.split("//").collect();
    let bind_addr: SocketAddr = bind[1].parse()?;

    // start notify service
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

    // pool data
    let raw_otxs = Arc::new(DashMap::new());
    let sent_txs = Arc::new(DashMap::new());

    // Make sure ServiceProvider start before all daemon processes
    let service_provider =
        HostServiceProvider::start(notify_ctrl.clone(), raw_otxs.clone(), sent_txs.clone())
            .map_err(|err| anyhow!(err))?;

    // init built-in plugins
    // Pool should load built-in plugins on demand from the config file.
    let dust_collector = DustCollector::new(
        service_provider.handler(),
        sign_info,
        CKB_URI.get().unwrap(),
    )
    .map_err(|err| anyhow!(err))?;
    let atomic_swap = AtomicSwap::new(service_provider.handler(), CKB_URI.get().unwrap())
        .map_err(|err| anyhow!(err))?;

    // Please design a mechanism so user is able to configure plugin options in the config file.
    // init plugins
    let plugin_manager = PluginManager::init(
        runtime_handle,
        notify_ctrl.clone(),
        service_provider,
        Path::new("./free-space"),
        vec![Box::new(dust_collector), Box::new(atomic_swap)],
    )
    .unwrap();

    // display all names of plugins
    let plugins = plugin_manager.plugin_configs();
    log::info!("actived plugins count: {:?}", plugins.len());
    plugins
        .iter()
        .for_each(|(_, plugin)| log::info!("plugin name: {:?}", plugin.1.name));

    // init otx pool rpc
    let rpc_impl = OtxPoolRpcImpl::new(raw_otxs, sent_txs, notify_ctrl);
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
        config.network_config.listen_uri
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
