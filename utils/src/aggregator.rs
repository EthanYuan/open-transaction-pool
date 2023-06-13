use super::build_tx::TxBuilder;

use config::{CkbConfig, ScriptConfig};

use anyhow::{anyhow, Result};
use ckb_jsonrpc_types as json_types;
use ckb_jsonrpc_types::TransactionView;
use ckb_sdk::{
    constants::SIGHASH_TYPE_HASH, rpc::CkbRpcClient, traits::DefaultTransactionDependencyProvider,
    Address,
};
use ckb_sdk::{
    traits::SecpCkbRawKeySigner, tx_builder::unlock_tx, unlock::ScriptUnlocker,
    unlock::SecpSighashUnlocker, HumanCapacity, ScriptGroup, ScriptId,
};
use ckb_types::{
    core::TransactionView as CoreTransactionView,
    packed::{Script, Transaction},
    H256,
};
use faster_hex::hex_decode;
use json_types::OutPoint;
use otx_format::jsonrpc_types::tx_view::tx_view_to_otx;
use otx_format::jsonrpc_types::OpenTransaction;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub struct OtxAggregator {
    pub committer: Committer,
    ckb_config: CkbConfig,
    script_config: ScriptConfig,
    tx_builder: TxBuilder,
}

impl OtxAggregator {
    pub fn new(ckb_config: CkbConfig, script_config: ScriptConfig) -> Self {
        let committer = Committer::new(ckb_config.get_ckb_uri());
        let tx_builder = TxBuilder::new(ckb_config.clone(), script_config.clone());
        OtxAggregator {
            committer,
            ckb_config,
            script_config,
            tx_builder,
        }
    }

    pub fn add_input_and_output(
        &self,
        open_tx: OpenTransaction,
        input: OutPoint,
        output_address: &Address,
        output_amout: OutputAmount,
        udt_issuer_script: Script,
        fee: u64,
    ) -> Result<OpenTransaction> {
        let aggregate_count = open_tx
            .get_aggregate_count()
            .map_err(|err| anyhow!(err.to_string()))?;
        let ckb_tx = open_tx
            .try_into()
            .map_err(|_| anyhow!("open tx convert to ckb tx"))?;
        let tx_info = self.tx_builder.add_input(
            ckb_tx,
            input.tx_hash,
            std::convert::Into::<u32>::into(input.index) as usize,
        )?;
        let ckb_tx = self.tx_builder.add_output(
            tx_info,
            output_address,
            output_amout.capacity,
            output_amout.udt_amount,
            udt_issuer_script,
        )?;

        tx_view_to_otx(
            ckb_tx,
            fee,
            aggregate_count,
            self.ckb_config.clone(),
            self.script_config.to_owned(),
        )
        .map_err(|err| anyhow!(err.to_string()))
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

#[derive(Clone)]
pub struct SignInfo {
    secp_address: Address,
    pk: H256,
    ckb_config: CkbConfig,
}

impl SignInfo {
    pub fn new(secp_address: &Address, pk: &H256, ckb_config: CkbConfig) -> Self {
        SignInfo {
            secp_address: secp_address.clone(),
            pk: pk.clone(),
            ckb_config,
        }
    }

    pub fn try_new(secp_address: Address, pk_file: PathBuf, ckb_config: CkbConfig) -> Result<Self> {
        let pk = parse_key(pk_file)?;
        Ok(SignInfo {
            secp_address,
            pk,
            ckb_config,
        })
    }

    pub fn secp_address(&self) -> &Address {
        &self.secp_address
    }

    pub fn privkey(&self) -> &H256 {
        &self.pk
    }

    pub fn sign_ckb_tx(&self, tx_view: TransactionView) -> Result<json_types::TransactionView> {
        let tx = Transaction::from(tx_view.inner).into_view();
        let (tx, _) = self.sighash_sign(&[self.pk.clone()], tx)?;
        Ok(json_types::TransactionView::from(tx))
    }

    pub fn sighash_sign(
        &self,
        keys: &[H256],
        tx: CoreTransactionView,
    ) -> Result<(CoreTransactionView, Vec<ScriptGroup>)> {
        if keys.is_empty() {
            return Err(anyhow!("must provide sender-key to sign"));
        }
        let secret_keys = keys
            .iter()
            .map(|key| secp256k1::SecretKey::from_slice(key.as_bytes()))
            .collect::<Result<Vec<secp256k1::SecretKey>, _>>()?;

        // Build ScriptUnlocker
        let signer = SecpCkbRawKeySigner::new_with_secret_keys(secret_keys);
        let sighash_unlocker = SecpSighashUnlocker::from(Box::new(signer) as Box<_>);
        let sighash_script_id = ScriptId::new_type(SIGHASH_TYPE_HASH.clone());
        let mut unlockers = HashMap::default();
        unlockers.insert(
            sighash_script_id,
            Box::new(sighash_unlocker) as Box<dyn ScriptUnlocker>,
        );

        let tx_dep_provider =
            DefaultTransactionDependencyProvider::new(self.ckb_config.get_ckb_uri(), 10);
        let (new_tx, new_still_locked_groups) = unlock_tx(tx, &tx_dep_provider, &unlockers)?;
        Ok((new_tx, new_still_locked_groups))
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

pub struct OutputAmount {
    pub capacity: HumanCapacity,
    pub udt_amount: Option<u128>,
}
