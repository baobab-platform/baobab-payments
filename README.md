# baobab-payments

> **Status:** architecture accepted and refreshed ([ADR-PAY-0001](docs/adr/ADR-PAY-0001%20—%20Adopt%20HyperSwitch.md)). The runnable payment façade and its sandbox provider are being built. **The HyperSwitch integration is NOT YET IMPLEMENTED.**

The headless payment orchestration engine of the Baobab platform. It executes monetary obligations that other engines have authorised, such as commercial subscription payments from `baobab-subscriptions` and commerce payments from `baobab-trade`. It does so through a Baobab-owned API, with HyperSwitch as the foundational orchestration implementation behind it.

## Role

### This engine owns

- payment execution and orchestration;
- processor routing, retry and fallback within the context the Control Plane authorised;
- authorisation, capture, cancellation and refund coordination;
- payment operational state and processor references;
- processor webhook ingestion (future work).

### It explicitly does not own

| Concern | Owner |
|---|---|
| Tenant, legal entity, market and PlatformAccount authority | `baobab-cp` |
| ProductSubscription, subscription classification, entitlement | `baobab-cp` |
| What is owed and when (billing cycle, usage, credits) | `baobab-subscriptions` |
| Orders, carts, commerce pricing | `baobab-trade` |
| Accounting, ledger, receivables, revenue recognition | `baobab-erp` |

A payment outcome never grants capabilities: the Control Plane remains the entitlement authority. INTERNAL subscriptions never reach this engine, because they carry no monetary charge.

HyperSwitch's hierarchy (Organisation → Merchant → Profile) is an implementation detail:

```text
HyperSwitch Organisation != Baobab Organisation
HyperSwitch Merchant     != LegalEntity
HyperSwitch Profile      != Market
```

Every mapping between them is explicit.

## Contracts

All cross-repository contracts are canonical in [`baobab-platform/shared`](https://github.com/baobab-platform/shared). The Baobab Payment API and its `payment.*` events are defined in `contracts/payments/v1`.

## Documentation

- [ADR-PAY-0001 — Adopt HyperSwitch as the Headless Baobab Payment Orchestration Engine](docs/adr/ADR-PAY-0001%20—%20Adopt%20HyperSwitch.md), including the 2026-09-25 runtime refresh. Current HyperSwitch requires **Superposition** in addition to PostgreSQL and Redis.
- [Contracts consumed and published](contracts/README.md)

## Local development

This repository uses the shared `baobab-dev` devcontainer (`full` profile, with Rust layered on, plus local PostgreSQL and Redis services). See `.baobab/environment.yaml` and `.devcontainer/`.
