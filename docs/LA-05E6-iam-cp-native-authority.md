# LA-05E6 — IAM-authenticated CP native payment authority

Adds HTTPS client-credentials IAM token retrieval (no bearer token from request body), CP runtime Context issuance `POST /v1/platform-context/resolve` for the SAME caller identity, and current CP legal-actor `POST /internal/legal-actor/v1/assess` and documentary restriction `POST /internal/documentary/v1/restrictions/assess` assessment. Every CP response is bounded to Context, operating Organisation, exact canonical operation and current receipt time; negative-only documentary clearance is never treated as positive PSP permission. Only `payments.intent.create` may reach HyperSwitch native Create.

CP legal mandate and documentary clearance are independently refreshed twice before reservation and twice after durable PostgreSQL journal commit and before PSP dispatch, verifying the same mandate and independent `PaymentProviderLegalReadiness`. A timeout, token error, revoked actor, hold, mismatched operation or missing readiness denies sending and leaves any already-reserved intent for reconciliation. No automatic money resend.

The new `PaymentService::create_current_cp_intent` is an internal opt-in adapter, not installed by public HTTP routes or production `main.rs`; `NativeTransactionService::local_integration` still requires loopback PostgreSQL NoTls and local/test/integration. No real legal registration evidence, issuer scope, merchant certification, bank settlement proof, PostgreSQL TLS, production CP route nor AWS account details are created by this PR.

Protected acceptance requires real IAM/CP workloads, merchant certification and settling bank account, PostgreSQL mTLS and PSP retry/webhook reconciliation, operational observability and independently signed staging evidence.

## P0-04A contract follow-up — 13 October 2026

Actual request/response contracts and their complete reference closure are now
pinned and embedded at Shared `a9c53087`. Runtime format/closed-shape validation
runs before existing consequential binding, expiry and provider checks, including
injected policy ports. This closes contract drift; it does not activate an IAM
workload or certify native money movement. See the [maintenance/audit plan](plans/P0-04A-payments-contract-convergence.md).
