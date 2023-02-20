use super::build_tx::{add_input, add_output, sighash_sign};
use super::const_definition::{CKB_URI, OMNI_OPENTX_TX_HASH, OMNI_OPENTX_TX_IDX};
use super::lock::omni::{build_cell_dep, TxInfo};

use anyhow::{anyhow, Result};
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    rpc::CkbRpcClient, traits::DefaultTransactionDependencyProvider,
    unlock::opentx::assembler::assemble_new_tx, unlock::OmniUnlockMode, Address, HumanCapacity,
};
use ckb_types::{
    packed::{Transaction, WitnessArgs},
    prelude::*,
    H256,
};
use faster_hex::hex_decode;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub struct OtxAggregator {
    pub signer: SecpSigner,
    pub committer: Committer,
}

impl OtxAggregator {
    pub fn new(address: Address, key_path: PathBuf, ckb_uri: &str) -> Self {
        let signer = SecpSigner::init_account(address, key_path);
        let committer = Committer::new(ckb_uri);
        OtxAggregator { signer, committer }
    }

    pub fn add_input_and_output(
        &self,
        open_tx: TxInfo,
        input: AddInputArgs,
        output: AddOutputArgs,
    ) -> Result<TxInfo> {
        let tx_info = add_input(open_tx, input.tx_hash, input.index)?;
        add_output(
            tx_info,
            self.signer.get_secp_address(),
            output.capacity,
            output.udt_amount,
        )
    }

    pub fn merge_otxs(otx_list: Vec<TxInfo>) -> Result<TxInfo> {
        let mut txes = vec![];
        let mut omnilock_config = None;
        for tx_info in otx_list {
            // println!("> tx: {}", serde_json::to_string_pretty(&tx_info.tx)?);
            let tx = Transaction::from(tx_info.tx.inner.clone()).into_view();
            txes.push(tx);
            omnilock_config = Some(tx_info.omnilock_config.clone());
        }
        if !txes.is_empty() {
            let mut ckb_client = CkbRpcClient::new(CKB_URI);
            let cell = build_cell_dep(&mut ckb_client, &OMNI_OPENTX_TX_HASH, OMNI_OPENTX_TX_IDX)?;
            let tx_dep_provider = DefaultTransactionDependencyProvider::new(CKB_URI, 10);
            let tx = assemble_new_tx(txes, &tx_dep_provider, cell.type_hash.pack())?;
            let tx_info = TxInfo {
                tx: json_types::TransactionView::from(tx),
                omnilock_config: omnilock_config.unwrap(),
            };
            return Ok(tx_info);
        }
        Err(anyhow!("merge otxs failed!"))
    }
}

pub struct Committer {
    ckb_uri: String,
}

impl Committer {
    pub fn new(ckb_uri: &str) -> Self {
        Committer {
            ckb_uri: ckb_uri.to_string(),
        }
    }

    pub fn send_tx(&self, tx: json_types::TransactionView) -> Result<H256> {
        let outputs_validator = Some(json_types::OutputsValidator::Passthrough);
        CkbRpcClient::new(&self.ckb_uri)
            .send_transaction(tx.inner, outputs_validator)
            .map_err(|e| anyhow!(e.to_string()))
    }
}

pub struct SecpSigner {
    secp_address: Address,
    pk_path: PathBuf,
}

impl SecpSigner {
    pub fn init_account(secp_address: Address, pk: PathBuf) -> Self {
        SecpSigner {
            secp_address,
            pk_path: pk,
        }
    }

    pub fn get_secp_address(&self) -> &Address {
        &self.secp_address
    }

    pub fn sign_tx(&self, tx_info: TxInfo) -> Result<json_types::TransactionView> {
        let tx = Transaction::from(tx_info.tx.inner).into_view();
        let key_path = parse_key(self.pk_path.clone())?;
        let (tx, _) = sighash_sign(&[key_path], tx)?;
        let witness_args =
            WitnessArgs::from_slice(tx.witnesses().get(0).unwrap().raw_data().as_ref())?;
        let lock_field = witness_args.lock().to_opt().unwrap().raw_data();
        if lock_field != tx_info.omnilock_config.zero_lock(OmniUnlockMode::Normal)? {
            println!("> transaction ready to send!");
        } else {
            println!("failed to sign tx");
        }
        Ok(json_types::TransactionView::from(tx))
    }
}

fn parse_key(key_path: PathBuf) -> Result<H256> {
    let mut content = String::new();
    let mut file = File::open(key_path)?;
    file.read_to_string(&mut content)?;
    let privkey_string: String = content
        .split_whitespace()
        .next()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("File is empty"))?;

    let bytes = decode_hex(&privkey_string)?;
    H256::from_slice(&bytes).map_err(Into::into)
}

fn decode_hex(mut input: &str) -> Result<Vec<u8>> {
    if input.starts_with("0x") || input.starts_with("0X") {
        input = &input[2..];
    }
    if input.len() % 2 != 0 {
        return Err(anyhow!("Invalid hex string lenth: {}", input.len()));
    }
    let mut bytes = vec![0u8; input.len() / 2];
    hex_decode(input.as_bytes(), &mut bytes)?;
    Ok(bytes)
}

pub struct AddInputArgs {
    /// omnilock script deploy transaction hash
    pub tx_hash: H256,

    /// cell index of omnilock script deploy transaction's outputs
    pub index: usize,
}

pub struct AddOutputArgs {
    /// The capacity to transfer (unit: CKB, example: 102.43)
    pub capacity: HumanCapacity,
    pub udt_amount: Option<u128>,
}
