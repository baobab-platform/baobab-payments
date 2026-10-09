# LA-05E — Legal responsibility PEP for Payments (candidate)

Accepted ADR-BCP-027 LA-05, after Shared #261 and CP #299.

The Rust `src/legal_actor.rs` defines an opt-in legal-actor assessment and execution gate for payment capture, beneficiary disbursement and contractual settlement. It requires a canonical CP `RUNTIME` context handle owned by the authenticated workload, exact `PAYMENT_BENEFICIARY` or `CONTRACTING_PARTY` role, operation reference, market, capability and an independently established expected LegalEntity. Before any irreversible callback executes it checks a fresh `AUTHORIZED` CP response, a matching responsible legal entity, current verification evidence, a bounded validity lease and a **separate positive PSP/merchant readiness decision**. A rejected/expired/ambiguous CP response, unauthorized beneficiary, absent PSP readiness or unavailable CP must not call the provider; replay is assessed again.

The request explicitly does not include tenant, Organisation, provider/legal actor or mandate selection: CP derives them from the issuer-subject-owned runtime context.

**Rollout status:** this is **not** production wiring. The existing payments engine uses an in-memory state store and a simulated sandbox provider. `PaymentContext.legal_entity_id` is historic routing context, NOT proof of entitlement. Do not connect this guard to a live HyperSwitch capture, declare any `payments.*` operation production-supported, or enable production until:
1. The actual server mutation handlers and provider implementations instantiate and call this guarded PEP on every irreversible operation, including retries;
2. The engine's authenticated workload can obtain a scoped, current `legal-actor:assess` CP token for the exact principal that owns `context_id`;
3. The merchant/entity/market/payment-beneficiary readiness adapter is implemented with independently verified HyperSwitch settings and KYC, and the canonical payment obligation is bound to the same operating business;
4. Durable idempotency, outbox, and PSP results are established; currently the engine is sandbox-only.

No Nabhold or subsidiary legal beneficiary is inferred or enrolled. PEO-02/03 and LA-06 remain independent onboarding work.

**Status of this seam:** an execution guard only. Provider publication must stay unsupported and staging/production money movement disabled. Completion also requires routing every real `create/confirm/capture/refund` HTTP handler through `execute_with_current_payment_actor`, a workload-authenticated CP client using the principal-owned RUNTIME context, and end-to-end revocation tests. No company account or beneficiary is provisioned or inferred.
