#![allow(dead_code)]

pub mod types;
pub mod uints;

use types::{
    AdjustAccountPayload, BlockInfo, DaoClaimPayload, DaoDepositPayload, DaoWithdrawPayload,
    GetAccountInfoPayload, GetAccountInfoResponse, GetBalancePayload, GetBalanceResponse,
    GetBlockInfoPayload, GetTransactionInfoResponse, MercuryInfo, PaginationResponse,
    QueryTransactionsPayload, SimpleTransferPayload, SudtIssuePayload, SyncState,
    TransactionCompletionResponse, TransferPayload, TxView,
};

use crate::const_definition::RPC_TRY_INTERVAL_SECS;
use crate::utils::client::{request, RpcClient};

use anyhow::Result;
use ckb_types::H256;

use std::thread::sleep;
use std::time::Duration;

pub struct MercuryRpcClient {
    client: RpcClient,
}

impl MercuryRpcClient {
    pub fn new(uri: String) -> Self {
        let client = RpcClient::new(uri);
        MercuryRpcClient { client }
    }

    pub fn get_balance(&self, payload: GetBalancePayload) -> Result<GetBalanceResponse> {
        request(&self.client, "get_balance", vec![payload])
    }

    pub fn get_mercury_info(&self) -> Result<MercuryInfo> {
        request(&self.client, "get_mercury_info", ())
    }

    pub fn get_sync_state(&self) -> Result<SyncState> {
        request(&self.client, "get_sync_state", ())
    }

    pub fn get_block_info(&self, block_hash: H256) -> Result<BlockInfo> {
        let payload = GetBlockInfoPayload {
            block_hash: Some(block_hash),
            block_number: None,
        };
        request(&self.client, "get_block_info", vec![payload])
    }

    pub fn query_transactions(
        &self,
        payload: QueryTransactionsPayload,
    ) -> Result<PaginationResponse<TxView>> {
        request(&self.client, "query_transactions", vec![payload])
    }

    pub fn get_transaction_info(&self, tx_hash: H256) -> Result<GetTransactionInfoResponse> {
        request(&self.client, "get_transaction_info", vec![tx_hash])
    }

    pub fn get_account_info(
        &self,
        payload: GetAccountInfoPayload,
    ) -> Result<GetAccountInfoResponse> {
        request(&self.client, "get_account_info", vec![payload])
    }

    pub fn build_transfer_transaction(
        &self,
        payload: TransferPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(&self.client, "build_transfer_transaction", vec![payload])
    }

    pub fn build_sudt_issue_transaction(
        &self,
        payload: SudtIssuePayload,
    ) -> Result<TransactionCompletionResponse> {
        request(&self.client, "build_sudt_issue_transaction", vec![payload])
    }

    pub fn build_adjust_account_transaction(
        &self,
        payload: AdjustAccountPayload,
    ) -> Result<Option<TransactionCompletionResponse>> {
        request(
            &self.client,
            "build_adjust_account_transaction",
            vec![payload],
        )
    }

    pub fn build_simple_transfer_transaction(
        &self,
        payload: SimpleTransferPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(
            &self.client,
            "build_simple_transfer_transaction",
            vec![payload],
        )
    }

    pub fn build_dao_deposit_transaction(
        &self,
        payload: DaoDepositPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(&self.client, "build_dao_deposit_transaction", vec![payload])
    }

    pub fn build_dao_withdraw_transaction(
        &self,
        payload: DaoWithdrawPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(
            &self.client,
            "build_dao_withdraw_transaction",
            vec![payload],
        )
    }

    pub fn build_dao_claim_transaction(
        &self,
        payload: DaoClaimPayload,
    ) -> Result<TransactionCompletionResponse> {
        request(&self.client, "build_dao_claim_transaction", vec![payload])
    }

    pub fn wait_block(&self, block_hash: H256) {
        while self.get_block_info(block_hash.clone()).is_err() {
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
    }

    pub fn wait_sync(&self) {
        loop {
            let sync_state = if let Ok(sync_state) = self.get_sync_state() {
                sync_state
            } else {
                continue;
            };
            if let SyncState::Serial(progress) = sync_state {
                log::info!("Mercury {:?}", progress);
                if progress.current == progress.target {
                    break;
                }
            }
            sleep(Duration::from_secs(RPC_TRY_INTERVAL_SECS))
        }
    }
}
