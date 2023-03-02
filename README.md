# CKB Open Transaction Pool

## About CKB Open Transaction

[CKB Open Transaction](https://github.com/doitian/rfcs/blob/rfc-open-transaction/rfcs/0046-open-transaction/0046-open-transaction.md) is an extensible transaction format and a workflow engine to construct CKB transactions in an offline manner.

This project provides the design and basic implementation of the architecture, develops cases of different application scenarios, and provides the necessary tool modules, which can be used for secondary development.

## Design Philosophy

We believe in processing open transactions as a data stream. This architecture involves the use of a Broker, which collects open transactions and dispatches them to Agents. 

An agent is both a Consumer and a Producer. It receives open transactions from the Broker, executes its logic, and notifies the broker of its processing result.

This project is an implementation of this design concept, that is, using the memory pool as the broker, and using the plug-in as the agent to expand its application business logic.

## Open Transaction Lock Script

Open transaction needs the support of the corresponding lock script, which should follow the [RFC: Composable Open Transaction Lock Script](https://cryptape.notion.site/RFC-Composable-Open-Transaction-Lock-Script-b737e7281a6442e089c55350e8a9e15e). 

The lock script used in this project is provided by the [Omni lock script](https://github.com/nervosnetwork/ckb-production-scripts/tree/opentx). The [CKB SDK](https://github.com/nervosnetwork/ckb-sdk-rust/pull/37) has also started to support the lock script.

## Documentation

- [Quick Start](./docs/quick-start.md)
- [RPC Documentation](./otx-pool/src/rpc/README.md)
- [Projects Layout](./docs/layout.md)
- [Plug-in Protocol](./docs/plug-in-protocol.md)
- Scenario Application Pattern
    - [Payment](./docs/scenario-application-pattern/payment.md)
    - [Swap](./docs/scenario-application-pattern/swap.md)

## Integration Test

The [integration-tests](./integration-test/) sub-project provides various scenarios for applying open transaction and open tx pool, which helps in understanding the project.

The integration-tests [README](./integration-test/README.md) outlines how to build a local development environment.

Existing scenario application cases include:

- [Payment: Dust Collector](./integration-test/src/tests/payment/dust_collector.rs#L29)
- [Swap: Atomic Swap](./integration-test/src/tests/swap/atomic_swap.rs#L41)
- ... more will be added later

Thank you for your contribution to the open-source community!
