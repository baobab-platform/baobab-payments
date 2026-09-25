# baobab-payments

> **Status:** the Baobab Payment API runs with a **non-production sandbox provider** ([ADR-PAY-0001](docs/adr/ADR-PAY-0001%20—%20Adopt%20HyperSwitch.md) §15.2). **The HyperSwitch integration is NOT YET IMPLEMENTED.** Every sandbox result and event is marked `simulated: true`, because no money moves. The sandbox is refused in production, and state is in-memory, so staging is refused too until durable persistence exists.

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

## Runtime

A Rust (axum, tokio) service. Requests and events are validated against the Shared `payments/v1` contracts vendored under `contracts/shared`, which `contracts.lock.yaml` pins.

| Route | Scope | Purpose |
|---|---|---|
| `POST /v1/payment-intents` | `payment:execute` | Create an intent (`CreatePaymentIntentRequest`) |
| `POST /v1/payment-intents/{id}/confirm` | `payment:execute` | Authorise with an opaque, tokenised payment method reference. `AUTOMATIC` also captures. |
| `POST /v1/payment-intents/{id}/cancel` | `payment:execute` | Cancel an uncaptured intent, voiding any authorisation |
| `POST /v1/payments/{id}/capture` | `payment:execute` | Capture an authorised payment, in full or in part |
| `POST /v1/payments/{id}/refunds` | `payment:refund` | Refund, up to what remains captured |
| `GET /v1/tenants/{tenant_id}/payment-intents/{id}`, `GET /v1/tenants/{tenant_id}/payments/{id}` | `payment:read` | Reads |
| `GET /health/live`, `GET /health/ready` | none | Readiness reports `real_payments: NOT_CONFIGURED` while the provider is simulated |

**Guarantees:**

- **Authentication.** Callers present Baobab workload tokens: the issuer, the audience `baobab-payments`, `actor_type: workload`, an allowed client (`baobab-subscriptions` by default), the route's scope, and a lifetime of at most 15 minutes. Static secrets are not accepted.
- **Idempotency.** Every money-moving request needs an `Idempotency-Key`, scoped per tenant. A retry replays the stored response and never authorises, captures or refunds twice. The same key with a different body is refused.
- **Context.** The payment context comes from the calling engine, which resolved it with the Control Plane.
  - The amount's currency must be the context currency.
  - A workload can only move money for its own engine: `source_engine` must be the caller's client.
- **Tenant isolation.** Every read and write is keyed by tenant. Another tenant's identifiers return 404.
- **No card data.**
  - Unknown fields are rejected.
  - The payment method reference is used for the provider call only. It is never stored, logged, returned or put in events.
- **Sandbox behaviour.** The sandbox authorises every reference except those beginning `pm_sandbox_decline`, which it declines deterministically.
- **Events.** `payment.created`, `.authorized`, `.captured`, `.failed`, `.cancelled` and `.refunded` are recorded as canonical envelopes, and each carries `simulated`. Relaying them is **not built yet**.
- **Errors and observability.**
  - Errors are RFC 9457 problems.
  - `X-Correlation-ID` is honoured or minted.
  - Logs are JSON lines carrying the route template, status, duration and client, and never bodies or tokens.

### Configuration

| Variable | Default | Notes |
|---|---|---|
| `BAOBAB_ENVIRONMENT` | (required) | `development` or `integration`. Production is refused because of the sandbox, and staging and production are refused because of in-memory state. |
| `WORKLOAD_ISSUER`, `WORKLOAD_JWKS_URI` | (required) | The JWKS URI must use https outside development. Keys are cached for 5 minutes and refreshed on an unknown `kid`. |
| `WORKLOAD_AUDIENCE` | `baobab-payments` | |
| `WORKLOAD_ALLOWED_CLIENTS` | `baobab-subscriptions` | Comma-separated |
| `HTTP_PORT` | `8080` | |
| `SHUTDOWN_GRACE_SECONDS` | `10` | |

### Build, test and run

```sh
cargo test --workspace --all-features --locked
SHARED_CONTRACTS_DIR=../shared cargo test --workspace --all-features --locked   # plus the vendored-contract drift check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
docker build -t baobab-payments .
```

The image is distroless (`cc-debian13`, pinned by digest). It runs as a non-root user, carries OCI labels, and its `HEALTHCHECK` runs `baobab-payments healthcheck`. It needs no shell.

### Not built yet

- the HyperSwitch adapter (with Superposition, PostgreSQL and Redis);
- durable persistence;
- the outbox relay;
- webhook ingestion;
- metrics.

## Documentation

- [ADR-PAY-0001 — Adopt HyperSwitch as the Headless Baobab Payment Orchestration Engine](docs/adr/ADR-PAY-0001%20—%20Adopt%20HyperSwitch.md), including the 2026-09-25 runtime refresh. Current HyperSwitch requires **Superposition** in addition to PostgreSQL and Redis.
- [Contracts consumed and published](contracts/README.md)

## Local development

This repository uses the shared `baobab-dev` devcontainer (`full` profile, with Rust layered on, plus local PostgreSQL and Redis services). See `.baobab/environment.yaml` and `.devcontainer/`.
