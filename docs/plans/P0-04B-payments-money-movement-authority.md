# P0-04B Payments — money-movement authority

**Authority:**
- ADR-SHARED-035, Payments Money-Movement Authority (shared#309, merged `2dc9a32`);
- ADR-PAY-0002, -0010, -0013, -0016, -0018, -0022;
- ADR-BCP-027 §6.1 and §10.2.

**CP side:** baobab-cp, the money-movement resolver (migration 000126).

## Rule

> Every independently initiated, consequential money movement must have current, operation-specific legal authority from CP and independent financial execution approval from Payments. Accounting authorization, group ownership, historical approvals and absence of restrictions never imply permission to move funds.

CP decides legal authority. Payments decides whether the movement is financially valid and safe. Both must permit, and neither overrides the other.

## What this increment implements (Gates A–C)

**Contracts.**
- Payments pins Shared `2dc9a32`, which adds `payments/v2` money-movement authority.
- That pin brings the closed catalogue, the request and decision schemas, and the golden fingerprint vectors.
- Owner golden vectors now cover the two new entries.
- The capability key is corrected to `payment.intent.create`.

**`src/money_movement.rs`.**
- **Classes.** Four classes: `authorization.create`, `capture.execute`, `refund.execute` and `authorization.void`.
  - `payments.payout.execute` is catalogued but never activatable.
- **Instruction.** The canonical instruction is built from server-verified values only.
- **Fingerprint.** It reproduces every Shared vector.
- **Decision check.** A decision is accepted only when all of these hold:
  - it answers exactly this request: context, engine, class, reference, attempt, fingerprint;
  - it is `AUTHORIZED` for the expected legal entity, tenant, Organisation and market;
  - it has the catalogued activity and capability;
  - its lease is 30 s or less, and still current;
  - it grants no provider permission.

**`src/money_movement_execution.rs`, the single enforcement point.**
1. **Payout** is refused before any I/O.
2. **Attempt 1.** Get a CP decision, then the negative-only documentary veto.
3. **Transaction 1.** Record the decision, the reservation and the outbox entry. The journal checks the decision, provider readiness and Payments' financial limits.
4. **Attempt n ≥ 2.** Get a fresh CP decision and run the documentary veto again.
5. **Transaction 2.** Record the decision and the dispatch. The journal requires the second decision and a current activation, and allows no earlier dispatch.
6. **Send** the provider instruction exactly once. The provider idempotency key is the reservation key.
7. **Transaction 3.** Record the observation and the outbox entry. The outcome is `CONFIRMED`, `REJECTED` or `UNKNOWN`.
8. **Unknown outcome.** On retry, read the provider record back:
   - confirmed → replay;
   - rejected → refused;
   - `ABSENT` → re-send once, on a fresh decision;
   - anything else → still unknown.
   It is never blindly re-sent.

**`db/migrations/0003_money_movement_authority.sql`, journal fences.** They hold even against direct SQL as the restricted runtime role.

*Tables and functions:*
- `authority_decision`: only AUTHORIZED decisions. Payments' own receipt time and transaction overwrite anything the caller supplies. An expired decision is refused.
- `dispatch_attempt` and `provider_observation`.
- `provider_activation` and `provider_activation_revocation`, the ADR-PAY-0022 readiness records. The runtime role gets SELECT only; payout is excluded.
- `reserve_money_movement()` is the only way to reserve a movement. The legacy `reserve_native_execution()` now refuses every money-moving kind.

*Reservation guard:*
- payout is always refused;
- the reservation needs a current attempt-1 decision for the same fingerprint and mandate, recorded in the same transaction;
- it needs a current provider activation;
- the financial checks run under a lock on the parent payment:
  - pending movements count against the balance until the provider rejects them;
  - capture: no more than the authorised amount;
  - void: in full, and only before any capture;
  - refund: no more than the confirmed captured balance, and only to the original payment method;
  - one authorisation per payment;
  - the same merchant, market, currency and Organisation as the parent.

*Dispatch guard:*
- an attempt ≥ 2 decision for the same instruction and mandate, recorded in the same transaction;
- the current activation;
- no earlier dispatch the provider has not proven `ABSENT`.

*COMMIT fences:* deferred constraint triggers re-check that the decision is still current at COMMIT.

**HyperSwitch transport.** Adds a read-only `retrieve`, used for reconciliation; 404 is `NotFound`. The refund id is Payments' own, so a lost refund response can be read back.

## Evidence

- **`db/tests/0003_money_movement_authority.sql` (PostgreSQL 17, as `payment_ci`).** It covers:
  - self-activation refused;
  - the legacy capture and payout routes refused;
  - payout decision and reservation refused;
  - no decision, an expired decision, a lease over 30 s, fingerprint drift, another mandate, a kind/class mismatch, a decision from another transaction;
  - missing and revoked activation;
  - a full lifecycle with balance limits;
  - unknown and rejected captures;
  - void rules;
  - dispatch rules, including re-send only after `ABSENT`;
  - replay and conflict;
  - immutability;
  - cross-tenant isolation;
  - the COMMIT fence.

  Dropping any one of the four guards makes the suite fail.
- **`money_movement` unit tests and `money_movement_execution` PostgreSQL 17 tests.** They use a loopback CP and a loopback provider that count every call. Covered:
  - the lifecycle needs two decisions per movement and sends each once;
  - replay sends nothing;
  - a changed instruction under the same key conflicts;
  - Payments' financial refusals happen despite CP authority;
  - CP denial, CP unavailable, fingerprint drift and documentary hold send nothing;
  - revocation between reservation and dispatch blocks the send, and the reservation dispatches later on fresh authority;
  - withdrawn readiness blocks the send;
  - a lost response is reconciled, never re-sent;
  - a lost request is re-sent once, on attempt 3;
  - void only before capture.
- **CI.** `la05-payment-journal` runs 0001 → 0002 → 0003, each with its tests. `la05-e5-native-postgres` runs the executor tests with `LA05_REQUIRE_PG`, so a missing database fails the job instead of skipping it.

## Not in this increment

- **HTTP wiring and the activation policy:**
  - routing the public routes through this enforcement point in development/integration;
  - replacing the blanket protected-environment 503 with a route- and operation-specific policy that still refuses.

  These come with Gate D1 (next PR), which needs them.
- **Gate D1:** the real Payments service, the real CP binary, real signed workload tokens, PostgreSQL 17 and a deployed HyperSwitch with a test connector, including the crash window.
- **Not yet:** Gate D2, Gate E, real IAM, real mandates, merchant certification, payout activation and production.

Protected environments still return `503 PAYMENT_EXECUTION_NOT_CERTIFIED`.
