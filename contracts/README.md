# Contracts

This engine defines no canonical schema of its own. The canonical contracts live in [`baobab-platform/shared`](https://github.com/baobab-platform/shared) and are pinned by commit.

| Shared contract | Role here |
|---|---|
| `contracts/payments/v1` | **Producer**: the Baobab Payment API (payment intents, payments, refunds) and `payment.*` events. Sandbox results are always marked `simulated`. |

## Where each contract is enforced

The files the runtime validates against are vendored byte-for-byte under `contracts/shared` and embedded at build time. The Shared commit is pinned in `/contracts.lock.yaml`. `tests/pinned_contracts.rs` fails on any drift when `SHARED_CONTRACTS_DIR` is set, as it is in CI.

| Contract | Enforced by |
|---|---|
| `payments/v1` requests | Every request is validated before use (`service.rs`). The rule that the amount's currency must equal the context currency, which JSON Schema cannot express, is enforced in code. |
| `payments/v1` `PaymentIntent`, `Payment`, `Refund`, events; `events/v1` envelope | Every scenario's outputs are validated in `tests/payments.rs` |
| `errors/v1` problem details | Every error response (`http.rs`), validated in the tests |
| `authorization/v1` scopes `payment:execute`, `payment:refund`, `payment:read` | Per route (`http.rs`, `auth.rs`) |
