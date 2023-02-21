use crate::instruction::command::run_command_output;

use anyhow::Result;
use ckb_sdk::Address;
use ckb_types::H256;

use std::str::FromStr;

pub fn ckb_cli_transfer_ckb(address: &Address, capacity: usize) -> Result<H256> {
    // ckb-cli
    // config --url http://127.0.0.1:8114"
    let (stdout, _) = run_command_output(
        "ckb-cli",
        vec![
            "wallet",
            "transfer",
            "--to-address",
            &address.to_string(),
            "--capacity",
            &capacity.to_string(),
            "--skip-check-to-address",
            "--privkey-path",
            "./devtools/dev_key",
        ],
    )
    .expect("ckb-cli transfer ckb failed");

    let tx_hash = look_after_in_line(&stdout, "0x")
        .split(' ')
        .collect::<Vec<&str>>()[0]
        .to_string();

    H256::from_str(&tx_hash).map_err(Into::into)
}

pub fn ckb_cli_get_capacity(address: &Address) -> Result<f64> {
    // ckb-cli
    // config --url http://127.0.0.1:8114"
    let (stdout, _) = run_command_output(
        "ckb-cli",
        vec!["wallet", "get-capacity", "--address", &address.to_string()],
    )
    .expect("get capacity");

    let amount = look_after_in_line(&stdout, "total:")
        .split(' ')
        .collect::<Vec<&str>>()[0]
        .parse::<f64>()
        .expect("look after in line");
    Ok(amount)
}

fn look_after_in_line(text: &str, key: &str) -> String {
    text.split(key).collect::<Vec<&str>>()[1]
        .split('\n')
        .collect::<Vec<&str>>()[0]
        .trim_matches(&['"', ' '][..])
        .to_owned()
}
