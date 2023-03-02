# Open Tx Pool JSON-RPC Protocols

## Table of Contents

- [RPC Methods](#rpc-methods)
  - [Method `submit_otx`](#method-submit_otx)
  - [Method `query_otx_by_id`](#method-query_otx_by_id)
- [RPC Errors](#rpc-errors)
- [RPC Types](#rpc-types)
  - [Type `H256`](#type-h256)
  - [Type `JsonBytes`](#type-jsonbytes)
  - [Type `OpenTransaction`](#type-opentransaction)
  - [Type `OpenTxStatus`](#type-opentxstatus)
  - [Type `OpenTxWithStatus`](#type-opentxwithstatus)
  - [Type `OtxKeyPair`](#type-otxkeypair)
  - [Type `OtxMap`](#type-otxmap)
  - [Type `OtxMapVec`](#type-otxmapvec)
  - [Type `Uint32`](#type-uint32)

## RPC Methods

### Method `submit_otx`

- `submit_otx(otx)`
  - `otx`: [`JsonBytes`](#type-jsonbytes)
- result: [`OpenTxWithStatus`](#type-opentxwithstatus)

Submits a new open transaction into the open transaction pool.

##### Params

- `otx` - the CKB open transaction, which is serialized by molecule.

##### Returns

The RPC returns the open transaction hash, which is the CKB transaction hash after converting open tx to CKB tx. 

A well-formed Open Transaction can be converted from its own description format to CKB tx. If Open tx lacks a certain field, the corresponding default value will be filled during the conversion process to ensure the conversion is successful.

##### Examples

Request

```
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "submit_otx",
  "params": [
    "0x6b0400001c00000020010000260200002a020000df02000094030000040100001c00000038000000540000008c000000c4000000e40000001c0000001000000014000000140000000100000004000000000000001c0000001000000014000000140000000000010004000000010000003800000010000000140000001400000010000100200000004ba6616b9f1db87cd64dc179e53eb12e5591e7453565effd3d41149f38050922380000001000000014000000140000001100010020000000b973bc0d611a6d1d45adb7e331c3777313a34b28d84b58e290300ae7887541b320000000100000001400000014000000400001000800000000b707840300000020000000100000001400000014000000410001000800000000e40b5402000000060100000c000000890000007d0000001000000048000000640000003800000010000000140000001400000002000000200000008592d17f7d574cf51b744d66fe9e14a09b915ecaf7ff40450d270c8b2a7a13721c000000100000001400000014000000030000000400000003000000190000001000000014000000140000000400000001000000007d0000001000000048000000640000003800000010000000140000001400000002000000200000008592d17f7d574cf51b744d66fe9e14a09b915ecaf7ff40450d270c8b2a7a13721c0000001000000014000000140000000300000004000000090000001900000010000000140000001400000004000000010000000004000000b500000008000000ad000000140000004c0000006800000088000000380000001000000014000000140000000600000020000000598ed2f84e544ff5376ac73bf098b43f0b0c8c2a25014b3987ec6934cc63091b1c0000001000000014000000140000000700000004000000000000002000000010000000140000001400000008000000080000000000000000000000250000001000000014000000190000005000010001000000000800000000b7078403000000b500000008000000ad00000008000000a5000000100000001400000014000000090000008d0000008d000000100000008d0000008d0000007900000079000000100000007900000079000000650000000000000000000000010000001400f07f1600f0011200f07f20efbead20de0000f00000005eaf3e074183ae8bf2afe4728ab4c28a57839479d78d51264c4484abcd2477fa4e77a19c7ec713244f65bc1efe282929ca49fb2a202cd8bab5c4bab8da260c5301d700000008000000cf00000018000000380000007000000089000000b7000000200000001000000014000000140000000d0000000800000000e40b5402000000380000001000000014000000140000000e00000020000000bb4469004225b39e983929db71fe2253cba1d49a76223e9e1d212cdca1f79f28190000001000000014000000140000000f00000001000000012e00000010000000140000001400000010000000160000000030255bfdfe48baeb4036273c84babbb43a43dd5c10180000001000000014000000140000001500000000000000"
  ]
}
```

Response

```
{
  "id": 42,
  "jsonrpc": "2.0",
  "result": "0x4ba6616b9f1db87cd64dc179e53eb12e5591e7453565effd3d41149f38050922"
}
```

### Method `query_otx_by_id`

- `query_otx_by_id(otx)`
  - `id`: [`H256`](#type-h256)
- result: [`OpenTxWithStatus`](#type-opentxwithstatus) 

Return the status of an open transaction.

##### Params

- `otx` - the CKB open transaction hash.

##### Returns

This PRC returns the status of the queried open transaction.

##### Examples

Request

```
{
  "id": 42,
  "jsonrpc": "2.0",
  "method": "query_otx_by_id",
  "params": [
    "0x4ba6616b9f1db87cd64dc179e53eb12e5591e7453565effd3d41149f38050922"
  ]
}
```

Response


```
{
  "id": 42,
  "jsonrpc": "2.0",
    "result": {
    "otx": {
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
            "value_data": "0x11a63f9b2fbcabf6feb9c40b890290e70ffdf320c61c144cf8997f8adbe12e25"
        },
        {
            "key_type": "0x10011",
            "key_data": null,
            "value_data": "0x61fea25e3fe992213c6bba31eb75498a1a5372d55c36cee8153c6f1bf7077b49"
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
            "value_data": "0x20ba0c00def92c214b75ab30fd9a25a7e5d4d3ef3ae4a6477400adb7da6c8ac9"
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
            "value_data": "0x8d000000100000008d0000008d0000007900000079000000100000007900000079000000650000000000000000000000010000001400f07f1600f0011200f07f20efbead20de0000f0000000c29462d0b6d14d6fd60c2a05f7c4408a1979e3195c6cf8bda2c61103db89183c0ab2a428f76b2c5ffcb4f9867f2528c43f25eb4a5e30487b97405ea3acdf24ed01"
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
            "value_data": "0x00c61204d170d6cca289d317f89b119436040ef47910"
            },
            {
            "key_type": "0x15",
            "key_data": null,
            "value_data": "0x"
            }
        ]
        ]
    },
    "status": "Pending"
    }
}
```

## RPC Errors

## RPC Types

### Type `H256`

The 32-byte fixed-length binary data.

The name comes from the number of bits in the data.

In JSONRPC, it is encoded as a 0x-prefixed hex string.

#### Fields

`H256` is a JSON object with the following fields.

-   `0`: https://doc.rust-lang.org/1.61.0/std/primitive.array.html - Converts `Self` to a byte slice.

### Type `JsonBytes`

Variable-length binary encoded as a 0x-prefixed hex string in JSON.

##### Example


|  JSON | Binary |
| --- |--- |
|  “0x” | Empty binary |
|  “0x00” | Single byte 0 |
|  “0x636b62” | 3 bytes, UTF-8 encoding of ckb |
|  “00” | Invalid, 0x is required |
|  “0x0” | Invalid, each byte requires 2 digits |

### Type `OpenTransaction`

The open transaction.

Refer to RFC [CKB Open Transaction: An Extensible Transaction Format](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md).

#### Fields

`OpenTransaction` is a JSON object with the following fields.

- `meta`: [`OtxMap`](#type-otxmap) - Meta info map.
- `cell_deps`: [`OtxMapVec`](#type-otxmapvec) - An array of cell dep maps.
- `header_deps` : [`OtxMapVec`](#type-otxmapvec)- An array of header dep maps.
- `inputs`: [`OtxMapVec`](#type-otxmapvec) - An array of input cell maps.
- `witnesses` : [`OtxMapVec`](#type-otxmapvec) - An array of witness maps.
- `outputs` : [`OtxMapVec`](#type-otxmapvec) - An array of output cell maps.

### Type `OpenTxStatus`

Status for an open transaction.

`Status` is equivalent to `"Pending" | "Proposed" | "Committed" | "Unknown" | "Rejected"`.

- Status "Pending", the open transaction is in the pool, and not proposed yet.
- Status "Proposed", the open transaction is in the pool and has been proposed.
- Status "Committed", the open transaction has been committed to the canonical chain.
- Status "Unknown", the pool has not seen the open transaction, or it should be rejected but was cleared due to storage limitations.
- Status "Rejected", the open transaction has been recently removed from the pool. Due to storage limitations, the pool can only hold the most recently removed transactions.

### Type `OpenTxWithStatus`

The JSON view of an open transaction as well as its status.

#### Fields

`OpenTxWithStatus` is a JSON object with the following fields:

- `otx`: [`OpenTransaction`](#type-opentransaction) - The open transaction.

- `status`: [`OpenTxStatus`](#type-opentxstatus) - The open transaction status.

### Type `OtxKeyPair`

Key-value pair, the basic field unit to build CKB open transaction.

#### Fields

- `key_type` : [`Uint32`](#type-uint32) - Key type.
- `key_data` : [`JsonBytes`](#type-jsonbytes) `|` `null` - Key data.
- `value_data`: [`JsonBytes`](#type-jsonbytes) - Value.

In a map, the combination of key_type and key_data is unique.

### Type `OtxMap`

#### Fields

-   `0`: `Array<` [`OtxKeyPair`](#type-otxkeypair) `>` - A collection of `OtxKeyPair`.

### Type `OtxMapVec`

-   `0`: `Array<` [`OtxMap`](#type-otxmap) `>` - A collection of `OtxMap`.

### Type `Uint32`

The  32-bit unsigned integer type encoded as the 0x-prefixed hex string in JSON.

##### Examples


|  JSON | Decimal Value |
| --- |--- |
|  “0x0” | 0 |
|  “0x10” | 16 |
|  “10” | Invalid, 0x is required |
|  “0x01” | Invalid, redundant leading 0 |
