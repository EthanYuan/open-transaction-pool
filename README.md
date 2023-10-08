# CKB Open Transaction Pool

## About CKB Open Transaction

The [CKB Open Transaction](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md) (OTX) is an extensible transaction format and workflow engine guide that supports the combination of multiple OTXs with partial transaction signatures, as well as multiple-party signatures for the same multi-signature transaction, to construct a complete [CKB](https://github.com/nervosnetwork/ckb) transaction off-chain.

Compared to the CKB Transaction format described in the [RFC 0022-transaction-structure](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0022-transaction-structure/0022-transaction-structure.md), OTX can carry more auxiliary information to describe how it can be aggregated into a complete CKB transaction.

This project is a scalable OTX solution based on memory pool. We have developed several user cases in different application scenarios which can be used as reference cases for secondary development of dApps.

## Design Philosophy

We believe that the optimal architecture involves processing OTXs as a data stream. This architecture utilizes a Broker to collect OTXs and dispatch them to Agents.

An Agent serves as both a Consumer and a Producer. It receives OTXs from the Broker, executes its logic, and notifies the Broker of its processing result.

This project is an implementation of this design concept. It uses the memory pool as the Broker and plugins as Agents to expand its application business logic. We have developed several built-in [plugins](./plugins-built-in/) which follow the [plug-in protocol](./otx-plugin-protocol/) and are suitable for use in different application scenarios.

## Open Transaction Lock Script

Open transactions require support from a corresponding lock script, and the project currently integrates [otx-sighash-lock](https://github.com/EthanYuan/otx-sighash-lock), and its corresponding [SDK](./otx-sdk/), which supports partial transaction signature verification, making the aggregation of multiple signed transactions very simple - and significantly reducing the cost of interaction.

## Documentation

- [Quick Start](./docs/quick-start.md)
- [RPC Documentation](./otx-pool/src/rpc/README.md)
- [Projects Layout](./docs/layout.md)
- [Pool and Plugin](./docs/pool-and-plugin.md)
- Scenario Application Pattern
    - [Swap](./docs/scenario-application-pattern/atomic-swap.md)
    - [Payment](./docs/scenario-application-pattern/payment.md)
## Integration Test

The [integration-tests](./integration-test/) sub-project provides various scenarios for applying OTX, which helps in understanding the project.

The integration-tests [README](./integration-test/README.md) outlines how to build a local development environment.

Existing scenario application cases include:

- [Atomic Swap: CKB to UDT](./integration-test/src/tests/swap/atomic_swap_ckb_to_udt.rs)
- [Atomic Swap: UDT to UDT](./integration-test/src/tests/swap/atomic_swap_udt_to_udt.rs)
- [Payment: small blank check](./integration-test/src/tests/payment/small_blank_check.rs)
- ... more will be added later