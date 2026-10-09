# LA-05E — Payment beneficiary PEP candidate (NOT production accepted)

**Authority:** ADR-BCP-027 LA-05, Shared #261, CP LA-05A #299.

The new `src/legal_actor.rs` module defines an opt-in, deny-by-default payment execution adapter that takes a prior CP-owned RUNTIME context ID, current CP `PAYMENT_BENEFICIARY` assessment, matched canonical LegalEntity, exact market/activity/capability/operation, bounded evidence TTL and independently registered merchant/provider readiness. It invokes no native payment mutation until both independent checks have passed. On retries, CP must be consulted again.

This **does not** enable processor payments or replace the existing HTTP sandbox routes. The running engine currently has only an in-memory, simulated provider; its HyperSwitch runtime, persistent captures/refunds, market/merchant settlement account binding and authenticated per-processor authority are incomplete. Therefore LA-05E is a candidate execution guard only; **provider publication must stay unsupported, staging and production money movement must stay disabled**.

Completion requires routing every real `create/confirm/capture/refund` money-moving HTTP handler through this guard, a workload-authenticated real CP client using the principal-owned PlatformContext, token/registry approval, a concrete merchant/legal entity/profile/settlement readiness adapter, durable idempotency/storage, HyperSwitch integration and end-to-end revocation tests. A legacy `PaymentContext.legal_entity_id` or HyperSwitch merchant alias cannot substitute for the current mandate.

No company account or beneficiary is automatically provisioned or assumed.
