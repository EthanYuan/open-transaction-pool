# Pattern: Atomic Swap

## 1 Pattern Explanation

Atomic Swap refers to the exchange of assets between two parties, including:

- CKB <==> UDT
- UDT <==> UDT

The initiator of an Open Transaction (OTX) locks the payment of one asset and the receipt of another asset using the `Single|AnyoneCanPay (0x83)` mode of  [otx-sighash-lock](https://github.com/EthanYuan/otx-sighash-lock). If there is another OTX that can match it exactly and the sum of their transaction fees meets the minimum requirements, the Atomic Plugin will merge them into a final CKB transaction.

## 2 OTX Overview

The test case CKB <==> UDT for this pattern have already been implemented in [Swap: CKB to UDT](../../integration-test/src/tests/swap/atomic_swap_ckb_to_udt.rs#L29):

Alice is willing to exchange 10 CKB for 10 UDT-A and pay 1 CKB as a transaction fee. Alice's OTX:

```
{
    inputs: [
        {capacity: 142+10+1, data: "", type: "", lock: Alice},
    ],
    outputs: [
        {capacity: 142, data: 10, type: udt A, lock: Alice},
    ]
}
```

Bob is willing to exchange 10 UDT-A for 10 CKB and pay 1 CKB as a transaction fee. Bob's OTX:

```
{
    inputs: [
        {capacity: 142, data: 10, type: udt A, lock: Bob},
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
        {capacity: 142, data: 10, type: udt A, lock: Bob},
    ],
    outputs: [   
        {capacity: 142, data: 10, type: udt A, lock: Alice},
        {capacity: 152-1, data: "", type: "", lock: Bob},
    ]
}
```

## 3 Workflow

### 3.1 Creation of OTX on the wallet side

The wallet party creates an atomic swap OTX where the input must be locked by the `Single|AnyoneCanPay (0x83)` mode of [otx-sighash-lock](https://github.com/EthanYuan/otx-sighash-lock). After signing the OTX, submitted it to the OTX Pool operated.

Deployment information for lock script `otx-sighash-lock` is detailed in [configs](../../src/configs/).

In this pattern, the `cell dep`, `input`, `output`, and `witness` in the OTX are all determined, so they can be converted into the corresponding [`Essential Keys`](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md#essential-keys) relatively easily. The fields in `Essential Keys` correspond one-to-one with the fields in [CKB Open Transaction](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md). 

Our focus is on the `Extra Keys` used in this pattern: `Identifying Group` and  `Accounting Key Group`. 

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

The Extra key-value pairs mentioned above, added during the creation process, provide a direct display of the input and output asset statistics for the current OTX. This eliminates the need for the receiving party to traverse all inputs and outputs to obtain the atomic swap proposal.

The OTX sdk can help to initialise an OTX through  the API: [`OtxBuilder::build_otx`](../../otx-sdk/src/build_tx.rs). This API facilitates the creation of the `Essential Keys` and `Extra Keys` mentioned above.

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

### 3.2 Pool side

The OTX pool operates as a service that provides [RPCs](../../otx-pool/src/rpc/README.md), including `submit_otx`, and initializes the Atomic Swap plugin as well as other plugins. When a newly submitted OTX is received, it is indexed by the OTX pool, and a notification is sent to all plugins.

### 3.3 Plugin：Atomic Swap

When the OTX Pool notifies the plugin of a new OTX, the plugin checks if it has an Atomic Swap requirement by examining the OTX's `Account Extra Keys`. If an Atomic Swap requirement is found, the plugin searches for matching OTXs in its internal index:

- If no matching OTX is found, the plugin adds the new OTX to its own index for future matching.
- If a matching OTX is found, the plugin combines the new OTX and the matched OTX into a final CKB transaction and submits it to the CKB node.

Once the transaction is successfully sent, the plugin removes the corresponding OTX from its own indexed list and promptly notifies the host OTX Pool. The OTX Pool then deletes its indexed OTXs and sends a notification to inform other registered plugins.

The atomic swap plugin provides access to the OTX swap proposals through the [get_all_swap_proposals](../../plugins-built-in/atomic-swap/src/rpc.rs) PRC. The swap proposal is presented in the following data structure:

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

For the example in [2 OTX Overview](#2-otx-overview), it has the following json form:

Alice's OTX swap proposal:

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

Bob's OTX swap proposal:

```json
  {
    "swap_proposal": {
      "sell_udt": {
        "code_hash": "0x9c6933d977360f115a3e9cd5a2e0e475853681b80d775d93ad0f8969da343e56",
        "hash_type": "type",
        "args": "0x019965e078505327d1d67e30cc68d48e80005394b80c91f3f8c9a72c28e53ca2"
      },
      "sell_amount": 10,
      "buy_udt": {
        "code_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
        "hash_type": "data",
        "args": "0x"
      },
      "buy_amount": 1000000000,
      "pay_fee": 100000000
    },
    "otx_id": "0xecf65486f802a6365f6b0d19af4ca3a9d6f109e0adc547798e42dfe01e334748"
  }
```

The swap proposals are generated based on the value of the `Accounting Key Group` field in the OTX Format.

It is crucial to note that an OTX must meet the following conditions in order to be indexed by the atomic swap plugin: **after deducting the maximum fee, the OTX should only contain swaps involving two types of assets**.