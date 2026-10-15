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

## Gate D1 (second increment)

### Public routes through the enforcement point

- **Activation policy** (`src/activation.rs`): each public mutation is either `Sandbox`, `Native` or `Refused`.
  - Staging, production and any unrecognised environment are `Refused` (503 `PAYMENT_EXECUTION_NOT_CERTIFIED`) on every route, even with a native backend present.
  - Activating a protected environment is a reviewed code change here, never a flag.
- **Native backend** (`PAYMENTS_NATIVE_BACKEND=postgres+hyperswitch`): configuration accepts it only in development and integration.
  - Every remote hop is TLS (CP, IAM, HyperSwitch); secrets are files.
  - The journal is reached only on loopback.
- **`src/native_api.rs`:** the Shared `payments/v1` routes on the native backend.
  - Create is an intent (no money moves) with the live CP legal-actor and documentary checks.
  - Confirm is an authorisation, then capture, cancel (a void once authorised) and refund.
  - The instruction is computed from Payments' own intent row and the tenant's merchant binding (`src/merchant_binding.rs`), never from the request body.
  - Reads come from the journal. Native execution is MANUAL capture only.
- **Migration 0004:**
  - the intent record;
  - one request per Idempotency-Key;
  - a cancellation record. A cancelled intent can never be authorised, and an authorised one can only be voided.

### Rejections must be proven

A provider refusal is journalled `REJECTED` only when the provider's own record proves the movement had no effect; otherwise it stays `UNKNOWN`. It keeps holding the balance and is reconciled, never re-sent. Reconciliation reads payments with `force_sync=true`.

### The harness

`acceptance/gate-d1/` runs:
- the release Payments image;
- the CP release binary at a pinned merged commit;
- a test OIDC issuer signing real RS256 tokens;
- a TLS edge with a throwaway CA;
- PostgreSQL 17;
- the `hyperswitch-router` v1.127.0 image, whose **unmodified Stripe connector** points at the deterministic test processor.

**CP seeding** goes through CP's admin API for:
- tenant bootstrap and activation;
- market creation, with four-eyes activation;
- the mandate: propose, decide and activate by three distinct humans, and terminate for revocation.

Four rows have no admin API and are recorded as **synthetic** in the evidence:
- the human identities;
- the legal-entity verification;
- the tenant's market assignment;
- the `baobab-payments` engine instance binding.

**Scenarios** (`driver/gate_d1.py`):

| # | Scenario | Asserted |
|---|---|---|
| 1 | Authorise, capture and refund in full | One provider operation each and two CP decisions each; read-back is consistent across the Payments journal, HyperSwitch and the processor |
| 2 | One partial capture; partial refunds up to the captured amount | The remainder is released and a second capture is rejected by the provider; an over-limit refund is refused before any call |
| 3 | Overcapture; double capture | Refused by Payments, no provider call |
| 4 | Void, then a capture after the void | The void succeeds; the capture is 409 with no provider call |
| 5 | Void after capture | 409, no provider call |
| 6 | Identical replay; changed request under the same key; duplicate refund | No second provider call; 409 for the changed request |
| 7 | Declined card | The intent is FAILED and the journal records REJECTED |
| 8 | CP revokes the mandate between the reservation's decision and the execution-point decision | 403; reserved but never dispatched; CP recorded the denial |
| 9 | CP unavailable | 503 and retryable; no provider call |
| 10 | Provider activation withdrawn | 503; CP authorises but Payments refuses |
| 11 | Forged legal entity, foreign platform account, unbound tenant, missing scope, invalid token or wrong audience | Refused; nothing journalled |
| 12 | **Crash window:** Payments is killed after the processor applied the capture and before the outcome was recorded | After restart the retry reconciles from the provider; one capture only |
| 13 | Lost response | The capture stays UNKNOWN, is never re-sent and keeps holding the balance (see finding 2) |
| 14 | Dropped request | The retry proves it absent and dispatches once more on a fresh CP decision |
| 15 | Payout and direct SQL as the runtime role | No route; payout reservation, dispatch without a decision, history rewrite and self-activation are all refused; even the owner cannot rewrite history |
| 16 | Staging and production with the native backend configured | Startup refused |

**Evidence:** `gate-d1-report.txt` (`Gate D1 failures: N`), `gate-d1-evidence.json`, the processor call log, the issuer's token log and every service log. The run fails if a log carries a token or a card number.

**Production exclusion:**
- `scripts/verify_no_test_processor.py` fails if anything outside `acceptance/gate-d1/` references the test services, or points the Stripe connector anywhere but Stripe.
- The test services refuse to start outside `BAOBAB_ENVIRONMENT=integration`.

### Findings from Gate D1

1. **Context lifetime.** CP's runtime context lives 15 minutes by default; Payments accepts at most 10. The harness sets `PLATFORM_CONTEXT_TTL=5m`. A deployment must set it too, or align the two bounds.
2. **Lost response in HyperSwitch.** HyperSwitch v1.127's Stripe connector retries a capture whose response was lost, without an idempotency key. The processor refuses the retry, and HyperSwitch records the payment as `failed` although it was captured. Payments now refuses to believe either answer. A real PSP sandbox (Gate D2) must show whether its own webhook or sync heals this.
3. **Stale provider record.** After a connector timeout, HyperSwitch's record reads `processing` until it is asked to sync.

## Not in these increments

- **Signed webhook replay** has no public route on the native backend; it stays proven by the `verified-webhook-settlement` job, not by Gate D1.
- **Not yet:** Gate D2, Gate E, real IAM, real mandates, merchant certification, payout activation and production.

Protected environments still return `503 PAYMENT_EXECUTION_NOT_CERTIFIED`.
