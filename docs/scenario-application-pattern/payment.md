# Payment

In payment scenarios, there are various sub-models that can be named to organize these models. Each model can be implemented independently as an agent plugin.

## 1 Pattern: Dust Collector

### 1.1 Pattern Explanation

The Dust Collector pattern involves collecting multiple small, "blank checks" (i.e. OTX without payee) to solve the 61 Capacity problem and the ACP collection hotspot problem that exists with online transfer transactions. This pattern also supports xUDT payments.

The broker in this pattern is usually the centralized payee. After gathering a certain number of small, blank checks, the payee can add its own output, as well as its own input, cell dep, and signature as needed (e.g., when the total amount collected is insufficient to create a new payee output cell).

This pattern has two risks since no payee is specified:

Open Transaction leakage, where anyone can receive the balance in an Open Transaction by adding their own output when assembling the final Ckb tx.
Miners do evil, they split the blank check open transaction from the complete tx for profit. A countermeasure is for brokers to submit Ckb tx to a self-built honest node, or a Ckb node they trust, making the success rate of the evil node less.

### 1.2 Open Tx Overview

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

#### 1.3.1 Creation of open tx on the wallet side

The wallet party creates the payment open tx, signs it, and submits it to the open transaction pool operated by the payee. Here the format of Open Transaction follows [CKB Open Transaction](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md) Format.

In this pattern, the `cell dep`, `input`, `output`, and `witness` in the Open Transaction are all determined, so they can be converted into the corresponding `Basic Keys` relatively easily.

The custom key that needs to be used is the Accounting Keys:

- `Input CKB Capacity` : the total input CKB capacity in Shannons.
- `Output CKB Capacity` : the total output CKB capacity in Shannons.
- `Input xUDT Amount` :  the total input xUDT tokens identified by the type script serialized in `key_data`.
- `Output xUDT Amount` : the total output xUDT tokens identified by the type script serialized in `key_data`.

The above custom Open Transaction key-value pairs, added at creation time, can directly display the input and output asset statistics for the current Open Transaction, so that the party receiving that Open Transaction does not need to traverse all inputs and outputs for balance statistics.

#### 1.3.2 Broker side

The Open Transaction pool starts as a service, provides RPCs such as `submit_otx`, and initializes the dust collector plugin as well as other plugins. When it receives a newly submitted Open Transaction, it indexes it and notifies the plugin with a message.

#### 1.3.3 Agent side

In the dust collector pattern, the agent plugin acts as the payee. After receiving payment Open Transactions for a period of time, it merges the currently accumulated Open Transactions, assembles them into a final Ckb transaction, and submits it to the Ckb node. When the Ckb transaction is sent successfully, the agent deletes its own indexed Open Transaction and immediately notifies the broker, which deletes its own indexed otxs and sends a notification to inform other registered plugins.

## 2. Pattern: Enhanced Dust Collector

### 2.1 Pattern Explanation

Compared to a blank check, the Enhanced Dust Collector pattern specifies the payee in the Open Transaction (OTX).

In this pattern, multiple OTXs with the same payee can be concatenated and merged. However, it is required that the input and output for each payee in the OTXs are fixed in position.

The pattern requires the payee to do a full sign at the end, making it safer than the Dust Collector pattern.

### 2.2 Open Tx Overview

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
