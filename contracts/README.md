# Contracts

This engine defines no canonical schema of its own. The canonical contracts live in [`baobab-platform/shared`](https://github.com/baobab-platform/shared) and are pinned by commit.

| Shared contract | Role here |
|---|---|
| `contracts/payments/v1` | **Producer**: the Baobab Payment API (payment intents, payments, refunds) and `payment.*` events. Sandbox results are always marked `simulated`. |

## Where each contract is enforced

The files the runtime validates against are vendored byte-for-byte under `contracts/shared` and embedded at build time. The Shared commit `a9c53087d63caa356edf54ec8037ba1713c4827f` is pinned in `/contracts.lock.yaml`. `scripts/sync_contracts.py` copies all 19 declared files from immutable Git blobs and generates the Rust include inventory and SHA-256 provenance; `--check` also rejects obsolete extra files. `tests/pinned_contracts.rs` fails on any drift when `SHARED_CONTRACTS_DIR` is set, as it is in CI.

| Contract | Enforced by |
|---|---|
| `payments/v1` requests | Every request is validated before use (`service.rs`). The rule that the amount's currency must equal the context currency, which JSON Schema cannot express, is enforced in code. |
| `payments/v1` `PaymentIntent`, `Payment`, `Refund`, events; `events/v1` envelope | Every scenario's outputs are validated in `tests/payments.rs` |
| `errors/v1` problem details | Every error response (`http.rs`), validated in the tests |
| `authorization/v1` scopes `payment:execute`, `payment:refund`, `payment:read` | Per route (`http.rs`, `auth.rs`) |

## P0-04A authority consumers

| Owned entry | Enforcement |
|---|---|
| `control-plane/v1/PlatformContextResolveRequest`, `PlatformContext` | CP HTTP client validates input before IAM contact and response before existing exact tenant/Organisation/market/lease checks. Stored Context identifiers also use the owner's UUID format. |
| `organisation/v2/AssessLegalActorRequest`, `AssessLegalActorResponse` | CP transport and consequential policy ports validate closed schemas before current actor, mandate, operation and independent PSP checks. |
| `admission/v2/DocumentaryRestrictionAssessmentRequest`, `DocumentaryRestrictionAssessmentResponse` | Transport and injected policy ports enforce the owner wire before negative-only Organisation/operation binding and short freshness window. |
| `payments/v1/ReadinessRequest` | Private workload-authenticated merchant-readiness route uses the embedded owner schema. Valid requests still receive 503 without independent certification. |
| Workload/scope registries | Declared and byte-pinned vocabulary; no runtime admission or synthetic activation is inferred. |

Schema resolution is explicitly offline and formats are assertions. JSON authority
boundaries reject duplicate members recursively. CP/IAM bodies retain at most
16 KiB/8 KiB respectively, enforced while reading under the existing three-second
complete response timeout. OAuth provider envelopes are not silently relabelled
as Baobab token-exchange contracts. Error diagnostics contain owner schema paths,
never instance values. Readiness returns no fabricated `READY` decision.

Run the maintainer regeneration/check commands in the root README. The independent
Python owner checker loads original Git blobs and compares 130 synthetic golden
vectors across all eight authority entries with the compiled Rust validator.
Public payment requests/resources/events retain their existing conformance tests.
See the [audit and maintenance plan](../docs/plans/P0-04A-payments-contract-convergence.md).
