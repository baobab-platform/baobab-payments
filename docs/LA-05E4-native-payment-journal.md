# LA-05E4 — PostgreSQL 17 durable payment operations and provider-evidence journal

**Authority:** ADR-BCP-026/027 LA-05, ADR-PAY-0004/0010/0011/0015/0022.

This is *executable database functionality* and a PostgreSQL 17 security test, not a production payments integration. It creates zero business rows on migration.

## Implemented native database controls

- Durable immutable `payment_ops.native_execution` operation reservations indexed by tenant, legally responsible actor, operation type and idempotency key (ADR-PAY-0010).
- A SECURITY INVOKER `reserve_native_execution` function enforces exact replay equivalence for body SHA-256, historical legal actor, organisation, mandate reference, certification/activation references, market/currency, amount and payment reference; mismatched replays refuse.
- Append-only, provider-event-unique (provider + EngineInstance + environment + event ID) `provider_receipt` and tenant-scoped, immutable `transaction_outbox` tables bound by composite foreign keys to the originating operation.
- PostgreSQL FORCE ROW LEVEL SECURITY with fail-closed **tenant and responsible legal actor** session scope (`baobab.tenant_id`, `baobab.responsible_legal_entity_id`) on all three tables. Unprivileged clients missing tenant scope see no records and cannot insert. Row rewrite/deletion triggers reject reassignment of historical actor or PSP receipt.
- A real ephemeral PostgreSQL 17 integration workflow verifies replay, conflict, history immutability and cross-tenant isolation.

## Production blockers remain

**No database column asserting a 'certification reference' makes it certified.** The caller must cryptographically authenticate and verify current CP mandate, operation-specific documentary scope, independent PSP certification/merchant activation, settlement beneficiary and source obligation before invoking this database function. The current Payments Rust service does **not** yet use this schema and is deliberately not activated on staging/production.

Required follow-on increments: wired Rust PostgreSQL transactional repository, atomic PSP send/reconcile + terminal outcome replay, outbox publisher with delivery cursor, IAM+CP workload-bound PEP, official HyperSwitch merchant/profile configuration, PSP webhook signature and settlement/ERP reconciliation. The repository uses a sandbox provider until these are implemented and certified. No real business IDs or provider credentials are seeded.

## Operational requirements

Use an unprivileged application role with no `BYPASSRLS`; set both `baobab.tenant_id` and `baobab.responsible_legal_entity_id` inside a bounded transaction *only after authenticated tenant resolution*. Verify actual PostgreSQL 17 roles and denied direct access in protected staging. Never permit customer-supplied tenant scope, SQL GUC overrides, certification state or actor reference.
