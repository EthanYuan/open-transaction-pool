## Pattern 1: Atomic Swap

## 1.1 Pattern Explanation

Atomic Swap refers to the exchange of assets between two parties within the CKB, including:

- UDT <==> UDT
- CKB <==> UDT

The initiator of an Open Transaction (OTX) locks the payment of one asset and the receipt of another asset using an OTX locking script. If there is another OTX that can match it exactly, the Atomic Plugin will merge them into a final CKB transaction.

## 1.2 OTX Overview

The test case for this pattern have already been implemented in [Swap: Atomic Swap](../../integration-test/src/tests/swap/atomic_swap.rs#L29).

Alice is willing to exchange 10 UDT-A for 10 UDT-B and pay 1 CKB as a transaction fee. Alice's OTX:

```
{
	inputs: [
		{capacity: 144, data: 12, type: xudt A, lock: Alice},
		{capacity: 144, data: 5, type: xudt B, lock: Alice},
		{capacity: 201, data: "", type: "", lock: Alice} 
	],
	outputs: [
		{capacity: 144, data: 12-10, type: xudt A, lock: Alice},
		{capacity: 144, data: 5+10, type: xudt B, lock: Alice},
		{capacity: 201-1, data: "", type: "", lock: Alice} 
	]
}
```

Bob is willing to exchange 10 UDT-B for 10 UDT-A and pay 1 CKB as a transaction fee.

```j
{
	inputs: [
		{capacity: 144, data: 100, type: xudt B, lock: Bob},
		{capacity: 144, data: 10, type: xudt A, lock: Bob},
		{capacity: 201, data: "", type: "", lock: Bob} 
	],
	outputs: [
		{capacity: 144, data: 100-10, type: xudt B, lock: Bob},
		{capacity: 144, data: 10+10, type: xudt A, lock: Bob},
		{capacity: 201-1, data: "", type: "", lock: Bob} 
    ]
}
```

Final Transaction:

```
{
    inputs: [
		{capacity: 144, data: 12, type: xudt A, lock: Alice},
		{capacity: 144, data: 5, type: xudt B, lock: Alice},
		{capacity: 201, data: "", type: "", lock: Alice},
		{capacity: 144, data: 100, type: xudt B, lock: Bob},
		{capacity: 144, data: 10, type: xudt A, lock: Bob},
		{capacity: 201, data: "", type: "", lock: Bob},
		{capacity: 200, data: "", type: "", lock: Z}
	],
	outputs: [
		{capacity: 145-1, data: 12-10, type: xudt A, lock: Alice},
		{capacity: 145, data: 5+10, type: xudt B, lock: Alice},
		{capacity: 201-1, data: "", type: "", lock: Alice},
		{capacity: 145-1, data: 100-10, type: xudt B, lock: Bob},
		{capacity: 145, data: 10+10, type: xudt A, lock: Bob},
		{capacity: 201-1, data: "", type: "", lock: Bob},
		{capacity: 200+1, data: "", type: "", lock: Z} 
    ]
}
```

The Atomic Swap collects transaction fees from both parties, totaling 2 CKB. The actual resulting CKB transaction only pays 1 CKB as a transaction fee, while the remaining 1 CKB can be kept as a commission for the matching service.

### 1.3 Workflow

#### 1.3.1 Creation of OTX on the wallet side

The wallet creates a payment OTX in which all inputs to be unlocked must use the [Omni Lock](https://github.com/nervosnetwork/ckb-production-scripts/tree/opentx) that supports OTX. After signing the OTX, it is submitted to the OTX Pool operated by the payee.

In this pattern, the `cell dep`, `input`, `output`, and `witness` in the OTX are all determined, so they can be converted into the corresponding [`Essential Keys`](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md#essential-keys) relatively easily. The fields in `Essential Keys` correspond one-to-one with the fields in [CKB Open Transaction](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md). 

Our focus is on the `Extra Keys` used in this pattern: `Accouting Key Group`. Accounting helps to settle the balance of CKB capacities and UDT tokens.

| Name | key_type | key_data | value_data |
| --- | --- | --- | --- |
| Input CKB Capacity | OTX_ACCOUNTING_META_INPUT_CKB = 0x10040 | None | Uint64, the total input CKB capacity in Shannons. |
| Output CKB Capacity | OTX_ACCOUNTING_META_OUTPUT_CKB = 0x10041 | None | Uint64, the total output CKB capacity in Shannons. |
| Input xUDT Amount | OTX_ACCOUNTING_META_INPUT_XUDT = 0x10044 | Script serialized via Molecule | Uint128, the total input xUDT tokens identified by the type script serialized in key_data. |
| Output xUDT Amount | OTX_ACCOUNTING_META_OUTPUT_XUDT = 0x10045 | Script serialized via Molecule | Uint128, the total output xUDT tokens identified by the type script serialized in key_data. |

The above Extra key-value pairs, added at creation time, can directly display the input and output asset statistics for the current OTX, so that the party receiving that OTX does not need to traverse all inputs and outputs for balance statistics.

An example of a complete OTX in JSON format can be found here.

#### 1.3.2 Pool side

The OTX pool starts as a service, provides [RPCs](../../otx-pool/src/rpc/README.md) such as `submit_otx`, and initializes the Atomic Swap plugin as well as other plugins. When it receives a newly submitted OTX, it indexes it and notifies the plugin with a message.

#### 1.3.3 Plugin：Atomic Swap

When the OTX Pool notifies the plugin of a new OTX, the plugin checks if it has an Atomic Swap requirement by using the Account Extra Keys of the OTX. If it matches, the plugin will search for matching OTXs in its internal index:

-   If there is no matching OTX, the plugin will add the new OTX to its own OTX index for future matching.
-   If there is a matching OTX, the plugin will assemble the new OTX and the matched OTX into a final Ckb transaction and submit it to the Ckb node. When the Ckb transaction is sent successfully, the plugin deletes its own indexed OTX and immediately notifies the host OTX Pool, which deletes its own indexed otxs and sends a notification to inform other registered plugins.
