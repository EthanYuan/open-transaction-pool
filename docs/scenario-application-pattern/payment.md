# Payment

In payment scenarios, there are various sub-patterns that can be named to organize these patterns. Each pattern can be implemented independently as an agent plugin.

## 1 Pattern: Dust Collector

The test case for this pattern have already been implemented in [Payment: Dust Collector](../../integration-test/src/tests/payment/dust_collector.rs#L29).

### 1.1 Pattern Explanation

The naming of this pattern was inspired by "Bitcoin Dust".

The Dust Collector pattern involves collecting multiple small, "blank checks" (i.e. Open Transaction without payee) to solve the 61 Capacity problem and the ACP collection hotspot problem that exists with online transfer transactions. This pattern also supports xUDT payments.

The Open Transaction (OTX) Pool in this pattern is usually the centralized payee. After gathering a certain number of small, blank checks, the payee can add its own output, as well as its own input, cell dep, and signature as needed (e.g., when the total amount collected is insufficient to create a new payee output cell).

This pattern has two risks since no payee is specified:

- OTX leakage, where anyone can receive the balance in the OTX by adding their own output when assembling the final Ckb tx.
- Miners do evil, they split the blank check OTX from the complete transaction for profit. A countermeasure is for payee to submit CKB transaction to a self-built honest node, or a Ckb node they trust, making the success rate of the evil node less.

### 1.2 OTX Overview

Alice's otx：

```
{
    inputs: [
        {capacity: 151, data: "", type: "", lock: Alice} 
    ],
    outputs: [
        {capacity: 151-51, data: "", type: "", lock: Alice}
    ]
}
```

Bob's otx：

```
{
    inputs: [
        {capacity: 144, data: 51, type: xudt z, lock: Bob} 
    ],
    outputs: [
        {capacity: 144, data: 51-11, type: xudt z, lock: Bob} 
    ]
}
```

Payee's final Ckb tx:

The payee assembles the final Ckb transaction, which receives a total of 51 CKB from Alice and 11 UDT from Bob, and pays 1 CKB as a transaction fee.

```
{
    inputs: [
        {capacity: 151, data: "", type: "", lock: Alice},
        {capacity: 144, data: 51, type: xudt z, lock: Bob},
        {capacity: 200, data: "", type: "", lock: Payee},
        {capacity: 144, data: 10, type: xudt z, lock: Payee}
    ],
    outputs: [
        {capacity: 151-51, data: "", type: "", lock: Alice},
        {capacity: 144, data: 51-11, type: xudt z, lock: Bob},
        {capacity: 200+50, data: "", type: "", lock: Payee},
        {capacity: 144, data: 10+11, type: xudt z, lock: Payee}
    ]
}
```

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

Here is an example of an OTX in JSON format:
```json
{
  "meta": [
    {
      "key_type": "0x1",
      "key_data": null,
      "value_data": "0x00000000"
    },
    {
      "key_type": "0x10000",
      "key_data": null,
      "value_data": "0x01000000"
    },
    {
      "key_type": "0x10010",
      "key_data": null,
      "value_data": "0x1b4c21c9858e1d0d7295cbec4753b903440d4c16a2ad4c3b539489de33bfda0c"
    },
    {
      "key_type": "0x10011",
      "key_data": null,
      "value_data": "0xb1101d49b7586e20905029592bbd26049de28beaffb6bdb3c49d21c95d36bc50"
    },
    {
      "key_type": "0x10040",
      "key_data": null,
      "value_data": "0x00b7078403000000"
    },
    {
      "key_type": "0x10041",
      "key_data": null,
      "value_data": "0x00e40b5402000000"
    }
  ],
  "cell_deps": [
    [
      {
        "key_type": "0x2",
        "key_data": null,
        "value_data": "0x8592d17f7d574cf51b744d66fe9e14a09b915ecaf7ff40450d270c8b2a7a1372"
      },
      {
        "key_type": "0x3",
        "key_data": null,
        "value_data": "0x03000000"
      },
      {
        "key_type": "0x4",
        "key_data": null,
        "value_data": "0x00"
      }
    ],
    [
      {
        "key_type": "0x2",
        "key_data": null,
        "value_data": "0x8592d17f7d574cf51b744d66fe9e14a09b915ecaf7ff40450d270c8b2a7a1372"
      },
      {
        "key_type": "0x3",
        "key_data": null,
        "value_data": "0x09000000"
      },
      {
        "key_type": "0x4",
        "key_data": null,
        "value_data": "0x00"
      }
    ]
  ],
  "header_deps": [],
  "inputs": [
    [
      {
        "key_type": "0x6",
        "key_data": null,
        "value_data": "0x0f0748b59a2c734bee5942dc59566553043b0145ff31b3ce48c473b2df9f09c4"
      },
      {
        "key_type": "0x7",
        "key_data": null,
        "value_data": "0x00000000"
      },
      {
        "key_type": "0x8",
        "key_data": null,
        "value_data": "0x0000000000000000"
      },
      {
        "key_type": "0x10050",
        "key_data": "0x00",
        "value_data": "0x00b7078403000000"
      }
    ]
  ],
  "witnesses": [
    [
      {
        "key_type": "0x9",
        "key_data": null,
        "value_data": "0x8d000000100000008d0000008d0000007900000079000000100000007900000079000000650000000000000000000000010000001400f07f1600f0011200f07f20efbead20de0000f00000005cd3f884ed0775d28fa5a0d2af5c0c456c76433f14a94934279f36c9d6b7c41d6615635e87cdc05bb8396226b66aa29bb7eeec27f6e9fe4a037f6723ed78d9e401"
      }
    ]
  ],
  "outputs": [
    [
      {
        "key_type": "0xd",
        "key_data": null,
        "value_data": "0x00e40b5402000000"
      },
      {
        "key_type": "0xe",
        "key_data": null,
        "value_data": "0xbb4469004225b39e983929db71fe2253cba1d49a76223e9e1d212cdca1f79f28"
      },
      {
        "key_type": "0xf",
        "key_data": null,
        "value_data": "0x01"
      },
      {
        "key_type": "0x10",
        "key_data": null,
        "value_data": "0x00ab5821cf74ba8448ec9030f40a37963daeb70b3d10"
      },
      {
        "key_type": "0x15",
        "key_data": null,
        "value_data": "0x"
      }
    ]
  ]
}
```

#### 1.3.2 Pool side

The OTX pool starts as a service, provides [RPCs](../../otx-pool/src/rpc/README.md) such as `submit_otx`, and initializes the dust collector plugin as well as other plugins. When it receives a newly submitted OTX, it indexes it and notifies the plugin with a message.

#### 1.3.3 Plugin：Dust Collector

Whenever the OTX Pool notifies the plugin of a new OTX, the plugin checks whether it conforms to the payment behavior of blank checks using the OTX's Account Extra Keys. If it does, the plugin adds it to its own OTX index.

After receiving payment OTX for a period of time, it merges the currently accumulated OTXs, assembles them into a final Ckb transaction, and submits it to the Ckb node. 

When the Ckb transaction is sent successfully, the plugin deletes its own indexed OTX and immediately notifies the host OTX Pool, which deletes its own indexed otxs and sends a notification to inform other registered plugins.

## 2. Pattern: Personal Check

### 2.1 Pattern Explanation

Compared to a blank check, the Personal Check pattern specifies the payee in the OTX.

In this pattern, multiple OTXs with the same payee can be concatenated and merged. However, it is required that the input and output for each payee in the OTXs are fixed in position.

The pattern requires the payee to do a full sign at the end, making it safer than the Dust Collector pattern. The payment of the OTX can only be finally decided and signed by the payee.

### 2.2 OTX Overview

Alice's otx

```
{ 
    inputs: [ 
        {capacity: "", data: "", type: "", lock: secp Payee},
        {capacity: 151, data: "", type: "", lock: otx Alice}
    ], 
    outputs: [     
        {capacity: "", data: "", type: "", lock: secp Payee},
        {capacity: 151-51, data: "", type: "", lock: otx Alice}
    ]
}
```

Bob's otx

```
{ 
    inputs: [ 
        {capacity: "", data: "", type: "", lock: secp Payee},
        {capacity: 144, data: 50, type: xudt z, lock: otx Bob}
    ], 
    outputs: [ 
        {capacity: "", data: "", type: "", lock: secp Payee},
        {capacity: 144, data: 50-50, type: xudt z, lock: otx Bob}
    ]
}
```

Payee's final Ckb tx:

```
{ 
    inputs: [ 
        {capacity: 142, data: 100, type: xudt z, lock: secp Payee},
        {capacity: 151, data: "", type: "", lock: otx Alice}, 
        {capacity: 144, data: 51, type: xudt z, lock: otx Bob}
    ], 
    outputs: [ 
        {capacity: 142+50, data: 100+50, type: xudt z, lock: secp Payee},
        {capacity: 151-51, data: "", type: "", lock: otx Alice}, 
        {capacity: 144, data: 50-50, type: xudt z, lock: otx Bob}
    ]
}
```

### 2.3 Workflow

To be continued.
