# LA-05E3 — Real HyperSwitch transport (non-activating increment)

**Authority:** ADR-BCP-026/027 LA-05, ADR-PAY-0001, PAY-0004, PAY-0010, PAY-0012, PAY-0015, PAY-0018, PAY-0022.

## Delivered

* `src/hyperswitch.rs` implements real bounded outbound HTTP for HyperSwitch Payments Create, Confirm, Capture, Cancel and Refund operations, using existing `reqwest` (no dependency or lockfile updates).
* A configured API key is held only server-side, never logged or rendered in Debug. Redirect following is disabled. Only HTTPS is accepted by the public constructor; local plaintext is available solely to private test code.
* Requests are time-limited, POST only, have a stable idempotency header, refuse sensitive PAN/CVV/bank-account fields, reject unsafe IDs and provider redirects, and do not automatically retry network failures.
* The transport maps successful provider responses to an **observation** of provider reference/status, not to a Baobab `payment.captured` or settled accounting event. Failures, unrecognised payloads, large/malformed responses and unavailable providers fail closed.
* In-process tests issue actual loopback HTTP, inspect outbound headers/body, and cover credentials, idempotency, invalid responses, untrusted base URLs and errors.

## Deliberate safety boundary

**No production payment authority is activated.** `PaymentService` still uses the synthetic in-memory sandbox; `Config` continues to refuse staging and production. The authenticated HTTP money-changing routes still return `PAYMENT_EXECUTION_NOT_CERTIFIED` in protected environments. No merchant mappings, legal-actor mandates, banking beneficiaries, settlement account, PSP certification or live credentials are asserted.

The module is a transport adapter, **not** a permitted production PaymentProvider. A future integration must only call it inside a durable, transaction-bound, fresh IAM+CP/LegalActor+documentary+independent-PSP-readiness PEP. It must persist an idempotent operation and pending provider outcome **before** the request, reconcile ambiguous timeouts by remote reference, and publish immutable outbox facts after financial acceptance. Historical actors must not be rewritten.

## Additional coding gates before LA-05 Payments completion

1. PostgreSQL 17 transactional finance state, outbox, strict tenant isolation, idempotency and timeout reconciliation.
2. HyperSwitch merchant/profile routing from explicit CP/Payments verified provider mappings; do not trust request-supplied provider IDs.
3. Authenticated client-credentials CP legal-actor + documentary assessment on every protected mutation, with issuer and beneficiary separation.
4. Accepted PSP certification, settlements, chargebacks/refunds, webhook signatures, duplicate/out-of-order delivery and ERP reconciliation.
5. Protected staging integration against deployed HyperSwitch + PostgreSQL + Redis + Superposition, with approved credentials and positive/negative controls.

These remain unfinished; in particular no code change here should be interpreted as green operational acceptance.
