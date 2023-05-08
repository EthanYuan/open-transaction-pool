use otx_format::jsonrpc_types::OpenTransaction;

use anyhow::Result;
use ckb_jsonrpc_types::{BlockNumber, CellDep, CellOutput, OutPoint, Script, TransactionView};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum ScriptGroupType {
    Lock,
    Type,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ScriptGroup {
    pub script: Script,
    pub group_type: ScriptGroupType,
    pub input_indices: Vec<usize>,
    pub output_indices: Vec<usize>,
}

pub struct TransactionStats {
    total_cycles: u64,
    verified_groups: Vec<ScriptGroup>,
    failed_groups: Vec<ScriptGroup>,
}

impl TransactionStats {
    pub fn success_group_count(&self) -> usize {
        self.verified_groups.len()
    }

    pub fn failed_groups_count(&self) -> usize {
        self.failed_groups.len()
    }

    pub fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

    pub fn verified_groups(&self) -> &Vec<ScriptGroup> {
        &self.verified_groups
    }

    pub fn failed_groups(&self) -> &Vec<ScriptGroup> {
        &self.failed_groups
    }

    pub fn verify_input(&self, index: usize) -> bool {
        self.verified_groups
            .iter()
            .any(|group| group.input_indices.contains(&index))
    }
}

pub fn evaluate_transaction(
    tx: &OpenTransaction,
    max_cycles: Option<u64>,
) -> Result<TransactionStats> {
    todo!()
}
