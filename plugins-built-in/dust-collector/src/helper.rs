use otx_format::jsonrpc_types::tx_view::tx_view_to_otx;
use otx_format::jsonrpc_types::OpenTransaction;
use otx_pool_config::{CkbConfig, ScriptConfig};

use anyhow::{anyhow, Result};
use ckb_jsonrpc_types as json_types;
use ckb_jsonrpc_types::{OutPoint, TransactionView};
use ckb_sdk::{
    rpc::CkbRpcClient, traits::DefaultCellDepResolver, Address, HumanCapacity, ScriptId,
};
use ckb_types::{
    bytes::Bytes,
    core::TransactionView as CoreTransactionView,
    core::{BlockView, Capacity, ScriptHashType},
    packed::{Byte32, CellDep, CellOutput, OutPoint as PackedOutPoint},
    packed::{Script, Transaction},
    prelude::*,
    H256,
};

pub struct OutputAmount {
    pub capacity: HumanCapacity,
    pub udt_amount: Option<u128>,
}

pub struct TxBuilder {
    ckb_config: CkbConfig,
    script_config: ScriptConfig,
}

impl TxBuilder {
    pub fn new(ckb_config: CkbConfig, script_config: ScriptConfig) -> Self {
        TxBuilder {
            ckb_config,
            script_config,
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
        let tx_info = self.add_input(
            ckb_tx,
            input.tx_hash,
            std::convert::Into::<u32>::into(input.index) as usize,
        )?;
        let ckb_tx = self.add_output(
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
            self.ckb_config.get_ckb_uri(),
            self.script_config.get_sudt_code_hash(),
            self.script_config.get_xudt_rce_code_hash(),
        )
        .map_err(|err| anyhow!(err.to_string()))
    }

    pub fn add_input(
        &self,
        tx_view: json_types::TransactionView,
        tx_hash: H256,
        output_index: usize,
    ) -> Result<json_types::TransactionView> {
        let tx = Transaction::from(tx_view.inner).into_view();
        let tx = self.add_live_cell(tx, tx_hash, output_index)?;
        Ok(json_types::TransactionView::from(tx))
    }

    fn add_live_cell(
        &self,
        tx: CoreTransactionView,
        tx_hash: H256,
        output_index: usize,
    ) -> Result<CoreTransactionView> {
        let mut ckb_client = CkbRpcClient::new(self.ckb_config.get_ckb_uri());
        let out_point_json = OutPoint {
            tx_hash: tx_hash.clone(),
            index: ckb_jsonrpc_types::Uint32::from(output_index as u32),
        };
        let cell_with_status = ckb_client.get_live_cell(out_point_json, false)?;
        let input_outpoint =
            PackedOutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, output_index as u32);
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
        &self,
        tx_view: TransactionView,
        payee_address: &Address,
        capacity: HumanCapacity,
        udt_amount: Option<u128>,
        udt_issuer_script: Script,
    ) -> Result<TransactionView> {
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
                    Byte32::from_slice(self.script_config.get_xudt_rce_code_hash().as_bytes())
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
            .out_point(PackedOutPoint::new(
                Byte32::from_slice(
                    self.script_config
                        .get_xdut_cell_dep()
                        .out_point
                        .tx_hash
                        .as_bytes(),
                )?,
                self.script_config
                    .get_xdut_cell_dep()
                    .out_point
                    .index
                    .into(),
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
}
