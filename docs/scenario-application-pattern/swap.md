## Pattern 1: Atomic Swap

## 1.1 Pattern Explanation

Atomic Swap refers to the exchange of assets between two parties, including:

- CKB <==> UDT
- UDT <==> UDT

The initiator of an Open Transaction (OTX) locks the payment of one asset and the receipt of another asset using [otx-sighash-lock](https://github.com/EthanYuan/otx-sighash-lock). If there is another OTX that can match it exactly and the sum of their transaction fees meets the minimum requirements, the Atomic Plugin will merge them into a final CKB transaction.

## 1.2 OTX Overview

The test case CKB <==> UDT for this pattern have already been implemented in [Swap: CKB to UDT](../../integration-test/src/tests/swap/atomic_swap_ckb_to_udt.rs#L29).

Alice is willing to exchange 10 CKB for 10 UDT-A and pay 1 CKB as a transaction fee. Alice's OTX:

```
{
    inputs: [
        {capacity: 142+10+1, data: "", type: "", lock: Alice},
    ],
    outputs: [
        {capacity: 142, data: 10, type: xudt A, lock: Alice},
    ]
}
```

Bob is willing to exchange 10 UDT-A for 10 CKB and pay 1 CKB as a transaction fee. Bob's OTX:

```
{
    inputs: [
        {capacity: 142, data: 10, type: xudt A, lock: Bob},
    ],
    outputs: [
        {capacity: 152-1, data: "", type: "", lock: Bob},
    ]
}
```

Since Alice and Bob's swap requirements just match and the sum of the fees paid by both meets the minimum requirements, the swap plugin will merge the two transactions into one final CKB transaction and send it to the CKB network.

Final Transaction:

```
{
    inputs: [
        {capacity: 142+10+1, data: "", type: "", lock: Alice},
        {capacity: 142, data: 10, type: xudt A, lock: Bob},
    ],
    outputs: [   
        {capacity: 142, data: 10, type: xudt A, lock: Alice},
        {capacity: 152-1, data: "", type: "", lock: Bob},
    ]
}
```

### 1.3 Workflow

#### 1.3.1 Creation of OTX on the wallet side

The wallet creates a swap OTX in which all inputs to be unlocked must use the [otx-sighash-lock](https://github.com/EthanYuan/otx-sighash-lock) that supports OTX. After signing the OTX, submitted it to the OTX Pool operated.

Deployment information for otx-sighash-lock is detailed in [configs](../../src/configs/).

In this pattern, the `cell dep`, `input`, `output`, and `witness` in the OTX are all determined, so they can be converted into the corresponding [`Essential Keys`](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md#essential-keys) relatively easily. The fields in `Essential Keys` correspond one-to-one with the fields in [CKB Open Transaction](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md). 

Our focus is on the `Extra Keys` used in this pattern: `Identifying Group` and  `Accouting Key Group`. 

Open Transaction hash and witness hash are meta map keys. They correspond to the transaction hash and the witness hash of the CKB transaction generated from the open transaction.

| Name | key_type | key_data | value_data |
| --- | --- | --- | --- |
| Transaction Hash | OTX_IDENTIFYING_META_TX_HASH = 0x10010 | None | Byte32
| Transaction Witness Hash | OTX_IDENTIFYING_META_TX_WITNESS_HASH = 0x10011 | None | Byte32
| Aggregate count | OTX_IDENTIFYING_META_AGGREGATE_COUNT = 0x10012 | None | Byte32

Accounting helps to settle the balance of CKB capacities and UDT tokens.

| Name | key_type | key_data | value_data |
| --- | --- | --- | --- |
| Input CKB Capacity | OTX_ACCOUNTING_META_INPUT_CKB = 0x10040 | None | Uint64, the total input CKB capacity in Shannons. |
| Output CKB Capacity | OTX_ACCOUNTING_META_OUTPUT_CKB = 0x10041 | None | Uint64, the total output CKB capacity in Shannons. |
| Max CKB Fee | OTX_ACCOUNTING_META_MAX_FEE = 0x10043 | None | Uint64,  the maximum fee in Shannons. |
| Input xUDT Amount | OTX_ACCOUNTING_META_INPUT_XUDT = 0x10044 | Script serialized via Molecule | Uint128, the total input xUDT tokens identified by the type script serialized in key_data. |
| Output xUDT Amount | OTX_ACCOUNTING_META_OUTPUT_XUDT = 0x10045 | Script serialized via Molecule | Uint128, the total output xUDT tokens identified by the type script serialized in key_data. |

The above Extra key-value pairs, added at creation time, can directly display the input and output asset statistics for the current OTX, so that the party receiving that OTX does not need to traverse all inputs and outputs for balance statistics.

The OTX sdk can help to initialise an otx, using this API: [`OtxBuilder::build_otx`](../../otx-sdk/src/build_tx.rs):

```rust
   pub fn build_otx(
        &self,
        inputs: Vec<OutPoint>,
        outputs: Vec<CellOutput>,
        outputs_data: Vec<packed::Bytes>,
        script_infos: Vec<ScriptInfo>,
        fee: u64,
    ) -> Result<OpenTransaction>;
```

#### 1.3.2 Pool side

The OTX pool starts as a service, provides [RPCs](../../otx-pool/src/rpc/README.md) such as `submit_otx`, and initializes the Atomic Swap plugin as well as other plugins. When it receives a newly submitted OTX, it indexes it and notifies the plugin.

#### 1.3.3 Plugin：Atomic Swap

When the OTX Pool notifies the plugin of a new OTX, the plugin checks if it has an Atomic Swap requirement by using the Account Extra Keys of the OTX. If so, the plugin will search for matching OTXs in its internal index:

-   If there is no matching OTX, the plugin will add the new OTX to its own OTX index for future matching.
-   If there is a matching OTX, the plugin will assemble the new OTX and the matched OTX into a final CKB transaction and submit it to the CKB node. When the transaction is sent successfully, the plugin deletes its own indexed OTX and immediately notifies the host OTX Pool, which deletes its own indexed otxs and sends a notification to inform other registered plugins.

For OTXs indexed by the atomic swap plugin, the OTX swap proposals can be accessed via the rpc [get_all_swap_proposals](../../plugins-built-in/atomic-swap/src/rpc.rs) interface provided by the plugin. The swap proposal will be presented in the following data structure:

```rust
pub struct SwapProposalWithOtxId {
    pub swap_proposal: SwapProposal,
    pub otx_id: H256,
}

pub struct SwapProposal {
    pub sell_udt: Script,
    pub sell_amount: u64,
    pub buy_udt: Script,
    pub buy_amount: u64,
    pub pay_fee: u64,
}
```

For the example in [1.2 OTX Overview](#12-otx-overview), it has the following json form:

```json
{
  "swap_proposal": {
    "sell_udt": {
      "code_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
      "hash_type": "data",
      "args": "0x"
    },
    "sell_amount": 1000000000,
    "buy_udt": {
      "code_hash": "0x9c6933d977360f115a3e9cd5a2e0e475853681b80d775d93ad0f8969da343e56",
      "hash_type": "type",
      "args": "0x926c30dcf98fecbfdffbffcd043f3ee4dbad0286e259b20f15f8be57d650b313"
    },
    "buy_amount": 10,
    "pay_fee": 100000000
  },
  "otx_id": "0x89a3e20524424a6d8bae9edc38c046fc030d942f5df7513c67079d72aacfc2b5"
}
```
The swap proposal here is calculated by relying on the value of the `Accouting Key Group` field in the OTX Format.

It is important to note that an OTX is required to meet the following conditions before it can be indexed by the atomic swap plugin: **after removing the max fee, only swaps of two asset types in the OTX**.
