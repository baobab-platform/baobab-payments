# LA-05E5 — PostgreSQL journal ↔ Rust transaction service ↔ HyperSwitch

**Authority:** ADR-BCP-026/027, ADR-PAY-0001, PAY-0004, PAY-0010, PAY-0011, PAY-0015, PAY-0022.

## Executable increment

`src/native_execution.rs` connects the existing `payment_ops` PostgreSQL 17 tables with a real Rust asynchronous transaction service and the `HyperSwitchTransport` of LA-05E3. `PaymentService::create_native_intent` delegates to this native service *only* if a trusted internal composition root explicitly installs it. Existing public HTTP handlers and the bootstrap runtime do **not** install the native service or enable production PSP operations.

A local synthetic rehearsal:

1. Validates tenant, legal actor, organisation, legal market, currency, amount, true external merchant binding and token-only HyperSwitch request shape **before** acquiring an operation slot.
2. Uses a fresh CP legal-actor mandate and negative-only documentary clearance twice, plus an independent provider-readiness authority.
3. Opens PostgreSQL, uses transaction-local tenant+legal-actor session context and FORCE RLS, takes an operation-key advisory transaction lock, and idempotently reserves one immutable operation plus its outbox record.
4. Commits the intent **before** the HyperSwitch request. A duplicate operation-key request never resends a money instruction. An ambiguous crash requires independent reconciliation.
5. After the database commit, refreshes the CP legal actor, documentary hold and independent provider-readiness assessments **again**, requiring the SAME mandate. A revocation/expiry stops provider dispatch and leaves a reserved operation requiring reconciliation.
6. Sends one real HTTPS HyperSwitch Create intent with `confirm=false`. Records the provider reply as an **observation**, not a canonical money-captured fact. Provider errors and ambiguous timeouts produce an immutable reconciliation-needed outbox record; there is no automatic money retry.
7. The synthetic PostgreSQL 17/HTTP CI test validates durable records and exactly-one-send replay behaviour using synthetic-only actor/binding IDs.

## Deliberate protected-runtime refusal

- No real CP or IAM authority adapter, certified PSP/merchant credentials, banking beneficiary/settlement verification, AWS account or environment is supplied by this PR. A `ProviderBinding` is a **trusted internal port input**, not evidence of certification.
- Only local/test/integration `NoTls` PostgreSQL connections to loopback are enabled; all protected environment constructors and non-loopback database URLs are rejected.
- Only `CREATE_INTENT` with `confirm=false` is implemented; AUTHORIZE/CAPTURE/VOID/REFUND/PAYOUT/SETTLEMENT and webhook authenticity remain separate mandatory financial gates. No production approval or ledger event is emitted.
- The journal's `provider_receipt` table is deliberately **not** populated by unsigned synchronous PSP API observations; authenticated webhook evidence requires actual signature verification, event identity and market/provider installation.
- The new `PaymentService` integration slot is not installed in `main.rs`. The process retains its existing fail-closed sandbox/protected configuration. The old in-memory state is not migrated or represented as a durable ledger.

## Remaining implementation requirements

Production must supply PostgreSQL TLS/mTLS, pooled transactional connections and Pg identity, IAM-authenticated client-to-tenant context, CP point-of-use real endpoints, independent certified merchant activation and settlement routing, atomic terminal-outcome projection and event-publisher delivery cursor, HyperSwitch signed webhooks with duplicate/reordered event recovery, multi-operation lifecycle transitions, ERP reconciliation and real protected staging proof. These are not assumed or simulated.

**Verdict:** The durable journal is wired to an executable synthetic Rust service and HyperSwitch transport, but full native Payments production convergence is **not complete**.
