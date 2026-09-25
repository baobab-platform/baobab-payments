# ADR-PAY-0003 — Tenant / Organisation Mapping

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Tenancy / Context Mapping |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001; ADR-PAY-0002; applicable Control Plane organisation, tenancy, context and isolation ADRs; ADR-SHARED-011 |
| **Related** | ADR-PAY-0004; ADR-PAY-0005; ADR-PAY-0009; ADR-PAY-0010; ADR-PAY-0012; ADR-PAY-0018; ADR-PAY-0022 |

---

# 1. Context

ADR-PAY-0001 establishes HyperSwitch as the foundational payment-orchestration engine behind the Baobab Payments façade.

ADR-PAY-0002 establishes that Baobab's canonical business context remains outside HyperSwitch and that `baobab-payments` consumes rather than originates canonical:

- tenants;
- organisations;
- legal entities;
- markets;
- digital estates;
- engine instances;
- and isolation context.

Payment orchestration systems nevertheless maintain their own organisational structures for purposes such as:

- merchant administration;
- connector configuration;
- credentials;
- business profiles;
- routing;
- operational isolation;
- reporting.

Those structures exist for payment-processing purposes.

They are not necessarily equivalent to Baobab's canonical organisational model.

Baobab must support organisational structures such as:

```text
Baobab Platform
│
├── Nabhold Group Africa
│   │
│   ├── ZuriBeans
│   ├── Thamani Global
│   └── Equator & Estate Co.
│
└── External Customer A
    │
    ├── Subsidiary A
    └── Subsidiary B
```

These relationships carry business meaning independent of payment processing.

A payment engine must therefore not infer that a HyperSwitch organisational object represents a Baobab tenant, corporate group, legal entity, subsidiary or other canonical organisation.

This ADR defines how Baobab tenant and organisation context maps into the payment engine without surrendering canonical authority.

---

# 2. Problem

A simplistic implementation could establish mappings such as:

```text
Baobab Tenant
      =
HyperSwitch Organisation
```

or:

```text
Baobab Organisation
      =
HyperSwitch Organisation
```

Such mappings are unsafe because the concepts belong to different domains.

Baobab tenancy concerns:

```text
platform isolation
customer boundary
capability consumption
governance
data isolation
commercial relationship with Baobab
```

while payment-engine organisational structures primarily concern:

```text
payment administration
merchant organisation
connector configuration
routing
payment operations
```

The concepts may occasionally have a one-to-one operational mapping.

That coincidence SHALL NOT establish semantic identity.

---

# 3. Decision

Baobab SHALL maintain its own canonical Tenant and Organisation identities independently of HyperSwitch.

HyperSwitch organisational objects SHALL be treated as external operational representations.

The governing relationship is:

```text
Baobab Canonical Context
        │
        ▼
Payment Mapping
        │
        ▼
HyperSwitch Operational Context
```

never:

```text
HyperSwitch Context
        │
        ▼
derive Baobab organisation
```

Therefore:

```text
HyperSwitch Organisation
        ≠
Baobab Tenant
```

and:

```text
HyperSwitch Organisation
        ≠
Baobab Organisation
```

unless an explicit mapping exists — and even then the objects remain semantically distinct.

---

# 4. Governing Principle

> **Baobab organisational identity is canonical. HyperSwitch organisational identity is an external payment-engine representation. Mapping connects the two; mapping never merges their authority.**

---

# 5. Canonical Hierarchy

Payments SHALL consume the organisational context established by the Control Plane.

Conceptually:

```text
PlatformAccount / Customer Context
             │
             ▼
          Tenant
             │
             ▼
        Organisation
             │
       ┌─────┴─────┐
       ▼           ▼
Organisation   Organisation
       │
       ▼
 Legal Entity
       │
       ▼
    Market
```

The exact corporate hierarchy remains governed by the Control Plane ADRs.

Payments SHALL NOT recreate this graph internally.

---

# 6. Tenant

A **Tenant** is a canonical Baobab isolation and platform-consumption boundary.

It is not fundamentally a payment concept.

Tenant identity may affect:

- isolation;
- capability resolution;
- provider configuration;
- credentials;
- payment data access;
- routing eligibility;
- audit;
- observability;
- operational policy.

Payments therefore requires tenant context.

Payments does not own Tenant lifecycle.

---

# 7. Organisation

An **Organisation** represents a canonical organisational actor or structural entity within Baobab's governed organisation model.

An Organisation may represent, according to Control Plane semantics:

```text
corporate group
company
subsidiary
business organisation
external customer organisation
other governed organisational actor
```

Payments SHALL reference canonical organisation identities where payment policy requires them.

It SHALL NOT independently interpret corporate ownership.

---

# 8. Legal Entity Remains Distinct

Organisation and Legal Entity SHALL remain distinct concepts.

A corporate organisation may contain:

```text
one legal entity
```

or potentially:

```text
multiple legal entities
```

depending on the canonical organisational model.

Payment merchant relationships are frequently legal-entity-specific.

Therefore this ADR intentionally does **not** map Legal Entity to HyperSwitch Merchant.

That responsibility belongs to:

**ADR-PAY-0004 — Merchant / Legal Entity / Profile Mapping.**

---

# 9. HyperSwitch Organisation

A HyperSwitch Organisation SHALL be treated as an operational payment-orchestration object.

Its purpose may include grouping payment resources and administrative configuration.

It SHALL NOT automatically imply:

```text
Baobab tenant
corporate parent
subsidiary
legal entity
customer organisation
platform account
```

The meaning of a HyperSwitch Organisation exists within the HyperSwitch operational domain.

---

# 10. Mapping Model

Baobab SHALL maintain explicit mapping records between canonical and payment-engine identities.

Conceptually:

```text
PaymentContextMapping
│
├── mapping_id
│
├── tenant_id
├── organisation_id
│
├── engine
├── engine_instance_id
│
├── external_object_type
├── external_object_id
│
├── mapping_scope
├── environment
│
├── status
├── effective_from
├── effective_to
│
├── created_at
├── created_by
└── revision
```

The exact schema SHALL align with canonical Control Plane mapping contracts rather than creating an incompatible Payments-only mapping system.

---

# 11. Existing Canonical Mapping Infrastructure

Where Baobab already provides canonical:

```text
ExternalReference
Mapping
MappingScope
EngineInstance
Context
```

concepts, Payments SHALL use them.

`baobab-payments` SHALL NOT introduce a competing generic external-reference framework.

Payment-specific records MAY reference or project canonical mappings where required for efficient execution.

The authoritative mapping relationship remains governed through Baobab's canonical mapping architecture.

---

# 12. Mapping Direction

A mapping SHALL be understood conceptually as:

```text
CanonicalEntity
       │
       ▼
     Mapping
       │
       ▼
ExternalReference
       │
       ▼
HyperSwitch Object
```

For example:

```text
Baobab Organisation
      org_abc
         │
         ▼
      Mapping
         │
         ▼
HyperSwitch Organisation
      hs_org_xyz
```

The mapping does not mean:

```text
org_abc == hs_org_xyz
```

It means:

> `hs_org_xyz` is an external operational representation associated with `org_abc` for a defined payment-engine scope.

---

# 13. Mapping Scope

Mappings SHALL be scoped sufficiently to prevent ambiguity.

Scope MAY include:

```text
tenant
organisation
legal entity
market
engine
engine instance
environment
region
```

as required.

At minimum, HyperSwitch mappings SHALL distinguish:

```text
engine instance
environment
```

because:

```text
Sandbox HyperSwitch
```

and:

```text
Production HyperSwitch
```

must not share assumed identity.

---

# 14. Environment Isolation

A production mapping SHALL NOT be inferred from a sandbox mapping.

For example:

```text
Baobab Organisation
       │
       ├──── SANDBOX ───► hs_org_test_123
       │
       └──── PRODUCTION ► hs_org_prod_789
```

These are distinct mappings.

Therefore:

```text
sandbox external ID
        ≠
production external ID
```

even where both represent the same canonical Baobab organisation.

---

# 15. Engine Instance Awareness

Baobab may eventually operate more than one payment-engine instance because of:

```text
region
data residency
customer isolation
availability topology
regulatory requirements
scale
migration
```

Mappings SHALL therefore support:

```text
Canonical Organisation
        │
        ├── HyperSwitch Instance A
        │       └── external organisation A
        │
        └── HyperSwitch Instance B
                └── external organisation B
```

No assumption SHALL exist that one canonical organisation has exactly one global HyperSwitch organisation identifier.

---

# 16. Cardinality

The architecture SHALL support mapping cardinalities beyond strict one-to-one relationships.

Possible relationships include:

```text
1 canonical organisation
        │
        └── 1 HyperSwitch organisation
```

but MAY also require:

```text
1 canonical organisation
        │
        ├── HyperSwitch organisation A
        └── HyperSwitch organisation B
```

for different:

```text
environments
regions
engine instances
operational partitions
```

Conversely, several canonical organisational contexts SHALL NOT be collapsed onto one external payment organisation unless explicitly justified, isolated and approved.

---

# 17. No Hierarchy Inference

Payments SHALL NOT infer corporate relationships from HyperSwitch hierarchy.

For example:

```text
HyperSwitch Organisation A
        │
        └── Merchant B
```

does not establish:

```text
Organisation A owns Legal Entity B
```

in Baobab.

Similarly, shared provider configuration does not prove corporate affiliation.

Canonical corporate relationships remain governed by the Control Plane.

---

# 18. No Tenant Inference

Payments SHALL never infer tenant identity from:

```text
HyperSwitch organisation ID
merchant ID
profile ID
connector account
provider credential
processor reference
```

Tenant identity SHALL originate from authenticated and resolved Baobab context.

---

# 19. Request Context

Every payment command SHALL arrive with or resolve to trusted Baobab context.

Conceptually:

```text
Payment Command
      │
      ▼
Authenticated Workload
      │
      ▼
Resolved Baobab Context
      │
      ├── tenant_id
      ├── organisation_id where applicable
      ├── legal_entity_id
      ├── market_id
      ├── engine_instance_id
      └── source context
      │
      ▼
Payments Mapping Resolution
      │
      ▼
HyperSwitch Context
```

User-supplied external IDs SHALL NOT establish authority.

---

# 20. Context Precedence

When canonical context and provider metadata disagree:

```text
Canonical Baobab Context
```

takes precedence for business authority.

Example:

```text
Authenticated Context:
tenant = tenant_A

Request body:
hyperSwitchOrganisation = organisation_B
```

shall not cause Payments to switch tenant context.

The request SHALL be rejected unless the external identifier is validly mapped within the authoritative context.

---

# 21. Fail-Closed Resolution

If Payments cannot resolve the required mapping unambiguously, execution SHALL fail closed.

For example:

```text
tenant known
organisation known
legal entity known

but

HyperSwitch organisation mapping missing
```

shall produce:

```text
PAYMENT CONTEXT NOT READY
```

rather than guessing an external organisation.

No fallback SHALL search for a similarly named HyperSwitch object.

---

# 22. Names Are Not Identity

Human-readable names SHALL NOT establish mappings.

For example:

```text
Baobab:
"ZuriBeans"

HyperSwitch:
"ZuriBeans"
```

does not prove that the two objects correspond.

Mappings SHALL use immutable identifiers.

Names are display metadata only.

---

# 23. Tenant Isolation

Payment records SHALL always preserve canonical tenant context.

At minimum, canonical payment aggregates SHALL remain attributable to:

```text
tenant_id
```

and the appropriate legal-entity/business context.

Queries SHALL enforce tenant isolation.

A valid provider reference SHALL NOT grant cross-tenant access.

---

# 24. Organisation Isolation

Where organisation context affects access or governance, payment records SHALL preserve sufficient organisation attribution.

However, organisation hierarchy SHALL not replace tenant isolation.

Conceptually:

```text
Tenant
  │
  ├── Organisation A
  │
  └── Organisation B
```

Organisation A and B may share a tenant while remaining distinct governed organisations.

Payment access policy SHALL respect the relevant canonical context.

---

# 25. External Customers

The mapping architecture SHALL work identically for future external Baobab customers.

Example:

```text
Baobab Platform
│
├── Tenant: Nabhold
│   ├── ZuriBeans
│   └── Thamani
│
└── Tenant: ExternalCustomer
    ├── Subsidiary A
    └── Subsidiary B
```

Payments SHALL not contain special architecture such as:

```text
if tenant == NABHOLD
```

for ordinary tenant mapping.

Nabhold is a customer context, not a privileged hard-coded tenancy model.

---

# 26. Group Relationships

Corporate group membership SHALL NOT imply shared payment identity.

For example:

```text
Nabhold Group Africa
        │
        ├── ZuriBeans
        └── Thamani Global
```

does not imply:

```text
same merchant
same provider credentials
same settlement account
same HyperSwitch profile
same payment configuration
```

Those decisions belong to the legal-entity and provider activation layers.

---

# 27. Shared Payment Infrastructure

Multiple tenants or organisations MAY consume the same physical:

```text
baobab-payments service
HyperSwitch deployment
PostgreSQL cluster
Redis infrastructure
```

where architecture permits.

Shared infrastructure SHALL NOT imply shared authority.

Logical isolation SHALL continue through canonical context, mapping and policy.

---

# 28. Dedicated Payment Infrastructure

Baobab MAY also assign:

```text
dedicated payment engine instance
```

to a tenant or customer where required by:

```text
regulation
contract
data residency
risk
scale
isolation
```

The canonical payment contracts SHALL remain unchanged.

Only engine resolution and mappings change.

---

# 29. Mapping Lifecycle

Mappings SHALL have lifecycle.

Representative states include:

```text
PENDING
ACTIVE
SUSPENDED
SUPERSEDED
RETIRED
```

A mapping SHALL NOT be physically deleted merely because a newer mapping replaces it if historical payments depend upon the old mapping.

---

# 30. Historical Mapping Integrity

Suppose:

```text
2026:
Baobab Organisation A
    → HyperSwitch Organisation X

2028:
Baobab Organisation A
    → HyperSwitch Organisation Y
```

Historical payments executed through X SHALL remain attributable to X.

The new mapping SHALL NOT rewrite historical provider identity.

---

# 31. Mapping Mutation

Mapping changes SHALL be controlled operations.

They MAY affect:

```text
money movement
provider routing
merchant configuration
settlement
reconciliation
```

Therefore mapping mutations SHALL require appropriate privileged authorisation.

PAY-0018 defines the detailed controlled-mutation model.

---

# 32. Mapping Audit

Every mapping mutation SHALL record enough information to answer:

```text
What canonical object was mapped?

To which external object?

In which engine instance?

In which environment?

Who created or changed it?

When?

Why?

What mapping preceded it?

Was it automatically provisioned or manually governed?
```

---

# 33. Provisioning

Payment mapping creation SHOULD normally occur through governed provisioning.

Conceptually:

```text
Control Plane
      │
      ▼
Tenant / Organisation provisioned
      │
      ▼
Payment capability required
      │
      ▼
Payments provisioning
      │
      ▼
HyperSwitch object created/resolved
      │
      ▼
ExternalReference established
      │
      ▼
Mapping activated
```

Manual creation directly in HyperSwitch SHALL NOT automatically establish Baobab mapping authority.

---

# 34. Idempotent Provisioning

Provisioning SHALL be idempotent.

Repeated provisioning requests for the same canonical context SHALL NOT create uncontrolled duplicate HyperSwitch organisational objects.

Conceptually:

```text
EnsurePaymentOrganisation(context)
```

rather than:

```text
CreatePaymentOrganisationEveryTime(context)
```

Provisioning SHALL first resolve existing authoritative mapping.

---

# 35. Duplicate Detection

Where an external object exists without a canonical mapping, Baobab SHALL NOT automatically adopt it based solely on matching names.

The condition SHALL be treated as:

```text
UNMAPPED EXTERNAL RESOURCE
```

and reconciled through controlled governance.

---

# 36. Orphan Detection

Baobab SHOULD detect:

```text
external payment organisation
without canonical mapping
```

and:

```text
canonical active payment mapping
whose external object no longer exists
```

These are configuration drift conditions.

---

# 37. Drift Detection

Representative drift includes:

```text
Mapping says:
org_A → hs_org_X

HyperSwitch:
hs_org_X deleted
```

or:

```text
Mapping:
PRODUCTION → hs_org_X

HyperSwitch:
hs_org_X exists only in sandbox
```

or:

```text
Baobab mapping:
SUSPENDED

HyperSwitch:
organisation still operationally routable
```

Drift SHALL be observable and remediable.

---

# 38. Deprovisioning

Deprovisioning SHALL distinguish:

```text
stop new payment activity
```

from:

```text
destroy payment history
```

Historical mappings may remain necessary for:

- refunds;
- disputes;
- settlement;
- reconciliation;
- audit;
- legal retention.

Therefore deprovisioning SHALL normally retire or suspend mappings rather than destroy them immediately.

---

# 39. Organisation Reorganisation

Corporate restructuring SHALL NOT silently mutate payment history.

For example:

```text
Subsidiary A
transferred from
Group X
to
Group Y
```

does not mean historical payments now belonged to Group Y.

Canonical organisational history remains governed by Control Plane records.

Payments retains the context valid at execution time.

---

# 40. Tenant Migration

If an organisational entity moves between tenant contexts under a governed migration, payment mappings SHALL be explicitly evaluated.

They SHALL NOT automatically follow the organisation.

Migration may require:

```text
new provider contracts
new merchant identity
new credentials
new HyperSwitch organisation
new settlement account
new market activation
```

The migration process SHALL preserve historical transaction ownership.

---

# 41. Merger and Acquisition

Payment mapping SHALL tolerate organisational events such as:

```text
acquisition
merger
divestiture
spin-off
subsidiary transfer
```

without changing the canonical meaning of historical payments.

A new corporate relationship is not permission to rewrite past payment context.

---

# 42. Tenant Termination

Tenant termination SHALL prevent new payment execution according to governed lifecycle rules.

It SHALL NOT necessarily remove access required for:

```text
refunds
disputes
settlement
reconciliation
regulatory retention
audit
```

Termination therefore requires coordinated lifecycle handling rather than simple external-resource deletion.

---

# 43. Security

Mapping resolution SHALL operate only on trusted canonical context.

External identifiers SHALL be treated as untrusted when supplied by callers.

Payments SHALL validate:

```text
tenant context
mapping ownership
engine instance
environment
mapping status
```

before using an external object.

---

# 44. Cross-Tenant Identifier Attack

The following attack SHALL fail:

```text
Tenant A workload
      │
      ▼
submits HyperSwitch ID
belonging to Tenant B
```

Payments SHALL resolve:

```text
Tenant A
+
requested external identifier
```

and detect that the mapping does not belong to the authenticated context.

The operation SHALL be rejected and SHOULD generate security telemetry.

---

# 45. Database Constraints

Where practical, persistence SHALL enforce mapping uniqueness appropriate to its scope.

Conceptually:

```text
canonical_entity
+
engine_instance
+
environment
+
mapping_scope
```

SHOULD prevent unintended duplicate active mappings.

Likewise, an external identifier SHOULD NOT simultaneously map to incompatible active canonical contexts.

Exact constraints SHALL follow the canonical Mapping contract.

---

# 46. Cache Behaviour

Mapping resolution MAY be cached for performance.

However:

- cache keys SHALL include authoritative scope;
- tenant boundaries SHALL be preserved;
- environment boundaries SHALL be preserved;
- invalidation SHALL occur after mapping mutation;
- stale mappings SHALL not remain indefinitely usable.

Sensitive provider credentials SHALL not be embedded into ordinary mapping cache entries.

---

# 47. Events

Relevant canonical events MAY include:

```text
payment.mapping.created.v1
payment.mapping.activated.v1
payment.mapping.suspended.v1
payment.mapping.superseded.v1
payment.mapping.retired.v1
payment.mapping.drift_detected.v1
```

Where generic Control Plane mapping events already provide the required semantics, Payments SHOULD consume those rather than duplicate them.

Event design SHALL avoid establishing competing mapping authority.

---

# 48. Observability

Every provider-bound payment operation SHOULD permit correlation across:

```text
tenant_id
organisation_id
legal_entity_id
market_id
payment_intent_id
payment_id
engine_instance_id
external organisation ID
merchant ID
profile ID
provider reference
correlation_id
```

Logs SHALL not expose credentials or sensitive payment data.

Detailed observability requirements belong to PAY-0019.

---

# 49. Readiness

Payment readiness SHALL verify that required mappings exist before production activation.

Conceptually:

```text
PAYMENT CONTEXT READINESS

Tenant resolved                         PASS
Organisation resolved                   PASS
Legal Entity resolved                   PASS
Payment Engine Instance resolved        PASS
HyperSwitch Organisation mapped         PASS
Merchant mapping                        PASS
Market/Profile mapping                  PASS
Provider activation                     PASS
--------------------------------------------
Payment context                          READY
```

If a mandatory mapping is missing:

```text
Payment context = BLOCKED
```

not partially ready.

---

# 50. Relationship to PAY-0004

This ADR defines:

```text
Tenant
Organisation
        │
        ▼
HyperSwitch Organisation mapping
```

PAY-0004 SHALL define:

```text
Legal Entity
        │
        ▼
HyperSwitch Merchant
        │
        ▼
HyperSwitch Business Profile
```

The two ADRs deliberately separate corporate context from merchant/payment-processing context.

---

# 51. Relationship to PAY-0005

PAY-0005 SHALL add:

```text
Market
Currency
Payment Method
```

to the context.

The resulting conceptual resolution chain becomes:

```text
Tenant
   │
Organisation
   │
Legal Entity
   │
Market
   │
Currency
   │
Payment Method
   │
   ▼
Eligible Payment Context
```

No single HyperSwitch hierarchy SHALL replace this canonical resolution.

---

# 52. Relationship to PAY-0022

PAY-0022 governs whether a provider is certified and activated.

Therefore:

```text
valid tenant mapping
```

does not imply:

```text
provider authorised
```

and:

```text
provider certified
```

does not imply:

```text
tenant / organisation mapping valid
```

Both conditions must hold before production execution where applicable.

---

# 53. Mapping Resolution Flow

```text
Payment Request
      │
      ▼
Authenticate Workload
      │
      ▼
Resolve Canonical Context
      │
      ├── Tenant
      ├── Organisation
      ├── Legal Entity
      ├── Market
      └── Engine Instance
      │
      ▼
Validate Context Authority
      │
      ▼
Resolve Canonical Mapping
      │
      ├──── Missing ───► BLOCK
      │
      ▼
Resolve HyperSwitch Organisation
      │
      ├──── Missing ───► DRIFT / BLOCK
      │
      ▼
Resolve Merchant / Profile
      │
      ▼
Evaluate Provider Eligibility
      │
      ▼
Payment Routing
```

---

# 54. Authority Flow

```text
┌──────────────────────────────┐
│        CONTROL PLANE         │
│                              │
│ Tenant                       │
│ Organisation                 │
│ Corporate Relationships      │
│ Legal Entity                 │
│ Market                       │
│ Engine Instance              │
│ Canonical Mapping Context    │
└──────────────┬───────────────┘
               │
               │ authoritative context
               ▼
┌──────────────────────────────┐
│       BAOBAB PAYMENTS        │
│                              │
│ validates payment context    │
│ resolves payment mappings    │
│ owns payment execution       │
└──────────────┬───────────────┘
               │
               │ external mapping
               ▼
┌──────────────────────────────┐
│          HYPERSWITCH         │
│                              │
│ Organisation                 │
│ Merchant                     │
│ Business Profile             │
│ Connector                    │
└──────────────────────────────┘
```

Authority flows downward.

Operational provider objects do not redefine the canonical hierarchy upward.

---

# 55. Invariants

The following invariants SHALL hold:

1. A HyperSwitch Organisation is not a Baobab Tenant.
2. A HyperSwitch Organisation is not automatically a Baobab Organisation.
3. A Baobab Organisation remains canonical independently of payment infrastructure.
4. Payments does not create canonical tenants.
5. Payments does not create canonical organisations.
6. HyperSwitch does not determine corporate relationships.
7. HyperSwitch does not determine tenant ownership.
8. Mappings are explicit.
9. Names do not establish identity.
10. Sandbox mappings do not establish production mappings.
11. Mappings are engine-instance-aware.
12. Mapping resolution fails closed.
13. External identifiers cannot override authenticated canonical context.
14. Cross-tenant external identifiers are rejected.
15. Historical mappings are not rewritten after organisational change.
16. Deprovisioning does not destroy required financial history.
17. Shared infrastructure does not imply shared authority.
18. Corporate group membership does not imply shared merchant identity.
19. Mapping changes are auditable.
20. HyperSwitch remains an operational representation beneath Baobab authority.

---

# 56. Consequences

## Positive

This decision:

- preserves canonical Baobab tenancy;
- prevents HyperSwitch hierarchy leakage;
- supports Nabhold and external customers uniformly;
- supports multi-instance payment deployment;
- supports sandbox/production isolation;
- protects tenant boundaries;
- makes organisational restructuring survivable;
- improves auditability;
- supports future regional deployment;
- prevents provider identifiers becoming business identity;
- provides a stable foundation for merchant mapping.

## Negative

It requires:

- explicit mapping records;
- provisioning workflows;
- mapping lifecycle management;
- drift detection;
- external-reference management;
- additional readiness checks;
- migration handling.

These costs are accepted because implicit organisational mapping in a financial system creates unacceptable isolation and authority risks.

---

# 57. Alternatives Considered

## Map Baobab Tenant directly to HyperSwitch Organisation

Rejected.

The concepts have different semantics and lifecycle authorities.

---

## Map every Baobab Organisation one-to-one to HyperSwitch Organisation

Rejected as a universal rule.

One-to-one mapping may occur operationally, but region, environment, engine-instance and isolation requirements can require multiple external representations.

---

## Let HyperSwitch define payment tenancy

Rejected.

HyperSwitch is the payment-orchestration implementation, not Baobab's platform tenancy authority.

---

## Infer mapping by organisation name

Rejected.

Names are mutable and non-unique.

---

## Hard-code Nabhold subsidiaries

Rejected.

Baobab must support external customers and future organisational structures using the same canonical architecture.

---

## Share one HyperSwitch identity across an entire corporate group

Rejected as a default.

Group membership does not establish shared merchant contracts, settlement ownership, credentials or legal payment authority.

---

# 58. Final Decision

Baobab SHALL preserve a strict distinction between:

```text
CANONICAL BUSINESS IDENTITY
```

and:

```text
PAYMENT-ENGINE OPERATIONAL IDENTITY
```

The mapping model is:

```text
Baobab Tenant
      │
      ▼
Baobab Organisation
      │
      │ canonical authority
      ▼
Canonical Mapping
      │
      │ external representation
      ▼
HyperSwitch Organisation
```

The following equivalences are prohibited:

```text
HyperSwitch Organisation
        =
Baobab Tenant

HyperSwitch Organisation
        =
Baobab Organisation

HyperSwitch hierarchy
        =
Baobab corporate hierarchy
```

Instead:

```text
CONTROL PLANE
defines
WHO THE ORGANISATION IS

        │
        ▼

CANONICAL MAPPING
defines
HOW THAT ORGANISATION IS REPRESENTED
IN A PARTICULAR PAYMENT ENGINE INSTANCE

        │
        ▼

BAOBAB PAYMENTS
uses
THAT REPRESENTATION FOR PAYMENT EXECUTION

        │
        ▼

HYPERSWITCH
provides
THE OPERATIONAL PAYMENT ORGANISATION
```

**Baobab owns organisational truth. HyperSwitch owns its operational payment representation. The mapping connects them without confusing them.**