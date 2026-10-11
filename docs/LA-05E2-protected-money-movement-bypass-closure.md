# LA-05E2 — Package 4 protected payments bypass closure

**Authority:** Accepted ADR-BCP-026 and ADR-BCP-027; supplements LA-05E, LA-05C, and Shared merchant-readiness/v1.

The engine implements an in-memory, simulated sandbox payment lifecycle. Its existing `PaymentLegalOperation` / `execute_with_current_payment_actor` and negative documentary veto are **policy ports, not wired production providers**. A JWT containing `payment:execute` or `payment:refund` is not proof of an actual contracting party, payment beneficiary, PSP registration, documentary eligibility or merchant readiness.

## Executable boundary

- All HTTP money-changing operations—create intent, confirm (including automatic capture), cancel, capture, refund—are now **403/503 fail closed according to caller authentication and protected environment readiness**, before the service and idempotent replay lookup.
- Workload authentication and exact payment scope are still required. A correctly scoped caller in staging, production or any unrecognised environment receives RFC 9457 `503 PAYMENT_EXECUTION_NOT_CERTIFIED` with `Cache-Control: no-store`. Only explicit development/integration environments can use the synthetic sandbox fixtures.
- The guard is applied to the single HTTP `mutate` junction rather than a subset of handlers. New protected HTTP mutations inherit the denial. Direct unauthorised PaymentService mutations are not an external network API; protected provider composition must never expose them.
- Protected production remains impossible to configure with the current SANDBOX-only provider; the HTTP guard is *defense in depth* against a later deployment change accidentally opening simulated payment paths.
- The separate authenticated `/internal/merchant-readiness/v1/assess` remains `503 MERCHANT_CERTIFICATION_NOT_CONFIGURED` rather than manufacturing READY.
- Verified current CP `PAYMENT_BENEFICIARY` / `CONTRACTING_PARTY` permission, negative-only documentary clearance, certified PSP/merchant status, legal-actor selection, transaction isolation, real provider callbacks, transactionally durable idempotency and exact RUNTIME context are **not** inferred from this denial.
- Unit/integration tests cover every protected handler, replay, authenticated versus unscoped access and absence of emitted financial events.

## Gate to remove blanket refusal

Implement an independently certified persistent PSP adapter and dedicated workload-authenticated CP policy client, then atomically gate each irreversible native provider operation and retry on **fresh operation-scoped CP legal actor + documentary decision, correct historical billing/settlement responsibility, and separate PSP readiness**. Only after authenticated staging, provider certification and explicitly approved support claims may a reviewed policy replace the protected refusal. Disabling a feature flag is not a legal override.

**State:** executable fail-closed Package 4 bypass closure; **not** a real payments execution capability nor completion of cross-provider acceptance, which require Package 5/7/8 qualifications.
