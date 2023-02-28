use crate::const_definition::{
    OMNI_CODE_HASH, SECP_DATA_CELL_DEP_TX_HASH, SECP_DATA_CELL_DEP_TX_IDX,
};

use super::const_definition::{
    OMNI_OPENTX_CELL_DEP_TX_HASH, OMNI_OPENTX_CELL_DEP_TX_IDX, XUDT_CELL_DEP_TX_HASH,
    XUDT_CELL_DEP_TX_IDX,
};
use super::lock::omni::{build_cell_dep, build_otx_omnilock_addr_from_secp, MultiSigArgs, TxInfo};

use anyhow::{anyhow, Result};
use ckb_crypto::secp::Pubkey;
use ckb_hash::blake2b_256;
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    constants::SIGHASH_TYPE_HASH,
    rpc::CkbRpcClient,
    traits::{
        DefaultCellCollector, DefaultCellDepResolver, DefaultHeaderDepResolver,
        DefaultTransactionDependencyProvider, SecpCkbRawKeySigner,
    },
    tx_builder::{
        balance_tx_capacity, fill_placeholder_witnesses, omni_lock::OmniLockTransferBuilder,
        unlock_tx, CapacityBalancer, TxBuilder,
    },
    unlock::{
        opentx::OpentxWitness, IdentityFlag, MultisigConfig, OmniLockConfig, OmniLockScriptSigner,
    },
    unlock::{OmniLockUnlocker, OmniUnlockMode, ScriptUnlocker},
    util::{blake160, keccak160},
    Address, HumanCapacity, ScriptGroup, ScriptId, SECP256K1,
};
use ckb_types::{
    bytes::Bytes,
    core::{BlockView, ScriptHashType, TransactionView},
    packed::{
        self, Byte32, CellDep, CellInputBuilder, CellOutput, OutPoint, Script, Transaction,
        WitnessArgs,
    },
    prelude::*,
    H160, H256,
};

use std::collections::HashMap;

pub struct GenOpenTxArgs {
    pub omni_identity_flag: IdentityFlag,

    pub multis_args: MultiSigArgs,

    /// The receiver address
    pub receiver: Address,

    /// The capacity to transfer (unit: CKB, example: 102.43)
    /// The open transaction capacity not decided to whom (unit: CKB, example: 102.43)
    pub capacity_with_open: Option<(HumanCapacity, HumanCapacity)>,

    pub udt_amount_with_open: Option<(u64, u64)>,

    pub fee_rate: u64,
}

pub struct Wallet {
    pk: H256,
    secp_address: Address,
    omni_otx_address: Address,
    ckb_uri: String,
}

impl Wallet {
    pub fn init_account(secp_address: Address, pk: H256, ckb_uri: &str) -> Self {
        let omni_otx_address = build_otx_omnilock_addr_from_secp(&secp_address, ckb_uri).unwrap();

        Wallet {
            pk,
            secp_address,
            omni_otx_address,
            ckb_uri: ckb_uri.to_owned(),
        }
    }

    pub fn get_omni_otx_address(&self) -> &Address {
        &self.omni_otx_address
    }

    pub fn gen_open_tx(&self, args: &GenOpenTxArgs) -> Result<TxInfo> {
        let (tx, omnilock_config) = self.build_open_tx(args)?;
        let tx_info = TxInfo {
            tx: json_types::TransactionView::from(tx),
            omnilock_config,
        };
        Ok(tx_info)
    }

    fn build_open_tx(&self, args: &GenOpenTxArgs) -> Result<(TransactionView, OmniLockConfig)> {
        let mut ckb_client = CkbRpcClient::new(&self.ckb_uri);
        let omni_lock_info = build_cell_dep(
            &mut ckb_client,
            OMNI_OPENTX_CELL_DEP_TX_HASH
                .get()
                .expect("get omni cell dep tx hash"),
            OMNI_OPENTX_CELL_DEP_TX_IDX
                .get()
                .expect("get omni cell dep tx id")
                .to_owned(),
        )?;

        let mut omnilock_config =
            self.generate_omni_config(args.omni_identity_flag, &args.multis_args)?;

        // Build CapacityBalancer
        let sender = Script::new_builder()
            .code_hash(omni_lock_info.type_hash.pack())
            .hash_type(ScriptHashType::Type.into())
            .args(omnilock_config.build_args().pack())
            .build();
        let placeholder_witness = omnilock_config.placeholder_witness(OmniUnlockMode::Normal)?;
        let balancer =
            CapacityBalancer::new_simple(sender.clone(), placeholder_witness, args.fee_rate);

        // Build:
        //   * CellDepResolver
        //   * HeaderDepResolver
        //   * CellCollector
        //   * TransactionDependencyProvider
        let genesis_block = ckb_client.get_block_by_number(0.into())?.unwrap();
        let genesis_block = BlockView::from(genesis_block);
        let mut cell_dep_resolver = DefaultCellDepResolver::from_genesis(&genesis_block)?;
        cell_dep_resolver.insert(
            omni_lock_info.script_id,
            omni_lock_info.cell_dep,
            "Omni Lock".to_string(),
        );
        let header_dep_resolver = DefaultHeaderDepResolver::new(&self.ckb_uri);
        let mut cell_collector = DefaultCellCollector::new(&self.ckb_uri);
        let tx_dep_provider = DefaultTransactionDependencyProvider::new(&self.ckb_uri, 10);

        // Build base transaction
        let unlockers = build_omnilock_unlockers(
            Vec::new(),
            omnilock_config.clone(),
            omni_lock_info.type_hash,
        );

        let (capacity, open_capacity) = args.capacity_with_open.unwrap();

        let output = CellOutput::new_builder()
            .lock(sender.clone())
            .capacity(capacity.0.pack())
            .build();

        let builder = OmniLockTransferBuilder::new_open(
            open_capacity,
            vec![(output, Bytes::default())],
            omnilock_config.clone(),
            None,
        );

        let base_tx = builder.build_base(
            &mut cell_collector,
            &cell_dep_resolver,
            &header_dep_resolver,
            &tx_dep_provider,
        )?;

        let secp256k1_data_dep = {
            // pub const SECP256K1_DATA_OUTPUT_LOC: (usize, usize) = (0, 3);
            let tx_hash = genesis_block.transactions()[0].hash();
            let out_point = OutPoint::new(tx_hash, 3u32);
            CellDep::new_builder().out_point(out_point).build()
        };

        let base_tx = base_tx
            .as_advanced_builder()
            .cell_dep(secp256k1_data_dep)
            .build();
        let (tx, _) = fill_placeholder_witnesses(base_tx, &tx_dep_provider, &unlockers)?;

        let tx = balance_tx_capacity(
            &tx,
            &balancer,
            &mut cell_collector,
            &tx_dep_provider,
            &cell_dep_resolver,
            &header_dep_resolver,
        )?;

        let tx = OmniLockTransferBuilder::remove_open_out(tx);
        let wit = OpentxWitness::new_sig_all_relative(&tx, Some(0xdeadbeef)).unwrap();
        omnilock_config.set_opentx_input(wit);
        let tx = OmniLockTransferBuilder::update_opentx_witness(
            tx,
            &omnilock_config,
            OmniUnlockMode::Normal,
            &tx_dep_provider,
            &sender,
        )?;
        Ok((tx, omnilock_config))
    }

    pub fn sign_open_tx(&self, tx_info: TxInfo) -> Result<TxInfo> {
        let tx = Transaction::from(tx_info.tx.inner).into_view();
        let pks = vec![&self.pk];
        let keys: Vec<secp256k1::SecretKey> = pks
            .iter()
            .map(|sender_key| {
                secp256k1::SecretKey::from_slice(sender_key.as_bytes())
                    .map_err(|err| format!("invalid sender secret key: {}", err))
                    .unwrap()
            })
            .collect();
        if tx_info.omnilock_config.is_pubkey_hash() || tx_info.omnilock_config.is_ethereum() {
            for (i, key) in keys.iter().enumerate() {
                let pubkey = secp256k1::PublicKey::from_secret_key(&SECP256K1, key);
                let hash160 = match tx_info.omnilock_config.id().flag() {
                    IdentityFlag::PubkeyHash => {
                        blake2b_256(&pubkey.serialize()[..])[0..20].to_vec()
                    }
                    IdentityFlag::Ethereum => {
                        keccak160(Pubkey::from(pubkey).as_ref()).as_bytes().to_vec()
                    }
                    _ => unreachable!(),
                };
                if tx_info.omnilock_config.id().auth_content().as_bytes() != hash160 {
                    return Err(anyhow!("key {:#x} is not in omnilock config", pks[i]));
                }
            }
        }
        let (tx, _) = self.sign_otx(tx, &tx_info.omnilock_config, keys)?;
        let witness_args =
            WitnessArgs::from_slice(tx.witnesses().get(0).unwrap().raw_data().as_ref())?;
        let lock_field = witness_args.lock().to_opt().unwrap().raw_data();
        if lock_field != tx_info.omnilock_config.zero_lock(OmniUnlockMode::Normal)? {
            log::info!("open transaction has been signed");
        } else {
            log::info!("failed to sign tx");
        }
        let tx_info = TxInfo {
            tx: json_types::TransactionView::from(tx),
            omnilock_config: tx_info.omnilock_config,
        };
        Ok(tx_info)
    }

    pub fn gen_open_tx_pay_udt(
        &self,
        inputs: Vec<OutPoint>,
        outputs: Vec<CellOutput>,
        outputs_data: Vec<packed::Bytes>,
    ) -> Result<TxInfo> {
        let secp_data_cell_dep = CellDep::new_builder()
            .out_point(OutPoint::new(
                Byte32::from_slice(
                    SECP_DATA_CELL_DEP_TX_HASH
                        .get()
                        .expect("get secp data cell dep tx hash")
                        .as_bytes(),
                )?,
                SECP_DATA_CELL_DEP_TX_IDX
                    .get()
                    .expect("get secp data cell dep tx id")
                    .to_owned() as u32,
            ))
            .build();
        let omin_cell_dep = CellDep::new_builder()
            .out_point(OutPoint::new(
                Byte32::from_slice(
                    OMNI_OPENTX_CELL_DEP_TX_HASH
                        .get()
                        .expect("get omni cell dep tx hash")
                        .as_bytes(),
                )?,
                OMNI_OPENTX_CELL_DEP_TX_IDX
                    .get()
                    .expect("get cell dep tx id")
                    .to_owned() as u32,
            ))
            .build();
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
        let cell_deps = vec![secp_data_cell_dep, omin_cell_dep, xudt_cell_dep];

        let (tx, omnilock_config) =
            self.build_open_tx_pay_udt(inputs, outputs, outputs_data, cell_deps)?;
        let tx_info = TxInfo {
            tx: json_types::TransactionView::from(tx),
            omnilock_config,
        };
        Ok(tx_info)
    }

    fn build_open_tx_pay_udt(
        &self,
        inputs: Vec<OutPoint>,
        outputs: Vec<CellOutput>,
        outputs_data: Vec<packed::Bytes>,
        cell_deps: Vec<CellDep>,
    ) -> Result<(TransactionView, OmniLockConfig)> {
        // generate omni config
        let mut omnilock_config = {
            let arg = H160::from_slice(&self.secp_address.payload().args()).unwrap();
            OmniLockConfig::new_pubkey_hash(arg)
        };
        omnilock_config.set_opentx_mode();

        // build opentx
        let tx_builder = TransactionView::new_advanced_builder();
        let inputs: Vec<packed::CellInput> = inputs
            .into_iter()
            .map(|out_point| {
                CellInputBuilder::default()
                    .previous_output(out_point)
                    .build()
            })
            .collect();

        let tx = tx_builder
            .inputs(inputs.clone())
            .outputs(outputs)
            .outputs_data(outputs_data)
            .cell_deps(cell_deps)
            .witnesses(vec![packed::Bytes::default(); inputs.len()])
            .build();
        let tx_dep_provider = DefaultTransactionDependencyProvider::new(&self.ckb_uri, 10);

        // update opentx input list
        let wit = OpentxWitness::new_sig_all_relative(&tx, Some(0xdeadbeef)).unwrap();
        omnilock_config.set_opentx_input(wit);
        let lock: Script = (&self.omni_otx_address).into();
        let tx = OmniLockTransferBuilder::update_opentx_witness(
            tx,
            &omnilock_config,
            OmniUnlockMode::Normal,
            &tx_dep_provider,
            &lock,
        )
        .unwrap();

        //sign
        let pks = vec![&self.pk];
        let keys: Vec<secp256k1::SecretKey> = pks
            .iter()
            .map(|sender_key| {
                secp256k1::SecretKey::from_slice(sender_key.as_bytes())
                    .map_err(|err| format!("invalid sender secret key: {}", err))
                    .unwrap()
            })
            .collect();

        // config updated, so unlockers must rebuilt.
        let unlockers = build_omnilock_unlockers(
            keys,
            omnilock_config.clone(),
            OMNI_CODE_HASH.get().expect("get omni code hash").to_owned(),
        );
        let (tx, _new_locked_groups) = unlock_tx(tx, &tx_dep_provider, &unlockers).unwrap();

        Ok((tx, omnilock_config))
    }

    fn generate_omni_config(
        &self,
        omni_identity_flag: IdentityFlag,
        multis_args: &MultiSigArgs,
    ) -> Result<OmniLockConfig> {
        let mut omnilock_config = match omni_identity_flag {
            IdentityFlag::PubkeyHash => {
                let sender_key = secp256k1::SecretKey::from_slice(self.pk.as_bytes())
                    .map_err(|err| anyhow!("invalid sender secret key: {}", err))?;
                let pubkey = secp256k1::PublicKey::from_secret_key(&SECP256K1, &sender_key);
                let pubkey_hash = blake160(&pubkey.serialize());
                OmniLockConfig::new_pubkey_hash(pubkey_hash)
            }
            IdentityFlag::Ethereum => {
                let sender_key = secp256k1::SecretKey::from_slice(self.pk.as_bytes())
                    .map_err(|err| anyhow!("invalid sender secret key: {}", err))?;
                let pubkey = secp256k1::PublicKey::from_secret_key(&SECP256K1, &sender_key);
                println!("pubkey:{:?}", hex_string(&pubkey.serialize()));
                println!("pubkey:{:?}", hex_string(&pubkey.serialize_uncompressed()));
                let addr = keccak160(Pubkey::from(pubkey).as_ref());
                OmniLockConfig::new_ethereum(addr)
            }
            IdentityFlag::Multisig => {
                let args = &multis_args;
                let multisig_config = build_multisig_config(
                    &args.sighash_address,
                    args.require_first_n,
                    args.threshold,
                )?;
                OmniLockConfig::new_multisig(multisig_config)
            }
            _ => {
                return Err(anyhow!(
                    "must provide a sender-key or an ethereum-sender-key"
                ));
            }
        };

        omnilock_config.set_opentx_mode();

        Ok(omnilock_config)
    }

    fn sign_otx(
        &self,
        mut tx: TransactionView,
        omnilock_config: &OmniLockConfig,
        keys: Vec<secp256k1::SecretKey>,
    ) -> Result<(TransactionView, Vec<ScriptGroup>)> {
        // Unlock transaction
        let tx_dep_provider = DefaultTransactionDependencyProvider::new(&self.ckb_uri, 10);

        let mut ckb_client = CkbRpcClient::new(&self.ckb_uri);
        let cell = build_cell_dep(
            &mut ckb_client,
            OMNI_OPENTX_CELL_DEP_TX_HASH
                .get()
                .expect("get omni cell dep tx hash"),
            OMNI_OPENTX_CELL_DEP_TX_IDX
                .get()
                .expect("get omni cell dep tx id")
                .to_owned(),
        )?;

        let mut _still_locked_groups = None;
        let unlockers = build_omnilock_unlockers(keys, omnilock_config.clone(), cell.type_hash);
        let (new_tx, new_still_locked_groups) =
            unlock_tx(tx.clone(), &tx_dep_provider, &unlockers)?;
        tx = new_tx;
        _still_locked_groups = Some(new_still_locked_groups);
        Ok((tx, _still_locked_groups.unwrap_or_default()))
    }
}

fn build_omnilock_unlockers(
    keys: Vec<secp256k1::SecretKey>,
    config: OmniLockConfig,
    omni_lock_type_hash: H256,
) -> HashMap<ScriptId, Box<dyn ScriptUnlocker>> {
    let signer = match config.id().flag() {
        IdentityFlag::PubkeyHash => SecpCkbRawKeySigner::new_with_secret_keys(keys),
        IdentityFlag::Ethereum => SecpCkbRawKeySigner::new_with_ethereum_secret_keys(keys),
        IdentityFlag::Multisig => SecpCkbRawKeySigner::new_with_secret_keys(keys),
        _ => unreachable!("should not reach here!"),
    };
    let omnilock_signer =
        OmniLockScriptSigner::new(Box::new(signer), config.clone(), OmniUnlockMode::Normal);
    let omnilock_unlocker = OmniLockUnlocker::new(omnilock_signer, config);
    let omnilock_script_id = ScriptId::new_type(omni_lock_type_hash);
    HashMap::from([(
        omnilock_script_id,
        Box::new(omnilock_unlocker) as Box<dyn ScriptUnlocker>,
    )])
}

fn build_multisig_config(
    sighash_address: &[Address],
    require_first_n: u8,
    threshold: u8,
) -> Result<MultisigConfig> {
    if sighash_address.is_empty() {
        return Err(anyhow!("Must have at least one sighash_address"));
    }
    let mut sighash_addresses = Vec::with_capacity(sighash_address.len());
    for addr in sighash_address {
        let lock_args = addr.payload().args();
        if addr.payload().code_hash(None).as_slice() != SIGHASH_TYPE_HASH.as_bytes()
            || addr.payload().hash_type() != ScriptHashType::Type
            || lock_args.len() != 20
        {
            return Err(anyhow!("sighash_address {} is not sighash address", addr));
        }
        sighash_addresses.push(H160::from_slice(lock_args.as_ref()).unwrap());
    }
    Ok(MultisigConfig::new_with(
        sighash_addresses,
        require_first_n,
        threshold,
    )?)
}
