use super::const_definition::{
    CKB_URI, MERCURY_URI, OTX_POOL_URI, RPC_TRY_COUNT, RPC_TRY_INTERVAL_SECS, SCRIPT_CONFIG,
};
use crate::const_definition::CURRENT_OTX_POOL_SERVICE_PROCESS;
use crate::utils::client::ckb_client::CkbRpcClient;
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::command::run_command_spawn;
use crate::utils::instruction::{ckb::generate_blocks, ckb::unlock_frozen_capacity_in_genesis};

use otx_pool_client::OtxPoolRpcClient;
use otx_pool_config::{parse, AppConfig, ConfigFile};

use anyhow::Result;
use ckb_sdk::Address;
use ckb_types::H256;

use std::env;
use std::panic;
use std::process::Child;
use std::thread::sleep;
use std::time::Duration;

pub fn setup() -> Vec<Child> {
    println!("Setup test environment...");

    let app_config: Result<ConfigFile> = parse("dev_chain/devnet_config.toml");
    let app_config: AppConfig = if let Ok(app_config) = app_config {
        app_config.into()
    } else {
        panic!("load code hash failed");
    };
    SCRIPT_CONFIG.set(app_config.get_script_config()).unwrap();

    let ckb = start_ckb_node();
    let (ckb, mercury) = start_mercury(ckb);

    vec![ckb, mercury]
}

pub fn teardown(childs: Vec<Child>) {
    if let Some(child) = CURRENT_OTX_POOL_SERVICE_PROCESS.lock().unwrap().as_mut() {
        child.kill().expect("teardown otx pool failed");
    }

    for mut child in childs {
        child.kill().expect("teardown failed");
    }
}

pub fn start_ckb_node() -> Child {
    let ckb = run_command_spawn(
        "ckb",
        vec!["run", "-C", "dev_chain/dev", "--skip-spec-check"],
    )
    .expect("start ckb dev chain");
    let ckb_client = CkbRpcClient::new(CKB_URI.to_string());
    for _try in 0..=RPC_TRY_COUNT {
        let resp = ckb_client.local_node_info();
        if resp.is_ok() {
            unlock_frozen_capacity_in_genesis().expect("unlock frozen capacity in genesis");
            return ckb;
        } else {
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
    }
    teardown(vec![ckb]);
    panic!("Setup test environment failed");
}

pub(crate) fn start_mercury(ckb: Child) -> (Child, Child) {
    let mercury = run_command_spawn(
        "cargo",
        vec![
            "run",
            "--manifest-path",
            "mercury/Cargo.toml",
            "--",
            "-c",
            "dev_chain/mercury_devnet_config.toml",
            "run",
        ],
    );
    let mercury = if let Ok(mercury) = mercury {
        mercury
    } else {
        teardown(vec![ckb]);
        panic!("start mercury");
    };
    let mercury_client = MercuryRpcClient::new(MERCURY_URI.to_string());
    for _try in 0..=RPC_TRY_COUNT {
        let resp = mercury_client.get_mercury_info();
        if resp.is_ok() {
            mercury_client.wait_sync();

            // This step is used to make mercury enter the normal serial sync loop state
            // only then can all initialization be completed
            if generate_blocks(1).is_err() {
                teardown(vec![ckb, mercury]);
                panic!("generate block when start mercury");
            }

            return (ckb, mercury);
        } else {
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
    }
    teardown(vec![ckb, mercury]);
    panic!("Setup test environment failed");
}

pub(crate) fn start_otx_pool(address: Address, pk: H256) {
    let mut lock = CURRENT_OTX_POOL_SERVICE_PROCESS.lock().unwrap();
    if let Some(child) = lock.as_mut() {
        child.kill().unwrap();
    }

    env::set_var("PRIVKEY", pk.to_string());
    env::set_var("DEFAUT_ADDRESS", address.to_string());

    let service = run_command_spawn(
        "cargo",
        vec![
            "run",
            "--manifest-path",
            "../Cargo.toml",
            "--",
            "--config-path",
            "dev_chain/devnet_config.toml",
        ],
    );
    let service = if let Ok(service) = service {
        service
    } else {
        panic!("start otx pool");
    };
    let client = OtxPoolRpcClient::new(OTX_POOL_URI.to_string());
    for _try in 0..=RPC_TRY_COUNT {
        let resp = client.query_otx_status_by_id(H256::default());
        if resp.is_ok() {
            if let Some(child) = lock.as_mut() {
                *child = service
            } else {
                *lock = Some(service);
            }
            return;
        } else {
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
    }
    teardown(vec![service]);
    panic!("start otx pool service failed");
}
