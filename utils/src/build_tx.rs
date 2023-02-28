use crate::const_definition::{
    CKB_URI, XUDT_CELL_DEP_TX_HASH, XUDT_CELL_DEP_TX_IDX, XUDT_CODE_HASH,
};

use anyhow::{anyhow, Result};
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    constants::SIGHASH_TYPE_HASH,
    rpc::CkbRpcClient,
    traits::{DefaultCellDepResolver, DefaultTransactionDependencyProvider, SecpCkbRawKeySigner},
    tx_builder::unlock_tx,
    unlock::ScriptUnlocker,
    unlock::SecpSighashUnlocker,
    Address, HumanCapacity, ScriptGroup, ScriptId,
};
use ckb_types::{
    bytes::Bytes,
    core::{BlockView, Capacity, ScriptHashType, TransactionView},
    packed::{Byte32, CellDep, CellOutput, OutPoint, Script, Transaction},
    prelude::*,
    H256,
};

use std::collections::HashMap;

pub fn add_input(
    tx_view: json_types::TransactionView,
    tx_hash: H256,
    output_index: usize,
) -> Result<json_types::TransactionView> {
    let tx = Transaction::from(tx_view.inner).into_view();
    let tx = add_live_cell(tx, tx_hash, output_index)?;
    Ok(json_types::TransactionView::from(tx))
}

fn add_live_cell(
    tx: TransactionView,
    tx_hash: H256,
    output_index: usize,
) -> Result<TransactionView> {
    let ckb_uri = CKB_URI.get().ok_or_else(|| anyhow!("CKB_URI is none"))?;
    let mut ckb_client = CkbRpcClient::new(ckb_uri);
    let out_point_json = ckb_jsonrpc_types::OutPoint {
        tx_hash: tx_hash.clone(),
        index: ckb_jsonrpc_types::Uint32::from(output_index as u32),
    };
    let cell_with_status = ckb_client.get_live_cell(out_point_json, false)?;
    let input_outpoint =
        OutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, output_index as u32);
    // since value should be provided in args
    let input = ckb_types::packed::CellInput::new(input_outpoint, 0);
    let cell_dep_resolver = {
        let genesis_block = ckb_client.get_block_by_number(0.into())?.unwrap();
        DefaultCellDepResolver::from_genesis(&BlockView::from(genesis_block))?
    };
    let code_hash = cell_with_status.cell.unwrap().output.lock.code_hash;
    let script_id = ScriptId::new_type(code_hash);
    let dep = cell_dep_resolver
        .get(&script_id)
        .as_ref()
        .unwrap()
        .0
        .clone();

    Ok(tx.as_advanced_builder().input(input).cell_dep(dep).build())
}

pub fn add_output(
    tx_view: ckb_jsonrpc_types::TransactionView,
    payee_address: &Address,
    capacity: HumanCapacity,
    udt_amount: Option<u128>,
    udt_issuer_script: Script,
) -> Result<json_types::TransactionView> {
    let tx = Transaction::from(tx_view.inner).into_view();
    let lock_script = Script::from(payee_address.payload());

    let mut output = CellOutput::new_builder()
        .capacity(Capacity::shannons(capacity.0).pack())
        .lock(lock_script)
        .build();
    let mut xudt_data = Bytes::default();

    if let Some(udt_amount) = udt_amount {
        let xudt_type_script = Script::new_builder()
            .code_hash(
                Byte32::from_slice(XUDT_CODE_HASH.get().expect("get xudt code hash").as_bytes())
                    .unwrap(),
            )
            .hash_type(ScriptHashType::Type.into())
            .args(udt_issuer_script.calc_script_hash().raw_data().pack())
            .build();

        output = output
            .as_builder()
            .type_(Some(xudt_type_script).pack())
            .build();
        xudt_data = Bytes::from(udt_amount.to_le_bytes().to_vec());
    }

    let xudt_cell_dep = CellDep::new_builder()
        .out_point(OutPoint::new(
            Byte32::from_slice(
                XUDT_CELL_DEP_TX_HASH
                    .get()
                    .expect("get xudt cell dep tx hash")
                    .as_bytes(),
            )?,
            XUDT_CELL_DEP_TX_IDX
                .get()
                .expect("get xudt cell dep tx id")
                .to_owned() as u32,
        ))
        .build();

    let tx = tx
        .as_advanced_builder()
        .output(output)
        .output_data(xudt_data.pack())
        .cell_dep(xudt_cell_dep)
        .build();

    Ok(json_types::TransactionView::from(tx))
}

pub fn sighash_sign(
    keys: &[H256],
    tx: TransactionView,
) -> Result<(TransactionView, Vec<ScriptGroup>)> {
    if keys.is_empty() {
        return Err(anyhow!("must provide sender-key to sign"));
    }
    let secret_key = secp256k1::SecretKey::from_slice(keys[0].as_bytes())
        .map_err(|err| anyhow!("invalid sender secret key: {}", err))?;
    // Build ScriptUnlocker
    let signer = SecpCkbRawKeySigner::new_with_secret_keys(vec![secret_key]);
    let sighash_unlocker = SecpSighashUnlocker::from(Box::new(signer) as Box<_>);
    let sighash_script_id = ScriptId::new_type(SIGHASH_TYPE_HASH.clone());
    let mut unlockers = HashMap::default();
    unlockers.insert(
        sighash_script_id,
        Box::new(sighash_unlocker) as Box<dyn ScriptUnlocker>,
    );

    // Build the transaction
    // let output = CellOutput::new_builder()
    //     .lock(Script::from(&args.receiver))
    //     .capacity(args.capacity.0.pack())
    //     .build();
    // let builder = CapacityTransferBuilder::new(vec![(output, Bytes::default())]);
    // let (tx, still_locked_groups) = builder.build_unlocked(
    //     &mut cell_collector,
    //     &cell_dep_resolver,
    //     &header_dep_resolver,
    //     &tx_dep_provider,
    //     &balancer,
    //     &unlockers,
    // )?;

    let ckb_uri = CKB_URI.get().ok_or_else(|| anyhow!("CKB_URI is none"))?;
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(ckb_uri, 10);
    let (new_tx, new_still_locked_groups) = unlock_tx(tx, &tx_dep_provider, &unlockers)?;
    Ok((new_tx, new_still_locked_groups))
}
