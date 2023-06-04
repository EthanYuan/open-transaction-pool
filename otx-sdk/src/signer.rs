use config::CkbConfig;
use config::ScriptConfig;
use otx_format::jsonrpc_types::{OpenTransaction, OtxBuilder};

use anyhow::{anyhow, Result};
use ckb_crypto::secp::Privkey;
use ckb_hash::new_blake2b;
use ckb_types::{bytes::Bytes, core::TransactionView, packed::*, prelude::*, H256};

pub const SIGNATURE_SIZE: usize = 65;
pub const MAGIC_CODE: &str = "COTX";

#[derive(PartialEq)]
pub enum SighashMode {
    All = 0x01,
    None = 0x02,
    Single = 0x03,
    AllAnyoneCanPay = 0x81,
    NoneAnyoneCanPay = 0x82,
    SingleAnyoneCanPay = 0x83,
}

pub struct Signer {
    privkey: Privkey,
    script_config: ScriptConfig,
    ckb_config: CkbConfig,
}

impl Signer {
    pub fn new(pk: H256, script_config: ScriptConfig, ckb_config: CkbConfig) -> Self {
        let privkey = Privkey::from_slice(pk.as_bytes());
        Signer {
            privkey,
            script_config,
            ckb_config,
        }
    }

    // This function provides compatibility with the secp256k1_blake2b_sighash_all signature scheme.
    pub fn sign_secp256k1_blake2b_sighash_all(
        &self,
        _otx: OpenTransaction,
    ) -> Result<OpenTransaction> {
        unimplemented!()
    }

    pub fn partial_sign(
        &self,
        otx: OpenTransaction,
        mode: SighashMode,
        indexs: Vec<usize>,
    ) -> Result<OpenTransaction> {
        match mode {
            SighashMode::AllAnyoneCanPay => self.sign_otx_all_anyone_can_pay(otx, indexs),
            SighashMode::SingleAnyoneCanPay => self.sign_otx_single_anyone_can_pay(otx, indexs),
            _ => unimplemented!(),
        }
    }

    fn sign_otx_all_anyone_can_pay(
        &self,
        _otx: OpenTransaction,
        _indexs: Vec<usize>,
    ) -> Result<OpenTransaction> {
        unimplemented!()
    }

    fn sign_otx_single_anyone_can_pay(
        &self,
        otx: OpenTransaction,
        mut indexs: Vec<usize>,
    ) -> Result<OpenTransaction> {
        let aggregate_count = otx.get_aggregate_count().unwrap_or(1);
        let mut tx: TransactionView = otx
            .try_into()
            .map_err(|_| anyhow!("otx convert to ckb tx"))?;

        indexs.sort_unstable();
        for index in indexs {
            tx = self.sign_tx_single_anyone_can_pay(tx, index)?;
        }
        let otx_builder = OtxBuilder::new(self.script_config.to_owned(), self.ckb_config.clone());
        let otx = otx_builder
            .tx_view_to_otx(tx.into(), aggregate_count)
            .map_err(|err| anyhow!(err.to_string()))?;
        Ok(otx)
    }

    fn sign_tx_single_anyone_can_pay(
        &self,
        tx: TransactionView,
        index: usize,
    ) -> Result<TransactionView> {
        // input
        let input = tx
            .inputs()
            .get(index)
            .ok_or_else(|| anyhow!("input index out of range"))?;
        let input_len = input.as_slice().len() as u64;

        // output
        let output = tx
            .outputs()
            .get(index)
            .ok_or_else(|| anyhow!("output index out of range"))?;
        let output_len = output.as_slice().len() as u64;

        // output data
        let output_data = tx.outputs_data().get(index).unwrap();
        let output_data_len = output_data.as_slice().len() as u64;

        // witness
        let witness = WitnessArgs::default();
        let zero_lock: Bytes = {
            let mut buf = Vec::new();
            buf.resize(1 + SIGNATURE_SIZE, 0);
            buf.into()
        };
        let witness_for_digest = witness
            .clone()
            .as_builder()
            .lock(Some(zero_lock).pack())
            .build();
        let witness_len = witness_for_digest.as_bytes().len() as u64;

        // hash
        let mut message = [0u8; 32];
        let mut blake2b = new_blake2b();
        blake2b.update(&input_len.to_le_bytes());
        blake2b.update(input.as_slice());
        blake2b.update(&output_len.to_le_bytes());
        blake2b.update(output.as_slice());
        blake2b.update(&output_data_len.to_le_bytes());
        blake2b.update(output_data.as_slice());
        blake2b.update(&witness_len.to_le_bytes());
        blake2b.update(&witness_for_digest.as_bytes());
        blake2b.finalize(&mut message);

        // add prefix
        add_prefix(SighashMode::SingleAnyoneCanPay as u8, &mut message);

        // sign
        let message = H256::from(message);
        let sig = self.privkey.sign_recoverable(&message)?;

        // witness
        let mut witness_lock = vec![SighashMode::SingleAnyoneCanPay as u8];
        witness_lock.extend_from_slice(&sig.serialize());
        let witness = witness
            .as_builder()
            .lock(Some(Bytes::from(witness_lock)).pack())
            .build()
            .as_bytes()
            .pack();

        // set witness
        let tx = tx.as_advanced_builder().witness(witness).build();

        Ok(tx)
    }
}

pub(crate) fn add_prefix(sighash: u8, message: &mut [u8]) {
    let mut blake2b = new_blake2b();
    blake2b.update(MAGIC_CODE.as_bytes());
    blake2b.update(b" ");
    blake2b.update(sighash.to_string().as_bytes());
    blake2b.update(b":\n");
    blake2b.update(message.len().to_string().as_bytes());
    blake2b.update(message);
    blake2b.finalize(message);
}
