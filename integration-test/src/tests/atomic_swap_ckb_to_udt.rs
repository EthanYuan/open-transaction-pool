#![allow(clippy::too_many_arguments)]
#![allow(unused_doc_comments)]

use crate::const_definition::{
    CKB_URI, MERCURY_URI, OTX_POOL_URI, SCRIPT_CONFIG, UDT_1_HASH, UDT_1_HOLDER_SECP_ADDRESS,
    UDT_2_HASH, UDT_2_HOLDER_SECP_ADDRESS,
};
use crate::help::start_otx_pool;
use crate::utils::client::ckb_cli_client::ckb_cli_transfer_ckb;
use crate::utils::client::mercury_client::MercuryRpcClient;
use crate::utils::instruction::ckb::aggregate_transactions_into_blocks;
use crate::utils::instruction::ckb::dump_data;
use crate::utils::instruction::mercury::{prepare_ckb_capacity, prepare_udt_1, prepare_udt_2};
use crate::utils::lock::secp::generate_rand_secp_address_pk_pair;
use crate::IntegrationTest;

use client::OtxPoolRpcClient;
use config::{CkbConfig, ScriptInfo};
use otx_format::jsonrpc_types::OpenTransaction;
use otx_format::types::OpenTxStatus;
use otx_sdk::address::build_otx_address_from_secp_address;
use otx_sdk::build_tx::OtxBuilder;
use otx_sdk::signer::{SighashMode, Signer};

use core_rpc_types::{AssetInfo, GetBalancePayload, JsonItem};

use anyhow::{Ok, Result};
use ckb_sdk::Address;
use ckb_types::prelude::Entity;
use ckb_types::{
    bytes::Bytes,
    core::{capacity_bytes, Capacity, ScriptHashType},
    packed::{Byte32, CellOutput, OutPoint, Script},
    prelude::*,
    H256,
};

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

inventory::submit!(IntegrationTest {
    name: "test_otx_swap_ckb_to_udt",
    test_fn: test_otx_swap_ckb_to_udt
});
fn test_otx_swap_ckb_to_udt() {}
