# ADR-PAY-0004 — Merchant / Legal Entity / Profile Mapping

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Merchant Identity / Legal-Entity Mapping |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001; ADR-PAY-0002; ADR-PAY-0003; applicable Control Plane organisation, legal-entity, context, mapping and isolation ADRs; ADR-SHARED-011 |
| **Related** | ADR-PAY-0005; ADR-PAY-0006; ADR-PAY-0007; ADR-PAY-0009; ADR-PAY-0012; ADR-PAY-0015; ADR-PAY-0017; ADR-PAY-0018; ADR-PAY-0022 |

---

# 1. Context

ADR-PAY-0001 establishes HyperSwitch as the foundational payment-orchestration implementation behind `baobab-payments`.

ADR-PAY-0002 establishes Baobab's canonical payment domain and authority boundaries.

ADR-PAY-0003 establishes that:

```text
HyperSwitch Organisation
        ≠
Baobab Tenant

HyperSwitch Organisation
        ≠
Baobab Organisation
```

and requires explicit mappings between canonical Baobab organisational context and payment-engine representations.

The next layer concerns the party on whose behalf payment processing actually occurs.

Payment processors commonly organise execution around concepts such as:

- merchant;
- merchant account;
- business profile;
- connector account;
- processor account;
- settlement account;
- acquiring relationship.

Baobab separately maintains canonical concepts such as:

- Organisation;
- Legal Entity;
- Market;
- Digital Estate;
- Engine Instance;
- Provider Activation.

These concepts overlap operationally but are not semantically interchangeable.

For example, ZuriBeans and Thamani Global may both:

```text
belong to the same corporate group
consume the same Baobab Platform
use the same baobab-payments engine
use the same HyperSwitch deployment
operate in the same market
use the same payment provider
```

while still being independent legal entities with potentially different:

```text
merchant agreements
provider credentials
settlement accounts
regulatory obligations
transaction descriptors
risk settings
reconciliation streams
```

Baobab therefore requires an explicit mapping architecture between canonical Legal Entity context and HyperSwitch Merchant / Business Profile structures.

---

# 2. Problem

A simplistic implementation could assume:

```text
Baobab Legal Entity
        =
HyperSwitch Merchant
```

and:

```text
Baobab Market
        =
HyperSwitch Business Profile
```

This would be unsafe.

A HyperSwitch Merchant is an operational payment-engine object.

A Baobab Legal Entity is a canonical business/legal identity.

Likewise, a Business Profile may be useful for representing a payment-processing configuration associated with a market, channel or business configuration, but it is not itself the canonical Market.

The mapping may frequently appear one-to-one.

That coincidence SHALL NOT establish semantic identity.

---

# 3. Decision

Baobab SHALL preserve independent canonical Legal Entity identity.

HyperSwitch Merchant and Business Profile objects SHALL be treated as external operational payment representations.

The governing model is:

```text
Baobab Organisation
        │
        ▼
Baobab Legal Entity
        │
        │ canonical authority
        ▼
Payment Mapping
        │
        ▼
HyperSwitch Merchant
        │
        ▼
HyperSwitch Business Profile
```

The following equivalences are prohibited:

```text
HyperSwitch Merchant
        =
Baobab Legal Entity
```

and:

```text
HyperSwitch Business Profile
        =
Baobab Market
```

Instead, explicit scoped mappings SHALL connect the domains.

---

# 4. Governing Principle

> **The Legal Entity defines who is conducting the business. The Merchant represents how that entity participates in the payment engine. The Business Profile represents a scoped payment-processing configuration. None replaces the canonical Baobab identity or market model.**

---

# 5. Legal Entity

A **Legal Entity** is a canonical Baobab business identity governed outside `baobab-payments`.

It represents the legally recognised entity relevant to activities such as:

```text
contracting
selling
purchasing
billing
receiving funds
incurring liabilities
taxation
financial reporting
regulatory obligations
```

Payments consumes the Legal Entity identity.

Payments SHALL NOT create canonical Legal Entities.

---

# 6. Legal Entity Is Fundamental to Payment Execution

Every production payment SHALL be attributable to the legal entity on whose authority payment processing occurs.

This is necessary because the legal entity may determine:

```text
provider contract
merchant account
processor credentials
settlement destination
transaction descriptor
market eligibility
payment-method eligibility
regulatory requirements
reconciliation ownership
accounting destination
```

Consequently, legal-entity context SHALL be established before production payment routing.

---

# 7. Merchant

A **Merchant** is an operational payment-engine representation used for payment processing.

Within HyperSwitch it SHALL be treated as an external payment resource.

It may carry or reference operational configuration associated with:

```text
payment processing
business profiles
connectors
routing
payment methods
provider relationships
```

A Merchant does not become the canonical legal identity.

---

# 8. Merchant Mapping

The conceptual relationship SHALL be:

```text
Baobab Legal Entity
       │
       ▼
Canonical Mapping
       │
       ▼
HyperSwitch Merchant
```

For example:

```text
legal_entity_zuribeans
       │
       ▼
mapping
       │
       ▼
hs_merchant_abc
```

The relationship means:

> `hs_merchant_abc` is an operational payment representation associated with `legal_entity_zuribeans` for a defined scope.

It does not mean:

```text
legal_entity_zuribeans == hs_merchant_abc
```

---

# 9. Merchant Mapping Scope

Merchant mappings SHALL be scoped.

Relevant dimensions MAY include:

```text
tenant
organisation
legal entity
payment engine instance
environment
region
market
provider arrangement
```

At minimum the mapping SHALL distinguish:

```text
legal entity
engine instance
environment
```

where these dimensions can affect operational identity.

---

# 10. Merchant Cardinality

Baobab SHALL NOT impose a universal:

```text
1 Legal Entity
      =
1 Merchant
```

constraint.

A simple deployment may use:

```text
Legal Entity A
      │
      ▼
Merchant A
```

but valid future architectures may require:

```text
Legal Entity A
      │
      ├── Merchant ZA
      ├── Merchant UG
      └── Merchant KE
```

or:

```text
Legal Entity A
      │
      ├── Merchant — Africa HyperSwitch Instance
      └── Merchant — EU HyperSwitch Instance
```

or:

```text
Legal Entity A
      │
      ├── Sandbox Merchant
      └── Production Merchant
```

Therefore mapping cardinality SHALL remain explicit and scoped.

---

# 11. Shared Merchant Representation

Several Legal Entities SHALL NOT share a HyperSwitch Merchant merely because they:

```text
share a parent company
share infrastructure
share a digital estate
share a payment provider
share a market
```

A shared Merchant MAY be permitted only where the actual payment/provider/legal arrangement supports it and Baobab governance explicitly approves the mapping.

Such an arrangement SHALL require architectural and operational evidence demonstrating that it does not destroy:

```text
legal-entity attribution
credential isolation
settlement ownership
reconciliation
auditability
provider contractual boundaries
```

Shared merchant identity SHALL therefore be exceptional rather than the default.

---

# 12. Group Membership Does Not Imply Merchant Sharing

For example:

```text
Nabhold Group Africa
       │
       ├── ZuriBeans
       └── Thamani Global
```

does not imply:

```text
ZuriBeans Merchant
       =
Thamani Merchant
```

even if both entities use:

```text
same provider
same currency
same market
same HyperSwitch deployment
```

Legal independence takes precedence over infrastructure convenience.

---

# 13. External Customers

The same rule applies to future Baobab customers.

For example:

```text
External Customer Group
       │
       ├── Subsidiary A
       └── Subsidiary B
```

does not imply a shared Merchant.

The architecture SHALL apply the same canonical mapping rules regardless of whether the legal entity belongs to Nabhold or an external Baobab customer.

---

# 14. Business Profile

A **Business Profile** is an operational payment-processing configuration beneath or associated with a Merchant.

It MAY represent distinctions relevant to payment execution such as:

```text
market
business line
payment channel
currency configuration
payment-method configuration
routing context
transaction descriptor
risk configuration
```

depending on supported HyperSwitch capabilities and Baobab architecture.

A Business Profile SHALL NOT become a canonical Baobab Market, Digital Estate or Legal Entity.

---

# 15. Business Profile Mapping

Conceptually:

```text
Legal Entity
     │
     ▼
Merchant
     │
     ├── Business Profile A
     ├── Business Profile B
     └── Business Profile C
```

Baobab SHALL explicitly record what canonical context each profile represents.

For example:

```text
Legal Entity: ZuriBeans
       │
       ▼
Merchant: ZuriBeans Production
       │
       ├── Profile: ZA
       │      └── mapped payment context
       │
       └── Profile: UG
              └── mapped payment context
```

This is an operational projection.

The canonical markets remain:

```text
Market ZA
Market UG
```

outside HyperSwitch.

---

# 16. Profile Cardinality

Baobab SHALL NOT impose:

```text
1 Market
    =
1 Business Profile
```

as a universal semantic rule.

A market may eventually require several profiles because of:

```text
business model
currency
channel
provider arrangement
regulation
regional infrastructure
risk configuration
```

Likewise, where operationally valid, one profile may support several compatible payment contexts.

Such mappings SHALL be explicit.

---

# 17. Market Mapping Deferred

PAY-0004 establishes that Business Profile does not equal Market.

Detailed mapping among:

```text
Market
Currency
Payment Method
Business Profile
```

belongs to:

**ADR-PAY-0005 — Market, Currency & Payment-Method Context.**

---

# 18. Digital Estate Does Not Equal Profile

The following mapping SHALL NOT be assumed:

```text
Digital Estate
      =
Business Profile
```

For example:

```text
ZuriBeans frontend
```

is not itself a HyperSwitch Business Profile.

A Digital Estate is a canonical Baobab delivery/interface context.

A Business Profile is an operational payment configuration.

Several estates may legitimately use the same payment profile where canonical context and policy permit.

---

# 19. Commerce Store Does Not Equal Merchant

A Medusa store or commerce context SHALL NOT automatically establish Merchant identity.

Therefore:

```text
Medusa Store
      ≠
HyperSwitch Merchant
```

and:

```text
Medusa Region
      ≠
HyperSwitch Business Profile
```

Any required relationship SHALL pass through Baobab canonical context.

PAY-0006 defines Trade/Medusa integration.

---

# 20. Subscription Account Does Not Equal Merchant

Likewise:

```text
Billing Account
      ≠
HyperSwitch Merchant
```

Subscriptions may originate payment obligations.

They do not determine canonical merchant identity.

PAY-0007 defines Subscription integration.

---

# 21. Canonical Resolution Chain

Production payment context SHALL conceptually resolve as:

```text
Tenant
   │
   ▼
Organisation
   │
   ▼
Legal Entity
   │
   ▼
Payment Engine Instance
   │
   ▼
Merchant Mapping
   │
   ▼
HyperSwitch Merchant
   │
   ▼
Business Profile Mapping
   │
   ▼
HyperSwitch Business Profile
```

Market and payment eligibility are then applied through PAY-0005 and PAY-0022.

---

# 22. Mapping Model

Where existing canonical Mapping and ExternalReference contracts can represent the relationship, Payments SHALL use them.

Conceptually:

```text
MerchantMapping
│
├── mapping_id
│
├── tenant_id
├── organisation_id
├── legal_entity_id
│
├── engine_instance_id
├── environment
│
├── hyperswitch_merchant_id
│
├── mapping_scope
├── status
│
├── effective_from
├── effective_to
│
├── created_by
├── created_at
└── revision
```

Profile mappings MAY additionally reference:

```text
market_id
business_profile_id
currency scope
payment-method scope
```

where appropriate.

The exact schema SHALL reuse Baobab canonical mapping infrastructure rather than duplicate it.

---

# 23. External References

HyperSwitch identifiers SHALL remain external references.

Examples:

```text
merchant_id
profile_id
merchant_connector_account_id
```

SHALL NOT replace canonical identifiers such as:

```text
legal_entity_id
market_id
```

The general rule remains:

```text
CanonicalEntity
      │
      ▼
Mapping
      │
      ▼
ExternalReference
```

---

# 24. Environment Separation

Sandbox and production Merchant identities SHALL remain separate.

Conceptually:

```text
Legal Entity A
      │
      ├── SANDBOX
      │      └── hs_merchant_test_A
      │
      └── PRODUCTION
             └── hs_merchant_prod_A
```

A sandbox Merchant SHALL never be promoted into production merely by changing configuration metadata.

---

# 25. Engine Instance Separation

Where Baobab operates multiple HyperSwitch instances:

```text
Legal Entity A
      │
      ├── Engine Instance Africa
      │       └── Merchant X
      │
      └── Engine Instance Europe
              └── Merchant Y
```

both mappings remain associated with the same canonical Legal Entity while representing different operational resources.

---

# 26. Provider Credentials

Provider credentials SHALL normally be associated with the correct operational Merchant / connector context.

Credentials SHALL NOT be stored in canonical Legal Entity records.

The relationship is:

```text
Legal Entity
      │
      ▼
Merchant Mapping
      │
      ▼
Merchant
      │
      ▼
Connector Account
      │
      ▼
Secret Reference
```

not:

```text
Legal Entity
      │
      └── raw PSP secret
```

Credential security is governed further by PAY-0012.

---

# 27. Merchant Connector Account

A provider connector account is an operational payment resource.

Conceptually:

```text
Merchant
   │
   ▼
Business Profile
   │
   ▼
Merchant Connector Account
   │
   ▼
Provider
```

The connector account SHALL NOT independently determine canonical Legal Entity or Market.

Its use must be constrained by canonical mapping and provider activation.

---

# 28. Provider Certification

A valid Merchant mapping does not imply that its connectors are production-authorised.

PAY-0022 remains authoritative for provider certification and market activation.

Therefore:

```text
Merchant mapped
```

does not mean:

```text
Provider certified
```

and:

```text
Connector configured
```

does not mean:

```text
Connector routable
```

---

# 29. Settlement Account

Settlement account ownership SHALL be attributable to the appropriate legal entity or lawful payment arrangement.

HyperSwitch or provider configuration may reference settlement details operationally.

Such configuration SHALL NOT redefine the canonical ownership of funds.

Conceptually:

```text
Payment
   │
   ▼
Provider
   │
   ▼
Settlement
   │
   ▼
Legal Entity Financial Context
   │
   ▼
ERP Reconciliation
```

PAY-0015 governs settlement and reconciliation.

---

# 30. Settlement Does Not Follow Corporate Parent by Default

The following SHALL NOT be assumed:

```text
Subsidiary payment
       │
       ▼
Parent company settlement account
```

merely because a corporate relationship exists.

Any such arrangement requires explicit lawful and governed configuration.

The Control Plane corporate graph expresses relationships.

It does not authorise movement of subsidiary funds into a parent's payment account.

---

# 31. Merchant Descriptor

Where customer-visible merchant or transaction descriptors are supported, their configuration SHALL be associated with the correct payment context.

Descriptors SHALL not be used to determine identity.

For example:

```text
descriptor = "ZURIBEANS"
```

does not prove:

```text
legal_entity_id = zuribeans
```

Descriptors are presentation/provider configuration, not canonical identity.

---

# 32. Merchant Provisioning

Merchant provisioning SHOULD occur through governed Baobab workflows.

Conceptually:

```text
Legal Entity provisioned
       │
       ▼
Payment capability requested
       │
       ▼
Resolve Payment Engine Instance
       │
       ▼
Determine Merchant requirement
       │
       ▼
Create / resolve HyperSwitch Merchant
       │
       ▼
Create canonical mapping
       │
       ▼
Create required Business Profiles
       │
       ▼
Create profile mappings
       │
       ▼
Certification / activation
       │
       ▼
READY
```

Direct manual creation inside HyperSwitch SHALL NOT itself establish canonical readiness.

---

# 33. Idempotent Provisioning

Merchant provisioning SHALL be idempotent.

Repeated provisioning commands for the same scoped canonical context SHALL first resolve existing active mappings.

The system SHALL avoid:

```text
legal_entity_A
    │
    ├── merchant_1
    ├── merchant_2
    ├── merchant_3
    └── merchant_4
```

created accidentally by repeated retries.

Where multiple Merchants are intentional, each SHALL have an explicit distinct scope.

---

# 34. Profile Provisioning

Profile provisioning SHALL likewise be idempotent and scope-aware.

A retry SHALL not create duplicate production Business Profiles for the same intended payment context.

---

# 35. Fail-Closed Mapping

If Payments cannot establish the appropriate Merchant and Business Profile unambiguously, real payment execution SHALL fail closed.

For example:

```text
tenant             = known
legal entity       = known
market             = known
merchant mapping   = missing
```

shall result in:

```text
PAYMENT CONTEXT NOT READY
```

not:

```text
select first merchant found
```

---

# 36. No Name-Based Resolution

Payments SHALL NOT resolve a Merchant because:

```text
Legal Entity name = "ZuriBeans"

and

Merchant name = "ZuriBeans"
```

Names are not identity.

Resolution SHALL use governed immutable mappings.

---

# 37. Cross-Legal-Entity Protection

A workload operating under:

```text
legal_entity_A
```

SHALL NOT execute through:

```text
merchant_B
```

unless an explicitly governed cross-entity arrangement authorises that relationship.

A caller-supplied Merchant ID cannot override canonical Legal Entity context.

---

# 38. Cross-Tenant Protection

An external Merchant reference mapped to another tenant SHALL be rejected.

Conceptually:

```text
Tenant A
   │
   ▼
Payment Request
   │
   └── merchant_id belonging to Tenant B
```

results in:

```text
REJECT
+
SECURITY TELEMETRY
```

where appropriate.

---

# 39. Business Profile Protection

The same rule applies to Business Profiles.

A caller SHALL NOT escape:

```text
legal entity
market
provider
environment
```

constraints by supplying another profile identifier.

Profile selection is server-authoritative.

---

# 40. Mapping Lifecycle

Merchant and Business Profile mappings SHALL support lifecycle states such as:

```text
PENDING
ACTIVE
SUSPENDED
SUPERSEDED
RETIRED
```

Only mappings valid for the requested execution context SHALL participate in new payment execution.

---

# 41. Historical Integrity

If a legal entity changes Merchant representation:

```text
2026:
Legal Entity A
      → Merchant X

2028:
Legal Entity A
      → Merchant Y
```

payments executed in 2026 SHALL continue to reference Merchant X.

Historical external references SHALL not be rewritten.

---

# 42. Merchant Migration

Merchant migration SHALL be treated as a controlled financial migration.

It MAY require consideration of:

```text
in-flight authorisations
captures
refunds
disputes
recurring payment references
provider tokens
settlements
reconciliation
historical reporting
```

Migration SHALL therefore not be implemented as a simple mapping replacement.

---

# 43. Profile Migration

Likewise, changing Business Profile representation SHALL preserve historical attribution.

New transactions MAY move to the new profile.

Historical transactions remain associated with the profile through which they were executed.

---

# 44. Corporate Reorganisation

If:

```text
Legal Entity A
```

moves from:

```text
Organisation X
```

to:

```text
Organisation Y
```

its historical Merchant relationships SHALL not automatically be rewritten.

The new corporate relationship may require review of:

```text
merchant contract
provider activation
settlement account
credentials
business profile
routing
```

before future payment activity proceeds.

---

# 45. Legal Entity Merger

A legal merger SHALL NOT cause Payments to silently merge Merchant identities.

For example:

```text
Legal Entity A
+
Legal Entity B
      │
      ▼
Legal Entity C
```

requires explicit migration decisions for:

```text
Merchant A
Merchant B
existing payments
refunds
disputes
settlements
tokens
provider contracts
```

Historical financial identity SHALL be preserved.

---

# 46. Legal Entity Divestiture

Divestiture SHALL likewise trigger review.

A divested subsidiary SHALL NOT continue using the former group's Merchant or provider configuration merely because the mapping still technically exists.

Corporate-relationship drift SHALL therefore be capable of blocking future payment readiness.

---

# 47. Deactivation

Merchant mapping deactivation SHALL prevent inappropriate new execution.

It SHALL NOT automatically destroy access needed for:

```text
refund
dispute
settlement
reconciliation
audit
regulatory retention
```

The distinction is:

```text
NO NEW PAYMENTS
```

versus:

```text
NO HISTORICAL PROCESSING
```

These are not equivalent.

---

# 48. Mapping Drift

Baobab SHOULD detect conditions such as:

```text
Canonical mapping:
Legal Entity A → Merchant X

HyperSwitch:
Merchant X missing
```

or:

```text
Canonical profile:
ZA → Profile X

HyperSwitch:
Profile X disabled
```

or:

```text
Canonical mapping:
Merchant X RETIRED

HyperSwitch:
Merchant X still routable
```

or:

```text
Canonical mapping:
Legal Entity A

HyperSwitch configuration:
connector associated with another merchant context
```

These are operational drift conditions.

---

# 49. Reconciliation Drift

Baobab SHOULD additionally detect discrepancies between:

```text
legal entity expected to receive funds
```

and:

```text
merchant / settlement configuration actually used
```

because such drift can create materially incorrect financial outcomes.

Where settlement ownership cannot be established confidently:

```text
PRODUCTION PAYMENT READINESS = BLOCKED
```

---

# 50. Controlled Mutation

The following operations SHALL be privileged:

```text
create merchant mapping
activate merchant mapping
change merchant mapping
suspend merchant mapping
retire merchant mapping

create profile mapping
activate profile mapping
change profile mapping
suspend profile mapping
retire profile mapping
```

Detailed IAM controls belong to PAY-0018.

---

# 51. Separation of Duties

Production merchant configuration SHOULD support separation between actors responsible for:

```text
canonical legal-entity governance
provider credentials
merchant provisioning
provider certification
payment activation
```

No ordinary application administrator SHOULD be able to redirect real payment processing to another merchant context.

---

# 52. Audit

Merchant/profile mapping changes SHALL answer:

```text
Which tenant?

Which organisation?

Which legal entity?

Which HyperSwitch Merchant?

Which Business Profile?

Which engine instance?

Which environment?

Which market scope?

Who changed it?

When?

Why?

Under which approval?

What was the previous mapping?
```

---

# 53. Events

Where payment-specific lifecycle events are required, representative events MAY include:

```text
payment.merchant_mapping.created.v1
payment.merchant_mapping.activated.v1
payment.merchant_mapping.suspended.v1
payment.merchant_mapping.superseded.v1
payment.merchant_mapping.retired.v1

payment.profile_mapping.created.v1
payment.profile_mapping.activated.v1
payment.profile_mapping.suspended.v1
payment.profile_mapping.retired.v1
```

Where canonical generic mapping events already carry the required semantics, Payments SHOULD reuse them.

---

# 54. Observability

Payment telemetry SHOULD permit correlation across:

```text
tenant_id
organisation_id
legal_entity_id
market_id
engine_instance_id
merchant_id
business_profile_id
payment_intent_id
payment_id
provider reference
correlation_id
```

Credentials and sensitive payment material SHALL NOT appear in telemetry.

---

# 55. Production Readiness

A legal entity SHALL NOT be considered payment-ready merely because a Merchant exists.

Representative readiness:

```text
LEGAL ENTITY PAYMENT READINESS

Canonical legal entity              PASS
Payment engine instance             PASS
Merchant mapping                    PASS
Merchant exists                     PASS
Business Profile mapping            PASS
Profile exists                      PASS
Market context                      PASS
Provider certification              PASS
Provider activation                 PASS
Credentials                         PASS
Settlement configuration            PASS
Reconciliation path                 PASS
-----------------------------------------
Payment context                     READY
```

A failure of a mandatory component SHALL block production payment execution.

---

# 56. ZuriBeans and Thamani Example

Conceptually:

```text
Nabhold Group Africa
       │
       ├───────────────────────────┐
       │                           │
       ▼                           ▼
   ZuriBeans                 Thamani Global
 Legal Entity                 Legal Entity
       │                           │
       ▼                           ▼
 ZuriBeans Merchant          Thamani Merchant
       │                           │
   ┌───┴───┐                   ┌───┴───┐
   ▼       ▼                   ▼       ▼
ZA Profile UG Profile       ZA Profile UG Profile
```

This is illustrative rather than a requirement that every market always have a dedicated profile.

The important property is that the subsidiaries remain independently attributable.

---

# 57. External Customer Example

```text
External Group
      │
      ├──────────────────┐
      ▼                  ▼
Subsidiary A        Subsidiary B
Legal Entity        Legal Entity
      │                  │
      ▼                  ▼
Merchant A          Merchant B
      │                  │
      ▼                  ▼
Profiles             Profiles
```

No Nabhold-specific branching is required.

---

# 58. Full Context Projection

Together, PAY-0003 and PAY-0004 establish:

```text
             BAOBAB CANONICAL DOMAIN

Tenant
  │
  ▼
Organisation
  │
  ▼
Legal Entity
  │
  ▼
Market
  │
  ▼
Payment Context

             │
             │ explicit mappings
             ▼

          PAYMENT ENGINE DOMAIN

HyperSwitch Organisation
  │
  ▼
Merchant
  │
  ▼
Business Profile
  │
  ▼
Connector / Provider
```

The two hierarchies cooperate.

They are not equivalent.

---

# 59. Resolution Flow

```text
Payment Command
      │
      ▼
Authenticate Workload
      │
      ▼
Resolve Tenant
      │
      ▼
Resolve Organisation
      │
      ▼
Resolve Legal Entity
      │
      ▼
Resolve Payment Engine Instance
      │
      ▼
Resolve HyperSwitch Organisation Mapping
      │
      ▼
Resolve Merchant Mapping
      │
      ├──── missing ────► BLOCK
      │
      ▼
Resolve Business Profile
      │
      ├──── missing ────► BLOCK
      │
      ▼
Validate Market Context
      │
      ▼
Validate Provider Activation
      │
      ▼
Determine Eligible Routing
      │
      ▼
Execute Payment
```

---

# 60. Invariants

The following invariants SHALL hold:

1. A HyperSwitch Merchant is not a Baobab Legal Entity.
2. A Business Profile is not a Baobab Market.
3. A Business Profile is not a Digital Estate.
4. A Medusa Store is not a Merchant.
5. A billing account is not a Merchant.
6. Legal Entity identity remains canonical outside Payments.
7. Merchant mapping is explicit.
8. Business Profile mapping is explicit.
9. Merchant names do not establish identity.
10. Profile names do not establish identity.
11. Sandbox Merchant mappings do not establish production mappings.
12. Merchant mappings are engine-instance-aware.
13. Legal entities do not automatically share Merchants because they share a parent.
14. Shared infrastructure does not imply shared merchant identity.
15. Provider credentials do not define legal identity.
16. Settlement configuration does not define legal identity.
17. Digital estates cannot select arbitrary Merchant identifiers.
18. Cross-tenant Merchant use is prohibited unless explicitly governed.
19. Cross-legal-entity Merchant use is prohibited unless explicitly governed.
20. Historical Merchant references remain immutable historical facts.
21. Deactivation does not erase historical payment processing.
22. Provider certification remains separate from Merchant mapping.
23. Mapping resolution fails closed.
24. HyperSwitch operational hierarchy does not redefine Baobab legal structure.

---

# 61. Consequences

## Positive

This decision:

- preserves legal-entity authority;
- prevents merchant identity leakage into canonical business identity;
- supports independent subsidiaries;
- supports external customer groups;
- enables multi-market payment operations;
- enables regional HyperSwitch deployments;
- supports provider-specific merchant arrangements;
- protects settlement ownership;
- improves reconciliation;
- supports legal-entity restructuring;
- prevents accidental cross-entity money movement;
- keeps HyperSwitch replaceable.

## Negative

It introduces:

- Merchant mappings;
- Business Profile mappings;
- lifecycle management;
- provisioning workflows;
- drift detection;
- migration requirements;
- additional readiness gates.

These costs are accepted because payment execution across the wrong legal or merchant context represents a severe financial and governance failure.

---

# 62. Alternatives Considered

## Legal Entity equals HyperSwitch Merchant

Rejected.

They belong to different authority domains.

---

## Market equals Business Profile

Rejected.

Business Profile is an operational payment configuration; Market remains canonical Baobab context.

---

## One Merchant for an entire corporate group

Rejected as the default.

Corporate affiliation does not establish common merchant contracts, credentials, settlement ownership or reconciliation.

---

## One Merchant per digital estate

Rejected.

Digital Estate is not the legal payment principal.

---

## Let Trade choose Merchant

Rejected.

Trade provides commercial context; payment merchant resolution belongs to governed Payments context.

---

## Let the frontend send Merchant/Profile IDs

Rejected as an authority mechanism.

Client-supplied operational identifiers cannot determine the legal context through which money moves.

---

## Infer Merchant from name

Rejected.

Names are neither immutable nor authoritative.

---

# 63. Relationship to Subsequent ADRs

PAY-0004 establishes the legal payment principal and its operational representation.

The subsequent architecture builds on it:

```text
PAY-0003
Tenant / Organisation
       │
       ▼
PAY-0004
Merchant / Legal Entity / Profile
       │
       ▼
PAY-0005
Market / Currency / Payment Method
       │
       ▼
PAY-0009
Routing / Eligibility
       │
       ▼
PAY-0022
Provider Certification / Activation
```

Operational execution therefore depends upon the intersection of:

```text
WHO
Legal Entity

WHERE
Market

WHAT MONEY
Currency

HOW
Payment Method

THROUGH WHOM
Certified Provider

UNDER WHICH OPERATIONAL IDENTITY
Merchant / Profile
```

---

# 64. Final Decision

Baobab SHALL model the relationship as:

```text
CANONICAL BAOBAB AUTHORITY

Tenant
   │
   ▼
Organisation
   │
   ▼
Legal Entity
   │
   │ explicit mapping
   ▼

PAYMENT ENGINE REPRESENTATION

HyperSwitch Organisation
   │
   ▼
Merchant
   │
   ▼
Business Profile
   │
   ▼
Connector / Provider
```

The Legal Entity remains the canonical business/legal principal.

The Merchant represents that principal within the payment engine for an explicitly governed scope.

The Business Profile represents payment-processing configuration beneath that Merchant.

Accordingly:

```text
Legal Entity
    ≠
Merchant

Market
    ≠
Business Profile

Digital Estate
    ≠
Business Profile

Corporate Group
    ≠
Merchant
```

The architecture SHALL preserve explicit mappings between these concepts and SHALL fail closed whenever the required mapping cannot be established.

**Baobab determines whose business is being conducted.  
The Payments engine determines the operational payment representation.  
HyperSwitch executes only within that governed representation.**