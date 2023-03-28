# CKB Open Transaction Pool

## About CKB Open Transaction

[CKB Open Transaction](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md) (OTX) is an extensible transaction format and workflow engine that supports the combination of multiple partial signed OTXs or multi-signed OTXs off-chain to construct a CKB transaction.

Compared to the CKB Transaction format described in the [RFC 0022-transaction-structure](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0022-transaction-structure/0022-transaction-structure.md), OTX can carry more auxiliary information to describe how it can be aggregated into a complete CKB transaction.

This project is an extensible OTX solution based on memory pools. We have developed several user cases in different application scenarios to make the solution reusable and versatile, which facilitates the secondary development of dApps.

## Design Philosophy

We believe that the optimal architecture involves processing OTXs as a data stream. This architecture utilizes a Broker to collect OTXs and dispatch them to Agents.

An Agent serves as both a Consumer and a Producer. It receives OTXs from the Broker, executes its logic, and notifies the Broker of its processing result.

This project is an implementation of this design concept. It uses the memory pool as the Broker and plugins as Agents to expand its application business logic.

## Open Transaction Lock Script

Open transactions require support from a corresponding lock script, which should follow the [RFC: Composable Open Transaction Lock Script](https://cryptape.notion.site/RFC-Composable-Open-Transaction-Lock-Script-b737e7281a6442e089c55350e8a9e15e). The locking script that adheres to this RFC can support the partial signing of transactions, providing great convenience for subsequent aggregation and reducing the cost of interaction.

The lock script used in this project is provided by the [Omni lock script](https://github.com/nervosnetwork/ckb-production-scripts/tree/opentx). We have also utilized the [CKB SDK](https://github.com/nervosnetwork/ckb-sdk-rust/pull/37), which has started to support this lock script.

## Documentation

- [Quick Start](./docs/quick-start.md)
- [RPC Documentation](./otx-pool/src/rpc/README.md)
- [Projects Layout](./docs/layout.md)
- [Pool and Plugin](./docs/pool-and-plugin.md)
- Scenario Application Pattern
    - [Payment](./docs/scenario-application-pattern/payment.md)
    - [Swap](./docs/scenario-application-pattern/swap.md)

## Integration Test

The [integration-tests](./integration-test/) sub-project provides various scenarios for applying OTX, which helps in understanding the project.

The integration-tests [README](./integration-test/README.md) outlines how to build a local development environment.

Existing scenario application cases include:

- [Payment: Dust Collector](./integration-test/src/tests/payment/dust_collector.rs#L29)
- [Swap: Atomic Swap](./integration-test/src/tests/swap/atomic_swap.rs#L41)
- ... more will be added later

Thank you for your contribution to the open-source community!
