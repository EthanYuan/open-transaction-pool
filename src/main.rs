use otx_pool::{logo::print_logo, OtxPoolService};
use otx_pool_config::{parse, AppConfig, ConfigFile};
use otx_pool_plugin_atomic_swap::{rpc::AtomicSwapRpc, AtomicSwap};
use otx_pool_plugin_dust_collector::DustCollector;
use otx_pool_plugin_signer::Signer;

use anyhow::{anyhow, Result};
use clap::Parser;

use std::sync::Arc;

pub const PLUGINS_DIRNAME: &str = "plugins";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, env = "OTXP_CONFIG_PATH")]
    config_path: String,
}

fn read_cli_args() -> Result<AppConfig> {
    let args = Args::parse();
    let config: ConfigFile = parse(args.config_path)?;
    Ok(config.into())
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

    let mut otx_pool_service = OtxPoolService::new(config.get_network_config())?;

    // add plugin AtomicUdtSwap
    if config.get_atomic_swap_config().is_enabled() {
        let atomic_swap = Arc::new(
            AtomicSwap::new(
                otx_pool_service.get_host_service_handler(),
                config.get_ckb_config(),
                config.get_script_config(),
            )
            .map_err(|err| anyhow!(err))?,
        );
        otx_pool_service.extended_rpc_with(AtomicSwapRpc::to_delegate(atomic_swap.clone()));
        otx_pool_service.add_plugin(Box::new(atomic_swap));
    }

    // add plugin DustCollector
    if config.get_dust_collector_config().is_enabled() {
        let dust_collector = Arc::new(
            DustCollector::new(
                otx_pool_service.get_host_service_handler(),
                config.get_dust_collector_config(),
                config.get_ckb_config(),
                config.get_script_config(),
            )
            .map_err(|err| anyhow!(err))?,
        );
        otx_pool_service.add_plugin(Box::new(dust_collector));
    }

    // add plugin Signer
    if config.get_signer_config().is_enabled() {
        let signer = Arc::new(
            Signer::new(
                otx_pool_service.get_host_service_handler(),
                config.get_signer_config(),
                config.get_ckb_config(),
                config.get_script_config(),
            )
            .map_err(|err| anyhow!(err))?,
        );
        otx_pool_service.add_plugin(Box::new(signer));
    }

    // start otx pool service
    otx_pool_service.start();

    // display all names of plugins
    let plugins = otx_pool_service.get_plugin_configs();
    log::info!("actived plugins count: {:?}", plugins.len());
    plugins
        .iter()
        .for_each(|(_, plugin)| log::info!("plugin name: {:?}", plugin.1.name));

    print_logo();

    // stop
    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || tx.send(()).unwrap()).unwrap();
    log::info!("Waiting for Ctrl-C...");
    rx.recv().expect("Receive Ctrl-C from channel.");

    otx_pool_service.stop();

    log::info!("Closing!");

    Ok(())
}
