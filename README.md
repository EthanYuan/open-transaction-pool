# open-transaction-pool

## What is CKB Open Transaction

[CKB Open Transaction](https://github.com/nervosnetwork/rfcs/pull/406) is an extensible transaction format and a workflow engine to construct CKB transactions.

The Open Transaction divides transaction construction into multiple small steps, each with a different modularized solution. A modular Open Transaction ecosystem could expand the possibilities for CKB DApps while lowering the barrier to development.

## Design Philosophy

We believe the best architecture is processing the open transactions as a data stream. In such architecture, the **Broker** will collect open transactions and dispatch them to **Agents**. The broker in this project is a local memory pool, and the agents are its plug-ins.

An agent is both a **Consumer** and a **Producer**. It receives open transactions from the Broker, executes its logic, and emits the modified transactions or new transactions to the Broker.

## Integration Test

[Integration tests](./integration-test/) help understand how the entire project is used.

[README](./integration-test/README.md) describes how to build a local test environment.

typical test cases: 

- [dust collector mode](./integration-test/src/tests/payment/dust_collector.rs)
- atomic swap
- ... more will be added later
