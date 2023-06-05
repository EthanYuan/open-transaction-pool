use crate::const_definition::{CKB_URI, SCRIPT_CONFIG};
use crate::utils::instruction::ckb::aggregate_transactions_into_blocks;
use crate::utils::instruction::ckb::dump_data;
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;

use config::CkbConfig;
use utils::client::ckb_cli_client::{ckb_cli_get_capacity, ckb_cli_transfer_ckb};
use utils::lock::omni::{MultiSigArgs, TxInfo};
use utils::wallet::{GenOpenTxArgs, Wallet};

use anyhow::Result;
use ckb_sdk_otx::{unlock::IdentityFlag, HumanCapacity};

use std::str::FromStr;

pub fn build_pay_ckb_signed_otx(
    payer: &str,
    prepare_capacity: usize,
    remain_capacity: usize,
    open_capacity: usize,
) -> Result<TxInfo> {
    // 1. init wallet instance
    let (address, pk) = generate_rand_secp_address_pk_pair();
    let wallet = Wallet::new(
        address,
        pk,
        CkbConfig::new("ckb_dev", CKB_URI),
        SCRIPT_CONFIG.get().unwrap().clone(),
    )
    .unwrap();
    let omni_address = wallet.get_omni_otx_address()?;

    // 2. transfer capacity to omni address
    let capacity = prepare_capacity;
    log::info!("{} prepare wallet: {:?} CKB", payer, capacity);
    let _tx_hash = ckb_cli_transfer_ckb(&omni_address, capacity).unwrap();
    aggregate_transactions_into_blocks()?;

    let capacity = ckb_cli_get_capacity(&omni_address).unwrap();
    assert_eq!(prepare_capacity as f64, capacity);

    // 3. generate open transaction
    let gen_open_tx_args = GenOpenTxArgs {
        omni_identity_flag: IdentityFlag::PubkeyHash,
        multis_args: MultiSigArgs {
            require_first_n: 1,
            threshold: 1,
            sighash_address: vec![],
        },
        receiver: omni_address,
        capacity_with_open: Some((
            HumanCapacity::from_str(&remain_capacity.to_string()).unwrap(),
            HumanCapacity::from_str(&open_capacity.to_string()).unwrap(),
        )),
        udt_amount_with_open: None,
        fee_rate: 0,
    };
    let open_tx = wallet.gen_open_tx(&gen_open_tx_args).unwrap();
    let file = format!("./free-space/dust_collector_{}_otx_unsigned.json", payer);
    dump_data(&open_tx, &file).unwrap();

    // 4. sign the otx
    let open_tx = wallet.sign_open_tx(open_tx).unwrap();
    dump_data(
        &open_tx,
        &format!("./free-space/dust_collector_{}_otx_signed.json", payer),
    )
    .unwrap();

    Ok(open_tx)
}
