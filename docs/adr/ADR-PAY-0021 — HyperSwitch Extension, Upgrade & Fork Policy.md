# ADR-PAY-0021 — HyperSwitch Extension, Upgrade & Fork Policy

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Dependency Governance / Extension Policy |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0020; ADR-PAY-0022; applicable Shared, Control Plane and Infrastructure ADRs |
| **Primary Technology** | HyperSwitch |
| **Primary Objective** | Preserve upstream compatibility while enabling Baobab-specific payment capabilities |

---

# 1. Context

Baobab Payments adopts HyperSwitch as its underlying payment orchestration technology.

HyperSwitch provides substantial payment infrastructure, including:

```text id="81vgpa"
connector integrations
payment orchestration
routing
payment-method support
refund processing
webhooks
payout capabilities
vault/token integrations
operational tooling
```

Baobab intentionally does not expose HyperSwitch as the canonical platform contract.

The architecture established by PAY-0001 is:

```text id="48cljz"
Baobab Engines
      │
      ▼
Canonical Payments Contract
      │
      ▼
baobab-payments
      │
      ▼
PaymentProvider Port
      │
      ▼
HyperSwitch Adapter
      │
      ▼
HyperSwitch
      │
      ▼
Payment Providers / Rails
```

This boundary is strategically important.

HyperSwitch is an implementation dependency.

It is not:

```text id="72pp1a"
Baobab's canonical payment model
Baobab's Tenant authority
Baobab's Legal Entity authority
Baobab's Market authority
Baobab's IAM
Baobab's ERP
Baobab's accounting ledger
Baobab's provider-certification authority
```

---

# 2. Problem

Baobab will inevitably encounter requirements not immediately supported by an upstream HyperSwitch release.

Examples may include:

```text id="x3j5qb"
African PSP connectors
mobile-money rails
regional bank-transfer schemes
market-specific webhook behaviour
provider-specific settlement reports
provider-specific payout flows
local authentication mechanisms
regional payment-method metadata
specialised reconciliation support
```

The easiest short-term response could be:

> modify HyperSwitch core.

Repeated frequently, that produces:

```text id="i7cx1a"
Baobab HyperSwitch Fork
          │
          ▼
Custom Core Changes
          │
          ▼
Increasing Upstream Divergence
          │
          ▼
Upgrade Conflicts
          │
          ▼
Security Patch Delay
          │
          ▼
Operational Risk
          │
          ▼
Permanent Fork
```

This is rejected as the default architecture.

---

# 3. Decision

Baobab SHALL adopt an:

> **Upstream-first, adapter-first, connector-first, configuration-first extension strategy.**

Baobab SHALL prefer, in order:

```text id="1kzw4e"
1. Baobab configuration
2. Baobab Payments façade / canonical adapter
3. HyperSwitch supported configuration
4. HyperSwitch supported extension mechanism
5. HyperSwitch connector implementation
6. Upstream HyperSwitch contribution
7. Temporary maintained patch
8. Permanent fork
```

The further down this sequence a change proceeds, the stronger the architectural justification required.

---

# 4. Governing Principle

> **Extend HyperSwitch at its supported boundaries; isolate Baobab semantics in Baobab; contribute generally useful capabilities upstream; fork only when no maintainable alternative exists.**

---

# 5. Architectural Boundary

```text id="e9aqfx"
┌──────────────────────────────────────────────┐
│              BAOBAB DOMAIN                  │
│                                              │
│ Tenant                                       │
│ Organisation                                 │
│ Legal Entity                                 │
│ Market                                       │
│ Canonical Payment State                      │
│ Provider Certification                       │
│ Provider Activation                          │
│ IAM                                          │
│ ERP / Accounting                             │
└───────────────────┬──────────────────────────┘
                    │
                    ▼
┌──────────────────────────────────────────────┐
│           BAOBAB PAYMENTS FAÇADE             │
│                                              │
│ Canonical API                                │
│ Context Validation                           │
│ Idempotency                                  │
│ State Translation                            │
│ Eligibility Envelope                         │
│ Event Translation                            │
│ Provider Abstraction                         │
└───────────────────┬──────────────────────────┘
                    │
                    ▼
┌──────────────────────────────────────────────┐
│          HYPERSWITCH ADAPTER                 │
│                                              │
│ Canonical ↔ HyperSwitch Translation          │
│ API Integration                              │
│ Error Translation                            │
│ Provider Reference Translation               │
└───────────────────┬──────────────────────────┘
                    │
                    ▼
┌──────────────────────────────────────────────┐
│              HYPERSWITCH                     │
│                                              │
│ Connector Orchestration                      │
│ Routing                                      │
│ Payment Processing                           │
│ Provider Integration                         │
└───────────────────┬──────────────────────────┘
                    │
                    ▼
                 PROVIDERS
```

Baobab-specific business semantics SHALL remain above the HyperSwitch boundary whenever possible.

---

# 6. Standardise Contracts, Not Implementations

The Baobab platform principle remains:

> **Standardise contracts, not implementations.**

Therefore:

```text id="i6br9s"
Baobab PaymentIntent
        ≠
HyperSwitch PaymentIntent

Baobab Payment
        ≠
HyperSwitch Payment

Baobab Refund
        ≠
HyperSwitch Refund

Baobab Payout
        ≠
HyperSwitch Payout

Baobab Market
        ≠
HyperSwitch Business Profile

Baobab Legal Entity
        ≠
HyperSwitch Merchant
```

---

# 7. HyperSwitch API Boundary

`baobab-payments` SHALL normally interact with HyperSwitch through supported APIs and documented integration mechanisms.

---

# 8. No Direct Database Integration

Baobab SHALL NOT create application dependencies on HyperSwitch internal database tables.

Prohibited:

```text id="90kqx7"
baobab-payments
      │
      ▼
SELECT *
FROM hyperswitch.payment_attempt
```

for normal application integration.

---

# 9. Why Database Coupling Is Rejected

Internal database coupling creates dependency on:

```text id="s2fhxm"
private schemas
migration details
storage representations
internal enums
implementation-specific relationships
```

and substantially increases upgrade risk.

---

# 10. HyperSwitch Database Ownership

HyperSwitch owns its own database schema.

Baobab Payments owns its canonical operational database.

---

# 11. Adapter Boundary

The HyperSwitch integration SHALL remain behind a Baobab-owned adapter implementing the Payments provider port.

Conceptually:

```text id="x0v98b"
trait PaymentProvider {
    create_payment(...)
    authorize(...)
    capture(...)
    cancel(...)
    refund(...)
    get_payment(...)
}
```

The exact Rust interface may evolve.

The architectural boundary SHALL remain.

---

# 12. Adapter Responsibilities

The HyperSwitch adapter MAY perform:

```text id="jve72p"
request translation
response translation
error normalization
provider-reference extraction
status translation
action translation
webhook normalization support
capability translation
```

---

# 13. Adapter Non-Responsibilities

The adapter SHALL NOT become authority for:

```text id="2u9j6h"
Tenant hierarchy
Legal Entity hierarchy
Market definition
commerce obligation
subscription billing
ERP accounting
provider certification
IAM authorization
```

---

# 14. Anti-Corruption Layer

The adapter serves as an architectural anti-corruption layer between:

```text id="4jv1bi"
Baobab Canonical Semantics
           │
           ▼
HyperSwitch Operational Semantics
```

Changes in one model SHOULD NOT unnecessarily propagate into the other.

---

# 15. Extension Hierarchy

Every proposed HyperSwitch-related change SHALL first be classified against the following hierarchy.

| Priority | Extension Strategy |
|---|---|
| 1 | Baobab-side configuration |
| 2 | Baobab façade logic |
| 3 | HyperSwitch configuration |
| 4 | Supported HyperSwitch extension |
| 5 | New/extended HyperSwitch connector |
| 6 | Upstream contribution |
| 7 | Temporary downstream patch |
| 8 | Permanent fork |

---

# 16. Configuration First

If behaviour can safely be achieved through supported configuration, configuration SHALL be preferred over source-code modification.

---

# 17. Configuration Is Governed

Configuration-first does not mean uncontrolled runtime mutation.

PAY-0018 controlled mutation remains binding.

---

# 18. Baobab-Specific Policy

Baobab-specific business policy SHOULD normally live in `baobab-payments`, not HyperSwitch core.

Examples:

```text id="xk9qsv"
Baobab Tenant context
Legal Entity mapping
Market context
provider certification
provider activation
canonical capability checks
Baobab-specific authorization
canonical event translation
```

---

# 19. HyperSwitch-Specific Behaviour

Implementation-specific payment orchestration belongs behind the adapter or within supported HyperSwitch mechanisms.

---

# 20. Connector-First Provider Extension

When Baobab requires a new payment provider, rail or PSP that HyperSwitch does not support, the preferred implementation is a HyperSwitch-compatible connector rather than modification of generic payment core.

---

# 21. Connector Principle

```text id="fgxxqf"
New Provider
     │
     ▼
New Connector
```

is preferable to:

```text id="s7n2mq"
New Provider
     │
     ▼
Modify Generic Payment Core
```

unless the provider genuinely requires a new general payment capability.

---

# 22. African Payment Providers

Baobab anticipates that some African payment providers may not initially have upstream HyperSwitch support.

Such absence SHALL NOT automatically justify a private HyperSwitch fork.

---

# 23. African Connector Strategy

Preferred sequence:

```text id="1we6rx"
Provider Requirement
        │
        ▼
Check Upstream Support
        │
        ├── Supported
        │      ▼
        │   Certify / Activate
        │
        └── Unsupported
               │
               ▼
        Can Connector Support It?
               │
          ┌────┴────┐
          │         │
         YES        NO
          │         │
          ▼         ▼
      Connector   General Capability Gap
          │         │
          ▼         ▼
     Upstream PR  Evaluate Upstream Core Change
```

---

# 24. Connector Ownership

A Baobab-developed connector SHALL have an identified maintainer.

---

# 25. Connector Certification

A new connector SHALL NOT become production routable merely because its code compiles.

PAY-0022 certification remains mandatory.

---

# 26. Connector Test Matrix

Connector testing SHOULD cover applicable:

```text id="o42o4x"
authorize
capture
void/cancel
refund
partial refund
webhook verification
payment sync
refund sync
asynchronous payment completion
mandates
recurring execution
payout
dispute
settlement/reconciliation
error translation
idempotency
timeout behaviour
```

depending on provider capabilities.

---

# 27. Unsupported Capability

If a connector does not support a capability:

```text id="6wjw3f"
NOT SUPPORTED
```

SHALL be represented explicitly.

The system SHALL NOT simulate provider support.

---

# 28. Provider-Specific Logic

Provider-specific protocol details SHOULD remain inside the connector/adapter layer.

---

# 29. No Leaf-Estate Provider Code

Provider-specific integration logic SHALL NOT be placed in:

```text id="50jz7a"
ZuriBeans frontend
Thamani frontend
Nabhold corporate estate
Trade business logic
Subscriptions business logic
```

---

# 30. Upstream Contribution

Where a Baobab-developed change is generally useful to HyperSwitch users, Baobab SHOULD prefer contributing it upstream.

---

# 31. Why Upstream Contribution Is Preferred

Upstream contribution can provide:

```text id="ebew2e"
shared maintenance
upstream testing
community review
security review
future compatibility
lower upgrade burden
```

---

# 32. Upstream Acceptance Is Not Guaranteed

Baobab SHALL not make production timelines entirely dependent on upstream acceptance.

---

# 33. Temporary Downstream Patch

Where a required upstream contribution cannot be merged in time, Baobab MAY maintain a temporary downstream patch.

---

# 34. Temporary Patch Definition

A temporary patch is:

```text id="x2vxx4"
small
isolated
documented
tested
tracked
intended for upstreaming/removal
```

---

# 35. Patch Registry

Every downstream patch SHALL be registered.

Conceptual record:

```text id="zqdyw1"
patch_id
title
upstream_version
affected_component
reason
owner
upstream_issue
upstream_pr
introduced_at
security_impact
upgrade_impact
removal_condition
status
```

---

# 36. Patch Status

Suggested states:

```text id="o12fg6"
PROPOSED
APPROVED
ACTIVE
UPSTREAM_SUBMITTED
UPSTREAM_ACCEPTED
SUPERSEDED
REMOVED
```

---

# 37. Patch Budget

Baobab SHOULD maintain a deliberately small downstream patch budget.

Growing patch count is architectural debt.

---

# 38. Patch Metrics

Operations/engineering SHOULD track:

```text id="64qev7"
active patch count
patch age
upstream status
upgrade conflicts
patch test coverage
```

---

# 39. Patch Age

Long-lived temporary patches SHALL trigger architectural review.

---

# 40. Permanent Fork

A permanent HyperSwitch fork is an exceptional architectural decision.

---

# 41. Fork Approval

A permanent fork SHALL require a separate ADR.

---

# 42. Fork ADR Requirements

The ADR SHALL demonstrate at minimum:

```text id="i5vpyh"
business necessity
why configuration is insufficient
why façade logic is insufficient
why adapter logic is insufficient
why connector extension is insufficient
why upstream contribution is insufficient
expected divergence
security ownership
maintenance ownership
upgrade strategy
staffing/cost
exit strategy
```

---

# 43. Fork Burden

A permanent fork means Baobab assumes responsibility for maintaining affected upstream functionality.

---

# 44. Fork Security Responsibility

Baobab SHALL assume responsibility for timely integration of relevant upstream security fixes into the fork.

---

# 45. Fork Compatibility Responsibility

Baobab SHALL own compatibility testing against:

```text id="3w6tnk"
database migrations
connectors
routing
vaulting
webhooks
scheduler
Superposition
APIs
SDK interactions where applicable
```

---

# 46. Fork Exit Strategy

Every permanent-fork ADR SHALL state how Baobab could eventually return to upstream.

---

# 47. No Silent Fork

A modified HyperSwitch build SHALL never silently masquerade as an unmodified upstream release.

---

# 48. Build Provenance

Baobab SHALL be able to determine exactly:

```text id="tbodsm"
upstream version
upstream commit
Baobab patches
build commit
container digest
build date
```

for every deployed HyperSwitch image.

---

# 49. Version Pinning

Production SHALL pin an explicitly tested HyperSwitch version.

---

# 50. No `latest`

Production SHALL NOT deploy:

```text id="7d45ri"
hyperswitch:latest
```

or equivalent mutable version selection.

---

# 51. Immutable Images

Production SHOULD deploy immutable image digests where practical.

---

# 52. Version Registry

Baobab SHOULD maintain a dependency record containing:

```text id="ksgyk7"
HyperSwitch version
Router version
Control Center version if deployed
vault version if deployed
encryption-service version if deployed
Superposition version
connector-specific dependencies
database compatibility
Baobab adapter version
```

as applicable to the deployed architecture.

---

# 53. Compatibility Matrix

Each supported Baobab release SHOULD have an explicit compatibility matrix.

Example:

| Component | Approved Version |
|---|---|
| `baobab-payments` | release X |
| HyperSwitch | approved pinned release |
| HyperSwitch adapter | release X |
| PostgreSQL | approved PostgreSQL 17.x |
| Superposition | approved compatible release |
| Vault | approved compatible release |
| Encryption service | approved compatible release |

Actual values belong to release configuration, not this ADR.

---

# 54. Upstream Release Monitoring

Baobab SHALL monitor upstream HyperSwitch releases.

---

# 55. Release Review

Each relevant release SHALL be reviewed for:

```text id="hratxm"
security fixes
breaking changes
database migrations
API changes
connector changes
routing changes
webhook changes
vault/token changes
payout changes
dispute changes
settlement changes
configuration changes
dependency changes
deprecated functionality
```

---

# 56. No Automatic Production Upgrade

A new upstream release SHALL NOT automatically enter production.

---

# 57. Upgrade Pipeline

The upgrade process SHALL follow:

```text id="0d2qgf"
Upstream Release
      │
      ▼
Release Review
      │
      ▼
Compatibility Assessment
      │
      ▼
Security Assessment
      │
      ▼
Migration Assessment
      │
      ▼
Adapter Compatibility
      │
      ▼
Connector Tests
      │
      ▼
Canonical Contract Tests
      │
      ▼
Integration Tests
      │
      ▼
DR / Rollback Review
      │
      ▼
Staging
      │
      ▼
Controlled Production Rollout
      │
      ▼
Observation
      │
      ▼
Full Adoption
```

---

# 58. Upgrade Classification

An upgrade SHOULD be classified:

```text id="3vz13i"
PATCH
MINOR
MAJOR / ARCHITECTURAL
SECURITY-CRITICAL
```

based on actual impact, not only version numbering.

---

# 59. Architectural Change Detection

A nominally small upstream version increment MAY still introduce an architectural dependency.

Therefore semantic version numbers alone SHALL not determine upgrade risk.

---

# 60. Superposition Precedent

HyperSwitch's adoption of Superposition demonstrates why release notes must be treated as architecture input.

A release can introduce:

```text id="y01dj4"
new mandatory service
configuration migration
new runtime dependency
new failure mode
new backup requirement
new DR requirement
```

without changing Baobab's canonical contracts.

---

# 61. Superposition

Where required by the selected HyperSwitch release, Superposition SHALL be treated as a first-class HyperSwitch runtime dependency.

---

# 62. Superposition Is Not Baobab Control Plane

```text id="34dk47"
HyperSwitch Superposition
          ≠
Baobab Control Plane
```

---

# 63. Configuration Authority

Superposition may resolve HyperSwitch operational configuration.

It SHALL NOT become canonical authority for:

```text id="pn7zby"
Tenant
Organisation
Legal Entity
Market
Digital Estate
Baobab Capability Binding
provider certification
Baobab IAM
```

---

# 64. Configuration Mapping

Where Baobab context constrains HyperSwitch configuration:

```text id="cxkwm0"
Baobab Canonical Context
          │
          ▼
Payments Eligibility
          │
          ▼
HyperSwitch Operational Context
          │
          ▼
Superposition Resolution
```

shall preserve the authority boundary.

---

# 65. No Duplicate Control Plane

Baobab SHALL not recreate Control Plane canonical concepts inside Superposition merely because HyperSwitch supports contextual configuration dimensions.

---

# 66. Runtime Configuration

Runtime HyperSwitch configuration changes SHALL be governed as production changes.

---

# 67. Dynamic Configuration Risk

The ability to change behaviour without deployment does not reduce the need for:

```text id="9rblmg"
authorization
audit
validation
change review
rollback
observability
```

---

# 68. Configuration Version

Material runtime configuration SHOULD have identifiable revision/version provenance.

---

# 69. Configuration Rollback

Baobab SHOULD be able to restore a known-good operational configuration without reverting canonical business history.

---

# 70. Configuration Drift

Baobab SHALL detect material drift between:

```text id="13aovc"
Baobab provider eligibility
Baobab merchant/profile mappings
HyperSwitch connector configuration
HyperSwitch routing
Superposition configuration
```

where technically possible.

---

# 71. Drift Does Not Rewrite Canonical State

External configuration drift SHALL not silently redefine Baobab authority.

---

# 72. Upgrade Database Migrations

HyperSwitch database migrations SHALL be treated as controlled production changes.

---

# 73. Migration Review

Before upgrade, Baobab SHALL understand:

```text id="qek0nj"
migration direction
schema impact
locking impact
downtime expectations
data transformation
rollback limitations
backup requirements
```

where applicable.

---

# 74. Pre-Migration Backup

Material HyperSwitch migrations SHOULD require verified recovery points.

---

# 75. Application Rollback Versus Schema Rollback

Baobab SHALL not assume:

```text id="qwwymc"
application rollback
=
database rollback
```

---

# 76. External Financial Reality

Neither application nor database rollback can reverse external money movement already performed.

PAY-0020 remains binding.

---

# 77. Upgrade During In-Flight Payments

Upgrades SHALL account for:

```text id="p74ydn"
PROCESSING payments
UNKNOWN attempts
pending captures
pending refunds
pending payouts
active disputes
webhook deliveries
scheduled retries
settlement ingestion
```

---

# 78. State Compatibility

A new version SHALL correctly interpret financial state created by the previous supported version.

---

# 79. Rolling Upgrade

Where rolling deployment is used, compatibility SHALL be established between versions concurrently serving traffic.

---

# 80. Mixed-Version Safety

Mixed-version operation SHALL NOT be assumed safe without testing.

---

# 81. Canonical Contract Stability

Baobab canonical payment contracts SHOULD remain more stable than HyperSwitch internal APIs.

---

# 82. Adapter Absorbs Change

Where feasible:

```text id="l14lb3"
HyperSwitch API Change
        │
        ▼
Adapter Change
        │
        ▼
Canonical Baobab API Unchanged
```

---

# 83. Breaking Canonical Change

A HyperSwitch upgrade SHALL NOT by itself justify a breaking Baobab canonical contract change.

---

# 84. Canonical Change Governance

If a genuine Baobab-domain change is required, it SHALL follow Shared contract governance independently of HyperSwitch upgrade pressure.

---

# 85. Deprecated HyperSwitch APIs

Baobab SHALL monitor upstream deprecations.

---

# 86. Deprecation Migration

Deprecated HyperSwitch interfaces SHOULD be migrated before forced removal where practical.

---

# 87. Provider Connector Changes

An upstream connector update MAY alter:

```text id="h5yswr"
request mapping
status mapping
webhook handling
payment-method support
refund behaviour
error semantics
```

and SHALL therefore be tested as financial behaviour, not merely compilation.

---

# 88. Connector Regression Testing

Critical providers SHALL have provider-specific regression suites.

---

# 89. Certification Revalidation

Material connector changes MAY require PAY-0022 recertification or targeted certification revalidation.

---

# 90. Routing Changes

Changes to HyperSwitch routing semantics SHALL be assessed against PAY-0009.

---

# 91. Routing Cannot Escape Eligibility

No HyperSwitch upgrade SHALL permit routing outside Baobab's authorised provider envelope.

---

# 92. Idempotency Changes

HyperSwitch idempotency changes SHALL be assessed against PAY-0010.

Baobab-level duplicate safety remains authoritative regardless of upstream implementation.

---

# 93. Webhook Changes

Webhook schema/verification/state changes SHALL be assessed against PAY-0011.

---

# 94. Tokenisation/Vault Changes

Changes involving:

```text id="wll1h8"
vaulting
tokenisation
saved payment methods
external vaults
network tokens
```

SHALL undergo PAY-0012 security/PCI review.

---

# 95. Refund Changes

Refund changes SHALL be tested against PAY-0013 canonical semantics.

---

# 96. Dispute Changes

Dispute changes SHALL be tested against PAY-0014.

---

# 97. Settlement Changes

Settlement/reconciliation changes SHALL be tested against PAY-0015.

---

# 98. Payout Changes

Payout connector/core changes SHALL be tested against PAY-0016.

---

# 99. FX Changes

Currency/FX-related changes SHALL be assessed against PAY-0017.

---

# 100. Security Changes

Authentication, permissions, secret-handling or administrative changes SHALL be assessed against PAY-0018.

---

# 101. Observability Changes

Telemetry changes SHALL be assessed against PAY-0019.

---

# 102. DR Changes

New dependencies, state stores or schedulers SHALL be incorporated into PAY-0020 recovery procedures before production adoption.

---

# 103. Upgrade Test Layers

Every material HyperSwitch upgrade SHOULD pass:

```text id="ovkpmv"
Unit Tests
    │
    ▼
Adapter Contract Tests
    │
    ▼
Canonical Payments Contract Tests
    │
    ▼
Connector Tests
    │
    ▼
HyperSwitch Integration Tests
    │
    ▼
End-to-End Tests
    │
    ▼
Failure / Timeout Tests
    │
    ▼
Upgrade / Migration Tests
    │
    ▼
Recovery Tests
```

---

# 104. Contract Tests

Baobab SHOULD maintain contract tests asserting canonical behaviour independent of HyperSwitch version.

---

# 105. Representative Contract Cases

Tests SHOULD include:

```text id="4iybyq"
create PaymentIntent
authorize
capture
manual capture
requires action
async processing
failure
timeout
UNKNOWN
cancel/void
partial capture
Refund
partial Refund
duplicate request
webhook
provider sync
provider failover
Dispute
Settlement
Payout
```

where supported.

---

# 106. Golden Provider Tests

Critical provider integrations SHOULD have stable representative test fixtures.

---

# 107. Provider Sandbox

Provider sandbox testing SHOULD be performed where available.

---

# 108. Sandbox Limitation

```text id="dzpfhy"
Provider Sandbox Success
        ≠
Production Certification
```

PAY-0022 remains authoritative.

---

# 109. Shadow Testing

Where safe and technically appropriate, Baobab MAY compare old/new HyperSwitch versions using non-financial shadow evaluation.

---

# 110. No Duplicate Shadow Money Movement

Shadow testing SHALL NOT send duplicate real financial mutations to providers.

---

# 111. Canary Upgrade

Production HyperSwitch upgrades SHOULD use controlled canary/progressive rollout where architecture permits.

---

# 112. Canary Boundary

Canary routing SHALL preserve:

```text id="66pscm"
Tenant
Legal Entity
Market
provider eligibility
idempotency
financial consistency
```

---

# 113. Cohort Safety

A single financial aggregate SHOULD not unpredictably oscillate between incompatible HyperSwitch versions.

---

# 114. Upgrade Freeze Conditions

An upgrade SHOULD be paused when there is:

```text id="g2x68m"
unresolved financial incident
large UNKNOWN backlog
reconciliation instability
provider outage
failed DR exercise affecting upgrade path
unresolved connector regression
```

unless the upgrade itself is required to remediate the incident.

---

# 115. Security Upgrade

Security-critical upstream releases MAY require accelerated adoption.

Accelerated does not mean untested.

---

# 116. Emergency Patch

An emergency security patch MAY follow a shortened change process but SHALL retain:

```text id="20lgra"
review
test evidence
build provenance
rollback/recovery plan
audit
```

---

# 117. Vulnerability Monitoring

Baobab SHALL monitor security advisories affecting deployed HyperSwitch components and dependencies.

---

# 118. Dependency Inventory

The deployment SHALL maintain enough dependency inventory to determine whether a disclosed vulnerability affects Baobab.

---

# 119. Software Provenance

Baobab SHOULD verify the provenance of HyperSwitch source/images/releases used for production.

---

# 120. Build From Source

If Baobab builds HyperSwitch itself, builds SHOULD be reproducible as far as practical.

---

# 121. Third-Party Image Trust

Third-party/upstream images SHALL be subject to Baobab's supply-chain and vulnerability policies.

---

# 122. Baobab-Patched Images

Patched images SHALL be stored in an approved Baobab-controlled registry.

---

# 123. Image Naming

Image naming SHALL make custom builds distinguishable from pristine upstream builds.

---

# 124. Source Tracking

Every custom build SHALL map back to:

```text id="63n43e"
Baobab repository commit
upstream commit
patch set
CI build
container digest
```

---

# 125. Patch Rebase

On every upstream upgrade, active patches SHALL be reassessed.

---

# 126. Patch Reassessment Outcomes

Each patch becomes:

```text id="ukqds6"
still required
modified
upstreamed
superseded
removed
```

---

# 127. No Zombie Patches

A patch whose purpose no longer exists SHALL be removed.

---

# 128. Upstream Conflict

A patch conflicting repeatedly with upstream is evidence that the extension boundary may be wrong.

---

# 129. Architectural Escalation

Repeated patch conflicts SHALL trigger consideration of:

```text id="pt2p70"
adapter relocation
connector relocation
upstream redesign
separate service
formal fork ADR
```

---

# 130. Separate Service Alternative

A capability that does not naturally belong inside HyperSwitch SHOULD be implemented as a separate Baobab service rather than forced into HyperSwitch core.

---

# 131. Examples of Separate-Service Candidates

Potential examples include future:

```text id="tw5t26"
financial ledger
specialised compliance service
treasury orchestration
market-specific regulatory reporting
```

depending on architecture.

---

# 132. Planned Ledger Engine

The planned Baobab Ledger Engine SHALL not be implemented as a HyperSwitch core modification.

---

# 133. ERP Boundary

ERP accounting functionality SHALL not be inserted into HyperSwitch.

---

# 134. Control Plane Boundary

Baobab tenancy/canonical context SHALL not be inserted into HyperSwitch core as a competing platform model.

---

# 135. IAM Boundary

Baobab IAM remains separate.

HyperSwitch's own administrative/authentication requirements do not replace Baobab IAM authority.

---

# 136. Control Center

HyperSwitch Control Center MAY be used for appropriate operational administration.

---

# 137. Control Center Is Not Baobab Control Plane

```text id="p29m8b"
HyperSwitch Control Center
          ≠
Baobab Control Plane
```

---

# 138. Control Center Mutations

Production Control Center changes SHALL be governed under PAY-0018 and configuration-drift controls where applicable.

---

# 139. Manual Configuration

Manual provider/routing changes in HyperSwitch that bypass Baobab governance SHALL be detected and reconciled.

---

# 140. Infrastructure Dependency Changes

Every HyperSwitch upgrade SHALL inspect whether new mandatory infrastructure has appeared.

---

# 141. Dependency Admission

A new mandatory dependency SHALL not silently enter production.

It requires assessment of:

```text id="7qjs49"
availability
security
backup
recovery
observability
capacity
networking
secrets
data residency
cost
```

---

# 142. Dependency Removal

Removed upstream dependencies SHALL also be cleaned from Baobab infrastructure when safe.

---

# 143. Configuration Migration

HyperSwitch configuration migrations SHALL be:

```text id="cyxbmv"
version-aware
repeatable where possible
tested
backed up
audited
validated after execution
```

---

# 144. Configuration Migration Validation

After migration, Baobab SHALL verify effective operational configuration, not merely migration-script exit status.

---

# 145. Configuration Semantics

A syntactically successful migration that changes routing/payment behaviour incorrectly is a failed migration.

---

# 146. Provider Activation After Upgrade

Provider certification does not automatically disappear after every HyperSwitch upgrade.

However, material changes SHALL trigger appropriate revalidation.

---

# 147. Revalidation Scope

Revalidation MAY be:

```text id="7lfblx"
full provider recertification
connector-specific recertification
capability-specific retest
security-only review
routing-only review
```

depending on change impact.

---

# 148. Upgrade Risk Matrix

Every material upgrade SHOULD be assessed across:

| Domain | Question |
|---|---|
| API | Did HyperSwitch API semantics change? |
| State | Did payment/refund/payout states change? |
| Database | Are migrations required? |
| Connector | Did provider behaviour change? |
| Routing | Did routing semantics/configuration change? |
| Webhooks | Did callback handling change? |
| Vault | Did tokenisation/storage change? |
| Security | Did auth/secret behaviour change? |
| Settlement | Did reconciliation behaviour change? |
| Payout | Did outbound movement behaviour change? |
| Infrastructure | Are new dependencies required? |
| DR | Can the new version be restored safely? |

---

# 149. Upgrade Evidence

An upgrade SHALL retain evidence of:

```text id="3q3kq5"
release reviewed
tests executed
migration tested
provider tests
security review
DR review
approvals
deployment
post-deployment verification
```

---

# 150. Post-Upgrade Monitoring

PAY-0019 heightened monitoring SHALL follow material upgrades.

---

# 151. Post-Upgrade Financial Verification

Verification SHALL include more than HTTP health.

Check:

```text id="2f91cc"
authorization
capture
Refund
webhook convergence
provider sync
idempotency
UNKNOWN rate
routing
settlement
reconciliation
```

as applicable.

---

# 152. Rollback Decision

Rollback SHALL consider whether the old version can safely interpret state written by the new version.

---

# 153. Forward Fix

Where schema/state compatibility prevents safe rollback, a forward fix MAY be safer.

---

# 154. Rollback Does Not Undo Payments

```text id="7aixm6"
Software Rollback
       ≠
Financial Rollback
```

---

# 155. Version Support Window

Baobab SHOULD define which HyperSwitch versions it actively supports.

---

# 156. Unsupported Version

A version outside the supported window SHALL not remain indefinitely in production without documented exception.

---

# 157. Upgrade Cadence

Baobab SHOULD maintain a deliberate upgrade cadence rather than allowing indefinite drift from upstream.

---

# 158. Upgrade Cadence Is Risk-Based

Not every upstream release must be adopted immediately.

But long-lived divergence SHALL be explicit.

---

# 159. Upgrade Debt

Distance from current upstream releases SHOULD be treated as operational debt.

---

# 160. Upgrade Debt Metrics

Possible measures:

```text id="0jzcl4"
releases behind
months behind
unapplied security fixes
active downstream patches
deprecated APIs remaining
```

---

# 161. Upgrade Ownership

A named engineering owner SHALL be accountable for HyperSwitch lifecycle management.

---

# 162. Provider Connector Ownership

Critical custom connectors SHALL also have explicit owners.

---

# 163. No Orphan Connector

A production connector without an active maintenance owner SHALL be considered a production-readiness risk.

---

# 164. Documentation

Custom connectors, patches and operational extensions SHALL be documented.

---

# 165. Documentation Minimum

Documentation SHOULD include:

```text id="t5c1hh"
purpose
provider
supported capabilities
unsupported capabilities
configuration
credentials
webhooks
errors
tests
certification
upgrade considerations
owner
```

---

# 166. Upstream References

Where a patch tracks an upstream issue or PR, that reference SHALL be retained.

---

# 167. Licensing

Baobab SHALL review applicable upstream licences and obligations when:

```text id="f2gcp4"
redistributing modified builds
shipping connectors
maintaining forks
embedding upstream code
```

---

# 168. Licence Compliance Is Release Requirement

A technically functional extension SHALL not enter production distribution if its licensing obligations are unresolved.

---

# 169. Intellectual Property

Baobab-developed generic connector contributions SHOULD be evaluated for upstream contribution subject to:

```text id="5c97tf"
commercial strategy
provider agreements
licensing
security
```

---

# 170. Provider Confidentiality

Provider contracts/secrets/confidential specifications SHALL not be exposed in public upstream contributions.

---

# 171. Secrets Never in Patch

No custom patch, connector or fork SHALL embed production credentials.

---

# 172. Test Credentials

Even test credentials SHALL follow repository secret-handling policy.

---

# 173. Observability

Custom connector/patch behaviour SHALL expose sufficient telemetry for PAY-0019.

---

# 174. Error Normalisation

Custom extensions SHALL integrate with canonical Baobab error classification.

---

# 175. Timeout Semantics

A custom connector SHALL correctly distinguish:

```text id="jrm1yd"
definitive failure
safe pre-submission failure
timeout
ambiguous external outcome
```

---

# 176. UNKNOWN Support

A connector unable to prove whether an external mutation succeeded SHALL permit Baobab to represent `UNKNOWN`.

---

# 177. No False Failure

A connector SHALL NOT map ambiguous external outcomes to definitive failure merely to simplify implementation.

---

# 178. Idempotency

Custom connectors SHALL use provider idempotency capabilities where available.

This supplements rather than replaces PAY-0010.

---

# 179. Webhook Verification

Custom connectors SHALL implement provider webhook verification where available.

---

# 180. Sync/Recovery

Where supported by the provider, custom connectors SHOULD implement query/sync operations needed to recover ambiguous state.

---

# 181. Settlement Support

Where a provider exposes settlement/reconciliation data, integration SHOULD expose it through the appropriate Payments reconciliation adapter rather than embedding provider-specific logic into ERP.

---

# 182. Payout Support

Provider Payout support SHALL comply with PAY-0016 and SHALL not merely reuse collection semantics without review.

---

# 183. FX Support

Provider FX behaviour SHALL comply with PAY-0017.

---

# 184. PCI Review

Any connector change affecting payment credentials SHALL trigger PAY-0012 review.

---

# 185. Certification

Every production provider extension remains subject to PAY-0022.

---

# 186. Fork Decision Tree

```text id="hcf6xb"
NEW REQUIREMENT
      │
      ▼
Can Baobab configuration solve it?
      │
  ┌───┴───┐
 YES      NO
  │        │
CONFIG   Can Payments façade solve it?
           │
       ┌───┴───┐
      YES      NO
       │        │
    FAÇADE   Can supported HyperSwitch
             configuration solve it?
                 │
             ┌───┴───┐
            YES      NO
             │        │
          CONFIG   Can connector/extension
                   boundary solve it?
                       │
                   ┌───┴───┐
                  YES      NO
                   │        │
              CONNECTOR   Is change generally
                          useful upstream?
                              │
                          ┌───┴───┐
                         YES      NO
                          │        │
                    UPSTREAM PR   Is temporary
                                  patch viable?
                                      │
                                  ┌───┴───┐
                                 YES      NO
                                  │        │
                             TEMP PATCH  SEPARATE
                                  │      SERVICE?
                                  │        │
                                  ▼        ▼
                              UPSTREAM   If NO:
                               /REMOVE   FORK ADR
```

---

# 187. Production Readiness Gate

```text id="bik9nu"
HYPERSWITCH LIFECYCLE READINESS

Pinned upstream version                    PASS
Immutable image/digest                     PASS
Version provenance                         PASS
Compatibility matrix                       PASS
Adapter boundary                           PASS
No direct HyperSwitch DB dependency        PASS
Canonical contracts independent            PASS
Active patch registry                      PASS
Patch owners                               PASS
Connector owners                           PASS
Provider-specific regression tests         PASS
Canonical contract tests                   PASS
Integration tests                          PASS
Webhook tests                              PASS
Idempotency tests                          PASS
Timeout / UNKNOWN tests                    PASS
Routing tests                              PASS
Refund tests                               PASS
Dispute tests                              PASS
Settlement tests                           PASS
Payout tests                               PASS
PCI/security review                        PASS
Dependency inventory                       PASS
Superposition compatibility                PASS
Migration review                           PASS
Backup / DR review                         PASS
Provider certification impact review       PASS
Staging validation                         PASS
Controlled rollout                         PASS
Post-upgrade monitoring                    PASS
Rollback / forward-fix plan                PASS
Upstream security monitoring               PASS
Licence review                             PASS
------------------------------------------------
HyperSwitch Lifecycle                      READY
```

---

# 188. Invariants

The following invariants SHALL hold:

1. HyperSwitch is an implementation dependency, not Baobab's canonical payment contract.
2. Baobab canonical payment semantics remain provider-independent.
3. Baobab Tenant authority remains outside HyperSwitch.
4. Baobab Legal Entity authority remains outside HyperSwitch.
5. Baobab Market authority remains outside HyperSwitch.
6. HyperSwitch does not become Baobab IAM.
7. HyperSwitch does not become Baobab ERP.
8. HyperSwitch does not become the Baobab ledger.
9. Baobab interacts with HyperSwitch through supported interfaces.
10. Direct HyperSwitch database integration is prohibited for normal application behaviour.
11. The HyperSwitch adapter remains an anti-corruption layer.
12. Configuration is preferred over source modification where appropriate.
13. Baobab-specific business policy remains in Baobab.
14. Provider-specific integration logic remains in appropriate connector/adapter boundaries.
15. New providers do not automatically require core modifications.
16. Unsupported African providers do not automatically justify a fork.
17. New connectors require ownership.
18. Connector compilation does not imply production certification.
19. Unsupported capabilities are explicit.
20. Leaf estates do not contain PSP integration logic.
21. Generally useful changes should be considered for upstream contribution.
22. Production timelines do not depend blindly on upstream acceptance.
23. Temporary downstream patches are documented.
24. Every downstream patch has an owner.
25. Every downstream patch has a removal/upstream strategy.
26. Patch count is deliberately constrained.
27. Long-lived patches trigger review.
28. Permanent forks require a separate ADR.
29. Forks require explicit maintenance ownership.
30. Forks require security-patch ownership.
31. Forks require an exit strategy.
32. Modified builds do not masquerade as pristine upstream releases.
33. Production HyperSwitch versions are pinned.
34. Mutable `latest` tags are prohibited.
35. Build provenance is retained.
36. Compatibility matrices are maintained.
37. Upstream releases are monitored.
38. New upstream releases are not automatically deployed.
39. Upgrade risk is determined by actual architecture impact, not version number alone.
40. New mandatory dependencies receive architecture review.
41. Superposition does not become the Baobab Control Plane.
42. HyperSwitch configuration dimensions do not redefine canonical Baobab context.
43. Dynamic configuration changes remain governed.
44. Configuration drift is detectable.
45. Database migrations are controlled changes.
46. Application rollback is not assumed to equal schema rollback.
47. Software rollback does not undo external financial reality.
48. In-flight financial operations are considered during upgrades.
49. Mixed-version operation is tested before reliance.
50. HyperSwitch API changes should be absorbed by the adapter where feasible.
51. HyperSwitch upgrades do not automatically break Baobab canonical contracts.
52. Canonical contract changes follow Baobab contract governance.
53. Provider connector changes receive financial regression testing.
54. Material connector changes may trigger certification revalidation.
55. Routing changes remain constrained by PAY-0009.
56. HyperSwitch idempotency never replaces Baobab idempotency.
57. Webhook changes remain constrained by PAY-0011.
58. Vault/token changes trigger PAY-0012 review.
59. Refund changes preserve PAY-0013 semantics.
60. Dispute changes preserve PAY-0014 semantics.
61. Settlement changes preserve PAY-0015 semantics.
62. Payout changes preserve PAY-0016 semantics.
63. FX changes preserve PAY-0017 semantics.
64. Security changes preserve PAY-0018 controls.
65. Observability changes preserve PAY-0019 visibility.
66. New runtime dependencies enter PAY-0020 DR planning.
67. Material upgrades pass layered testing.
68. Canonical contract tests are independent of HyperSwitch version.
69. Sandbox success is not production certification.
70. Shadow testing does not duplicate real money movement.
71. Production upgrades use controlled rollout where practical.
72. A financial aggregate is not arbitrarily split across incompatible versions.
73. Upgrade freezes may apply during financial instability.
74. Security-critical upgrades may accelerate process but do not eliminate validation.
75. Vulnerability monitoring covers deployed upstream components.
76. Custom builds have verifiable provenance.
77. Patched images are distinguishable.
78. Active patches are reassessed on every upgrade.
79. Obsolete patches are removed.
80. Repeated patch conflicts trigger architectural review.
81. Capabilities that do not belong in HyperSwitch may become separate Baobab services.
82. The planned Ledger Engine is not a HyperSwitch core modification.
83. ERP accounting is not inserted into HyperSwitch.
84. Baobab Control Plane semantics are not duplicated inside HyperSwitch.
85. HyperSwitch Control Center is not Baobab Control Plane.
86. Manual HyperSwitch changes remain governed.
87. Configuration migrations are validated semantically.
88. Provider certification is revalidated when material implementation behaviour changes.
89. Upgrade evidence is retained.
90. Post-upgrade verification includes financial behaviour.
91. Rollback compatibility is explicitly assessed.
92. Forward fix may be safer than rollback.
93. Upgrade debt is visible.
94. Production connectors have active maintainers.
95. Custom extensions are documented.
96. Licensing obligations are reviewed.
97. Production secrets never enter connector source or patches.
98. Custom extensions provide operational telemetry.
99. Ambiguous connector outcomes are not falsely mapped to failure.
100. Custom connectors support UNKNOWN where necessary.
101. Provider idempotency supplements rather than replaces Baobab duplicate safety.
102. Provider webhook verification remains mandatory where supported.
103. Provider sync capability should support recovery where available.
104. Settlement provider logic does not leak into ERP.
105. Payout integrations follow dedicated Payout semantics.
106. Credential-affecting changes trigger PCI/security review.
107. Every production provider extension remains subject to certification.
108. Upstream compatibility is an explicit architectural objective.
109. Fork avoidance is an architectural objective, not an absolute prohibition.
110. Financial correctness always outranks convenience of upgrading or customising HyperSwitch.

---

# 189. Consequences

## Positive

This decision provides:

- controlled HyperSwitch evolution;
- lower upstream divergence;
- easier security upgrades;
- clearer provider-extension strategy;
- support for African/local PSP development;
- stronger canonical-domain isolation;
- reduced vendor implementation coupling;
- reproducible builds;
- safer migrations;
- governed configuration;
- explicit fork economics;
- maintainable upgrade paths.

## Negative

The policy can require more discipline than modifying upstream source directly.

It introduces:

```text id="xy8ahx"
adapter maintenance
compatibility matrices
patch registry
connector certification
upstream contribution work
upgrade testing
migration testing
release monitoring
```

These costs are accepted because unmanaged divergence creates greater long-term financial and security risk.

---

# 190. Alternatives Considered

## Maintain a Baobab HyperSwitch Fork from Day One

Rejected.

It creates unnecessary ownership of a rapidly evolving payment platform.

---

## Never Modify HyperSwitch Under Any Circumstances

Rejected.

Some strategically necessary capabilities may genuinely require upstream or temporary downstream modification.

---

## Integrate PSPs Directly into Baobab Payments Instead

Rejected as the default.

This would duplicate payment-orchestration functionality HyperSwitch was adopted to provide.

Provider-specific reconciliation adapters may be justified where authoritative settlement information is otherwise unavailable, but they remain behind Payments boundaries.

---

## Put All Baobab Logic into HyperSwitch

Rejected.

It would collapse canonical platform authority into an implementation dependency.

---

## Expose HyperSwitch Directly to Trade and Digital Estates

Rejected.

It bypasses canonical payment contracts and Baobab governance.

---

## Automatically Track Latest HyperSwitch

Rejected.

Payment infrastructure upgrades require controlled validation.

---

## Avoid Upgrades to Reduce Risk

Rejected.

Indefinite stagnation increases security, compatibility and maintenance risk.

---

# 191. Extension Decision Formula

For every proposed modification:

```text id="nt6fo3"
Can configuration solve it?
        │
        ▼
Can Baobab façade solve it?
        │
        ▼
Can supported HyperSwitch extension solve it?
        │
        ▼
Can a connector solve it?
        │
        ▼
Can it be contributed upstream?
        │
        ▼
Can a temporary patch bridge the gap?
        │
        ▼
Would a separate Baobab service be cleaner?
        │
        ▼
Only then:
Consider a permanent fork.
```

---

# 192. Upgrade Decision Formula

```text id="81nrzv"
UPSTREAM RELEASE
       │
       ▼
SECURITY + ARCHITECTURE REVIEW
       │
       ▼
DEPENDENCY / MIGRATION REVIEW
       │
       ▼
PATCH REBASE
       │
       ▼
ADAPTER COMPATIBILITY
       │
       ▼
CONNECTOR REGRESSION
       │
       ▼
CANONICAL CONTRACT TESTS
       │
       ▼
END-TO-END FINANCIAL TESTS
       │
       ▼
DR / RECOVERY VALIDATION
       │
       ▼
STAGING
       │
       ▼
CONTROLLED PRODUCTION ROLLOUT
       │
       ▼
FINANCIAL OBSERVATION
       │
       ▼
ADOPT
```

---

# 193. Final Architectural Boundary

```text id="qsfb4r"
             BAOBAB PLATFORM
                   │
                   ▼
       Canonical Payment Contracts
                   │
                   ▼
           baobab-payments
                   │
        ┌──────────┴──────────┐
        │                     │
        ▼                     ▼
Baobab Domain Logic    PaymentProvider Port
                              │
                              ▼
                      HyperSwitch Adapter
                              │
                              ▼
                    ┌───────────────────┐
                    │    HYPERSWITCH    │
                    │                   │
                    │ Core              │
                    │ Routing           │
                    │ Connectors        │
                    │ Configuration     │
                    │ Scheduler         │
                    └─────────┬─────────┘
                              │
                              ▼
                       Payment Providers

Extension preference:

CONFIG
  ↓
FAÇADE
  ↓
SUPPORTED EXTENSION
  ↓
CONNECTOR
  ↓
UPSTREAM CONTRIBUTION
  ↓
TEMPORARY PATCH
  ↓
SEPARATE SERVICE
  ↓
PERMANENT FORK
```

---

# 194. Final Decision

Baobab SHALL permanently distinguish:

```text id="6v2khv"
HYPERSWITCH
    from
BAOBAB PAYMENTS DOMAIN

IMPLEMENTATION MODEL
    from
CANONICAL MODEL

HYPERSWITCH ORGANISATION
    from
BAOBAB TENANT / ORGANISATION

HYPERSWITCH MERCHANT
    from
BAOBAB LEGAL ENTITY

BUSINESS PROFILE
    from
BAOBAB MARKET

CONFIGURATION
    from
CANONICAL AUTHORITY

CONNECTOR
    from
PROVIDER CERTIFICATION

PROVIDER SUPPORT
    from
PROVIDER ACTIVATION

UPSTREAM RELEASE
    from
APPROVED BAOBAB RELEASE

UPSTREAM IMAGE
    from
BAOBAB-PATCHED IMAGE

PATCH
    from
FORK

APPLICATION ROLLBACK
    from
FINANCIAL ROLLBACK

UPGRADE SUCCESS
    from
FINANCIAL CORRECTNESS

SANDBOX COMPATIBILITY
    from
PRODUCTION CERTIFICATION
```

The governing strategy is:

```text id="0w2o0z"
KEEP BAOBAB SEMANTICS IN BAOBAB
              │
              ▼
USE STABLE CANONICAL CONTRACTS
              │
              ▼
ISOLATE HYPERSWITCH BEHIND ADAPTER
              │
              ▼
CONFIGURE BEFORE MODIFYING
              │
              ▼
EXTEND THROUGH SUPPORTED BOUNDARIES
              │
              ▼
BUILD CONNECTORS FOR PROVIDER GAPS
              │
              ▼
CONTRIBUTE GENERAL CAPABILITIES UPSTREAM
              │
              ▼
USE TEMPORARY PATCHES ONLY WHEN NECESSARY
              │
              ▼
MEASURE AND REMOVE DIVERGENCE
              │
              ▼
FORK ONLY BY EXPLICIT ARCHITECTURAL DECISION
```

**HyperSwitch is a powerful payment-orchestration implementation, but Baobab's architecture must remain larger and more stable than any one HyperSwitch release. Baobab therefore isolates HyperSwitch behind canonical contracts and a dedicated adapter, extends providers through connectors wherever possible, contributes reusable improvements upstream, tightly governs temporary downstream patches, pins and tests production versions, and treats permanent forking as an exceptional architectural decision.**

**This permits Baobab to support payment realities across Uganda, South Africa and future African markets without turning every local payment requirement into permanent divergence from upstream HyperSwitch.**

**Baobab standardises the payment contract, not the payment implementation. HyperSwitch may evolve, connectors may change, dependencies may be introduced or removed, and providers may come and go; Baobab's canonical financial boundaries remain authoritative.**