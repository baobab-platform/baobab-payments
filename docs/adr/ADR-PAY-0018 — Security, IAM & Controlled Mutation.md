# ADR-PAY-0018 — Security, IAM & Controlled Mutation

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Security / IAM / Authorization |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0017; ADR-PAY-0022; applicable Baobab IAM, Control Plane and Shared ADRs |
| **Related** | ADR-PAY-0019 through ADR-PAY-0021 |
| **Primary Domains** | Authentication, authorization, workload identity, privileged operations, financial mutation, approvals, audit, separation of duties |

---

# 1. Context

`baobab-payments` executes operations capable of causing real financial consequences.

Examples include:

```text
authorise payment
capture payment
cancel/void payment
refund payment
create payout
approve payout
cancel payout
change beneficiary destination
activate provider
suspend provider
change routing configuration
activate FX capability
replay financial events
perform reconciliation correction
```

A valid authenticated identity alone is therefore insufficient.

Likewise, possession of a broad permission such as:

```text
payment:execute
```

cannot mean:

> execute any payment belonging to any Tenant, Legal Entity or Market.

Baobab requires contextual, least-privilege and auditable authorization.

---

# 2. Problem

Several dangerous equivalences must be prevented:

```text
Authenticated
    ≠
Authorised

Authorised for Payments
    ≠
Authorised for every payment

Tenant membership
    ≠
Legal Entity authority

Legal Entity authority
    ≠
Every financial mutation

Read authority
    ≠
Write authority

Write authority
    ≠
Approval authority

Approval authority
    ≠
Execution authority

Administrator
    ≠
Unlimited financial authority

Provider administrator
    ≠
Payout approver

Possession of Resource ID
    ≠
Authority over Resource
```

Without an explicit model, financial APIs risk becoming dependent on:

- coarse roles;
- caller-controlled context;
- service-level bearer secrets;
- implicit trust between engines;
- UI-only authorization;
- provider-console permissions;
- broad administrator access.

These are rejected.

---

# 3. Decision

Baobab Payments SHALL enforce layered authorization based on:

```text
Identity
   +
Authentication
   +
Workload/User Classification
   +
Capability / Scope
   +
Canonical Tenant Context
   +
Organisation Context
   +
Legal Entity Context
   +
Market Context where relevant
   +
Resource Ownership
   +
Operation
   +
Financial State
   +
Policy
   +
Approval where required
   =
Authorised Mutation
```

No single factor is sufficient.

---

# 4. Governing Principle

> **Every financial mutation must be attributable to an authenticated identity, authorised for the requested operation, within the canonical business context, against the correct resource, in a state where the operation is valid, and under any required approval policy.**

---

# 5. Security Authorities

Baobab SHALL preserve the following boundaries.

| Concern | Authority |
|---|---|
| Human/workload authentication | Baobab IAM |
| Enterprise identity lifecycle | Baobab IAM |
| Tenant / Organisation / Legal Entity / Market | Control Plane |
| Capability availability | Control Plane |
| Engine / Engine Instance | Control Plane |
| Capability Binding | Control Plane |
| Payment-resource ownership | Payments |
| Payment-state validity | Payments |
| Financial mutation policy | Payments within platform governance |
| Provider certification/activation | Payments |
| Payment execution | Payments |
| Provider routing | Payments / HyperSwitch within authorised envelope |
| Commerce obligation | Trade |
| Subscription obligation | Subscriptions |
| Accounting authority | ERP |
| Secrets | Approved secrets infrastructure |
| Provider identity | External operational identity only |

---

# 6. IAM Is the Identity Authority

`baobab-iam` SHALL remain the platform authority for:

```text
human identity
workload identity
authentication
identity lifecycle
credential lifecycle
revocation
federation
session management
```

Payments SHALL NOT create a competing enterprise identity system.

---

# 7. Keycloak Boundary

Keycloak is the implementation engine behind Baobab IAM.

Therefore:

```text
Keycloak User
    ≠
Baobab Tenant

Keycloak Group
    ≠
Baobab Organisation

Keycloak Role
    ≠ automatically
Baobab Legal Entity Authority

Keycloak Client
    ≠ automatically
Baobab Capability Binding
```

Baobab canonical context remains authoritative.

---

# 8. Authentication Versus Authorization

Authentication answers:

> Who or what is making this request?

Authorization answers:

> May this identity perform this operation on this resource in this context now?

These questions SHALL remain distinct.

---

# 9. Authentication Is Necessary but Insufficient

A valid token SHALL NOT by itself establish:

```text
Tenant ownership
Legal Entity authority
Market authority
resource ownership
payment eligibility
payout approval
provider activation authority
```

---

# 10. Identity Classes

Payments SHALL distinguish at least:

```text
Human Identity
Workload Identity
```

and MAY later distinguish additional controlled identities such as:

```text
automation identity
support identity
break-glass identity
```

without weakening the core authorization model.

---

# 11. Workload Identity

Baobab engines SHALL authenticate to Payments using workload identity.

Examples:

```text
baobab-trade
baobab-subscriptions
baobab-erp
baobab-cp
approved reconciliation worker
approved operations worker
```

---

# 12. No Anonymous Financial Mutation

Production financial mutation endpoints SHALL NOT permit anonymous access.

---

# 13. No Shared Platform Super-Credential

Baobab SHALL NOT use one universal credential granting every engine unrestricted access to Payments.

---

# 14. Distinct Workload Principals

Each calling engine/workload SHOULD have independently identifiable credentials or workload identity.

Conceptually:

```text
Trade Workload
Subscriptions Workload
ERP Workload
Control Plane Workload
Payments Worker
```

SHALL be distinguishable in audit.

---

# 15. Static Bearer Secrets

Long-lived static bearer secrets SHALL NOT be the normal production workload-authentication mechanism.

Where legacy/external integration temporarily requires one, its use SHALL be:

```text
explicit
scoped
rotatable
audited
time-bounded where possible
```

and treated as technical debt.

---

# 16. Workload Authentication

Preferred workload authentication SHOULD use standards-supported mechanisms such as:

```text
OAuth/OIDC client authentication
short-lived access tokens
workload identity federation
mTLS where appropriate
```

according to Baobab IAM and infrastructure architecture.

---

# 17. Token Audience

A token intended for another Baobab engine SHALL NOT automatically be accepted by Payments.

Payments SHALL validate the appropriate audience.

Conceptually:

```text
aud = baobab-payments
```

or equivalent governed audience semantics.

---

# 18. Token Validation

Payments SHALL validate applicable:

```text
issuer
signature
audience
expiry
not-before
token type
client/workload identity
```

and other IAM-required claims.

---

# 19. Revocation

Identity or workload revocation SHALL propagate sufficiently quickly to prevent continued privileged financial mutation.

Detailed IAM revocation architecture remains governed by Baobab IAM ADRs.

---

# 20. Capability Versus Permission

Control Plane capability binding determines whether a business context has access to a Payments capability.

IAM authorization determines whether the requesting identity may invoke the corresponding operation.

Payments additionally determines whether the specific resource and financial state permit it.

Therefore:

```text
Capability Binding
      +
IAM Permission
      +
Payments Domain Validation
      =
Potentially Authorised Operation
```

---

# 21. Capability Availability Is Not User Authorization

A Legal Entity having:

```text
payment.refund.create
```

does not mean every authenticated user associated with that Legal Entity may issue Refunds.

---

# 22. Scope Model

Existing canonical workload scopes include:

```text
payment:execute
payment:refund
payment:read
```

These remain useful coarse permissions.

Additional privileged scopes MAY be introduced through Shared/IAM governance.

---

# 23. Scope Families

Conceptually, Payments may require scopes covering:

```text
payment:read
payment:execute
payment:capture
payment:cancel
payment:refund

payout:read
payout:create
payout:approve
payout:cancel

beneficiary:read
beneficiary:manage
beneficiary-destination:manage

payment-provider:read
payment-provider:certify
payment-provider:activate
payment-provider:suspend
payment-provider:revoke

payment-routing:read
payment-routing:manage

payment-reconciliation:read
payment-reconciliation:manage

payment-event:replay

payment-fx:read
payment-fx:manage
```

Exact canonical scope vocabulary SHALL be defined through Shared/IAM contracts rather than independently invented by leaf applications.

---

# 24. Scope Is Not Complete Authorization

Possession of:

```text
payout:approve
```

SHALL NOT mean:

```text
approve every payout
```

Contextual authorization remains mandatory.

---

# 25. Contextual Authorization

For a financial resource `R`, Payments SHALL evaluate:

```text
Principal
   │
   ▼
Requested Operation
   │
   ▼
Token/Identity Permissions
   │
   ▼
Canonical Tenant
   │
   ▼
Organisation
   │
   ▼
Legal Entity
   │
   ▼
Market where relevant
   │
   ▼
Resource Ownership
   │
   ▼
Current Financial State
   │
   ▼
Applicable Policy
   │
   ▼
Approval Requirements
   │
   ▼
ALLOW / DENY
```

---

# 26. Server-Authoritative Context

Tenant, Organisation, Legal Entity and Market context SHALL be resolved from trusted canonical sources.

Caller-supplied values are not authority.

---

# 27. Tenant Isolation

An identity authorised within Tenant A SHALL NOT access Tenant B's payment resources merely by supplying Tenant B identifiers.

---

# 28. Legal Entity Isolation

An identity authorised for ZuriBeans SHALL NOT automatically gain authority over Thamani financial resources.

---

# 29. Group Membership

The fact that:

```text
ZuriBeans
Thamani
Equator & Estate Co.
```

share a corporate parent SHALL NOT widen Payments authorization.

---

# 30. External Customer Groups

The same rule applies to external Baobab customers with parent/subsidiary structures.

---

# 31. Organisation Context

Organisation membership MAY participate in authorization but SHALL NOT replace explicit Legal Entity/resource authorization where the operation is legally attributable to a Legal Entity.

---

# 32. Resource Ownership

Payments SHALL verify that the requested resource belongs to the resolved authorised context.

For example:

```text
Payment P1
tenant = T1
legal_entity = LE1
```

cannot be refunded under:

```text
tenant = T1
legal_entity = LE2
```

merely because the caller knows `P1`.

---

# 33. Object-ID Knowledge Is Not Authorization

Canonical resource identifiers SHALL be treated as identifiers, not capabilities.

---

# 34. No Insecure Direct Object Reference

Every resource operation SHALL perform server-side contextual authorization.

This includes:

```text
Payment
PaymentIntent
PaymentAttempt
Refund
Dispute
Settlement
Payout
Beneficiary
Beneficiary Destination
Provider Activation
FX resource
Reconciliation resource
```

---

# 35. Read Versus Mutation

Read permissions SHALL remain separable from mutation permissions.

---

# 36. Sensitive Read

Some read operations require greater privilege than ordinary payment status retrieval.

Examples include:

```text
provider configuration
commercial FX spreads
beneficiary information
settlement reports
dispute evidence
reconciliation exceptions
audit evidence
```

---

# 37. Field-Level Minimisation

Even when a caller may read a resource, Payments SHALL expose only fields required by that caller's legitimate function.

---

# 38. Trade Access

`baobab-trade` SHOULD normally be able to:

```text
create/execute authorised commerce payment operations
read payments linked to its own commerce source references
request commerce-authorised Refunds
```

within its canonical context.

It SHALL NOT thereby receive:

```text
provider credentials
global routing configuration
unrelated payouts
other tenants' transactions
raw card data
```

---

# 39. Subscription Access

`baobab-subscriptions` SHOULD normally be able to:

```text
create authorised collection attempts
read payment state for its billing obligations
```

within its context.

It SHALL NOT become provider/routing administrator.

---

# 40. ERP Access

ERP SHOULD normally consume authoritative financial facts and reconciliation projections.

ERP SHALL NOT receive unrestricted payment-execution authority merely because it owns accounting.

---

# 41. Control Plane Access

Control Plane may provision/bind Payments capabilities and context.

It SHALL NOT automatically gain permission to fabricate payment success, Refund completion or Payout completion.

---

# 42. Payments Workers

Internal Payments workers SHALL use explicit service identities and narrowly scoped privileges.

A recovery worker is not a universal administrator.

---

# 43. Human Administrative Access

Human access to production Payments administration SHALL be explicitly privileged.

Ordinary business-user authentication SHALL not imply administrative access.

---

# 44. Administrative Surfaces

Administrative capabilities MAY include:

```text
provider certification
provider activation
provider suspension
routing policy
reconciliation investigation
event replay
beneficiary review
payout approval
financial recovery
```

Each SHALL be separately governable.

---

# 45. HyperSwitch Control Center

HyperSwitch Control Center remains an operational administration surface.

Access to it SHALL be tightly restricted.

It SHALL NOT become the canonical Baobab authorization system.

---

# 46. Direct Provider Console

Provider-console access SHALL be treated as privileged external access.

It SHALL NOT be the normal path for Baobab financial operations.

---

# 47. Out-of-Band Mutation

Financial mutation performed directly through HyperSwitch/provider consoles outside canonical Baobab workflow SHALL be treated as:

```text
exceptional
auditable
reconcilable
```

and potentially a security incident depending on circumstances.

---

# 48. Controlled Mutation

A Controlled Mutation is any operation capable of materially changing:

```text
financial state
money movement
payment eligibility
provider eligibility
routing
beneficiary destination
settlement interpretation
```

Such mutations require enhanced authorization.

---

# 49. High-Risk Mutations

Examples include:

```text
capture
Refund
Payout
beneficiary destination change
provider activation
provider suspension
routing override
FX override
manual reconciliation adjustment
event replay
administrative state recovery
```

---

# 50. State-Aware Authorization

Authorization SHALL include domain-state validation.

For example:

```text
payment:refund
```

does not authorise refunding an uncaptured Payment.

---

# 51. Permission Cannot Override State Machine

IAM authorization SHALL NOT bypass PAY-0008 canonical lifecycle rules.

---

# 52. Permission Cannot Override Idempotency

IAM authorization SHALL NOT bypass PAY-0010 duplicate-safety requirements.

---

# 53. Permission Cannot Override Routing Eligibility

An administrator SHALL NOT use ordinary routing configuration to bypass PAY-0009/PAY-0022 hard eligibility constraints.

---

# 54. Permission Cannot Override Sensitive-Data Boundary

Administrator privilege SHALL NOT justify exposing PAN/CVV or secrets contrary to PAY-0012.

---

# 55. Permission Cannot Rewrite History

No normal role SHALL authorise editing historical financial facts in place.

Corrections require new controlled facts, reconciliation adjustments or explicitly governed recovery procedures.

---

# 56. Approval Versus Execution

For high-risk operations:

```text
Request
   ≠
Approval
   ≠
Execution
```

---

# 57. Maker-Checker

Baobab SHALL support separation of duties where policy requires:

```text
Maker
   │
   ▼
Request
   │
   ▼
Checker
   │
   ▼
Approval
   │
   ▼
Execution
```

---

# 58. Same-Person Restriction

Where four-eyes policy applies:

```text
requester_id
    ≠
approver_id
```

SHALL be enforced.

---

# 59. Workload Execution After Human Approval

A human approval MAY authorise a Payments workload to execute the approved operation.

The actual provider API call need not be performed using the human's credentials.

---

# 60. Approval Artifact

A controlled approval SHOULD preserve:

```text
approval_id
operation
resource_id
Tenant
Legal Entity
requester
approver
policy
amount/currency where relevant
created_at
approved_at
expires_at?
revision
```

---

# 61. Approval Binding

Approval SHALL bind to the material financial request.

---

# 62. Material Mutation Invalidates Approval

Changes to relevant:

```text
amount
currency
beneficiary
destination
Legal Entity
provider override
FX terms
```

SHALL invalidate prior approval where those fields formed part of the approved decision.

---

# 63. Approval Expiry

Approvals MAY expire.

An expired approval SHALL NOT execute.

---

# 64. Approval Is Not Reusable Authority

Approval for one financial mutation SHALL NOT silently authorise another.

---

# 65. Refund Controls

Refund authorization SHOULD consider:

```text
original Payment
Legal Entity
Refund amount
remaining refundable amount
requesting workload/user
reason
policy threshold
approval requirement
```

---

# 66. High-Value Refund

Policies MAY require additional approval above configured thresholds.

Thresholds SHALL be configuration/policy, not hardcoded business logic.

---

# 67. Payout Controls

PAY-0016 Payout authorization SHALL be particularly strict.

At minimum:

```text
payer Legal Entity
beneficiary
destination
amount
currency
source obligation
approval
provider eligibility
```

must remain coherent.

---

# 68. Beneficiary Destination Change

A destination change SHALL be treated independently from payout execution.

The ability to create Payouts SHALL NOT automatically grant the ability to change beneficiary bank details.

---

# 69. Provider Certification

The ability to inspect a provider SHALL not grant authority to certify it.

---

# 70. Provider Activation

Certification and activation SHALL use separately controlled mutation semantics.

---

# 71. Separation of Provider Duties

Where organisational policy requires:

```text
Integration Engineer
       │
       ▼
Provider Integration
       │
       ▼
Security/Compliance Reviewer
       │
       ▼
Certification
       │
       ▼
Authorised Business/Operations Role
       │
       ▼
Legal Entity Activation
```

SHOULD be supportable.

---

# 72. Routing Administration

Routing administrators MAY manage soft routing preferences.

They SHALL NOT be able to make uncertified/inactive providers eligible merely through routing configuration.

---

# 73. Hard Constraints

The following SHALL remain non-overridable through ordinary routing administration:

```text
Tenant isolation
Legal Entity isolation
environment isolation
provider certification
provider activation
currency eligibility
payment-method eligibility
required compliance eligibility
```

---

# 74. Emergency Suspension

Authorised operators SHALL be able to suspend:

```text
provider
connector
route
beneficiary destination
other high-risk execution path
```

where required.

Emergency suspension SHOULD favour stopping new financial execution.

---

# 75. Emergency Suspension Does Not Erase History

Suspension SHALL preserve:

```text
existing Payments
Refund rights
Disputes
Settlement
Reconciliation
Audit
```

as required.

---

# 76. Break-Glass Access

Baobab MAY support emergency break-glass access.

It SHALL NOT be an ordinary administrator role.

---

# 77. Break-Glass Properties

Break-glass access SHOULD be:

```text
explicitly activated
strongly authenticated
time-bound
narrowly scoped
highly visible
fully audited
reviewed afterwards
```

---

# 78. Break-Glass Does Not Mean Unlimited

Even break-glass authority SHALL NOT normally permit:

```text
cross-tenant data exfiltration
PAN/CVV disclosure
history rewriting
untracked money movement
```

---

# 79. Break-Glass Financial Mutation

Emergency financial mutation SHOULD require the highest feasible level of:

```text
justification
approval
audit
reconciliation
post-incident review
```

---

# 80. Support Access

Support personnel SHOULD normally receive read-only, masked and context-limited access.

---

# 81. Impersonation

If administrative impersonation is ever supported, the system SHALL preserve both:

```text
actor identity
impersonated subject identity
```

in audit.

Silent impersonation is prohibited.

---

# 82. Audit Identity

Every privileged mutation SHALL preserve the actual initiating identity.

---

# 83. No Generic “Admin” Audit

Audit entries such as:

```text
changed_by = admin
```

without attributable principal identity are insufficient.

---

# 84. Audit Record

A controlled mutation audit record SHOULD contain:

```text
audit_id
actor_type
actor_id
workload/client_id
operation
resource_type
resource_id
Tenant
Organisation where relevant
Legal Entity
Market where relevant
before_revision
after_revision
approval_reference?
reason
correlation_id
timestamp
outcome
```

---

# 85. Audit Immutability

Audit records SHALL be append-oriented and protected against ordinary mutation.

---

# 86. Audit Is Not Application Logging

Security/financial audit SHALL NOT depend solely on ordinary logs.

---

# 87. Audit Data Minimisation

Audit SHALL contain sufficient evidence without storing prohibited sensitive payment credentials.

---

# 88. Reason Codes

High-risk manual operations SHOULD require structured reason codes and MAY additionally accept explanatory text.

---

# 89. Correlation

Privileged operations SHALL retain correlation identifiers across:

```text
IAM
Payments
HyperSwitch
Provider
Event
ERP
```

where applicable.

---

# 90. Policy Evaluation

Payments MAY implement policy evaluation internally or through an approved policy component.

The architectural requirement is stable:

```text
Policy Decision
=
Identity
+
Context
+
Operation
+
Resource
+
State
+
Configured Rules
```

---

# 91. Policy Engine Independence

Canonical authorization contracts SHALL NOT become inseparably coupled to one policy-engine implementation.

---

# 92. Default Deny

Payments authorization SHALL follow:

```text
not explicitly allowed
        │
        ▼
       DENY
```

for privileged financial operations.

---

# 93. Unknown Context

If required canonical authorization context cannot be resolved:

```text
UNKNOWN
    │
    ▼
DENY
```

---

# 94. Ambiguous Context

Ambiguous Tenant, Legal Entity, Market or resource ownership SHALL fail closed.

---

# 95. Missing Capability Binding

If the required Payments capability is not bound to the canonical context, the operation SHALL fail.

---

# 96. Revoked Capability

A revoked/suspended capability SHALL block new relevant operations.

Historical records remain accessible according to retention and operational policy.

---

# 97. Token Claims Versus Canonical Context

Token claims MAY carry contextual hints or claims.

Payments SHALL validate them against canonical authority where required rather than trusting arbitrary caller-provided context.

---

# 98. Stale Authorization

Long-running financial workflows SHALL account for the possibility that authority changes between:

```text
request
approval
execution
```

---

# 99. Revalidation Before Execution

High-risk operations SHOULD revalidate material authorization immediately before external financial execution.

---

# 100. Approval Does Not Freeze All Security State

A valid approval does not override a subsequently:

```text
revoked identity
suspended provider
suspended beneficiary
disabled Legal Entity capability
```

---

# 101. Race Conditions

Authorization decisions and financial state transitions SHALL be protected against race conditions.

---

# 102. Transactional Mutation

Where practical:

```text
authorization-relevant state check
+
domain transition
+
audit/outbox intent
```

SHOULD occur within a coherent local transaction boundary.

External provider calls remain outside distributed ACID.

---

# 103. TOCTOU

Baobab SHALL minimise time-of-check/time-of-use vulnerabilities between authorization and financial execution.

---

# 104. Concurrent Approval

Two concurrent approvals SHALL NOT result in duplicate financial execution.

PAY-0010 remains authoritative for duplicate safety.

---

# 105. Event Consumers

Event consumption itself SHALL be authenticated/authorised at the transport/workload level.

---

# 106. Event Possession Is Not Mutation Authority

Receiving:

```text
payment.captured
```

does not grant a consumer permission to mutate Payments state.

---

# 107. Event Replay

Canonical event replay is privileged.

Replay SHALL NOT:

```text
re-execute PSP charge
re-execute Refund
re-execute Payout
```

merely because an event is being redelivered.

---

# 108. Replay Authorization

Replay SHOULD be constrained by:

```text
event type
consumer
Tenant
Legal Entity
time range
reason
operator
```

where practical.

---

# 109. Webhook Authentication

PAY-0011 webhook authentication is independent of IAM user authentication.

External provider callbacks use provider-specific verification.

---

# 110. Webhook Cannot Assume Human Authority

A cryptographically valid provider webhook establishes authenticated external provenance.

It does not establish arbitrary administrative authority.

---

# 111. Provider Credentials

Provider API credentials SHALL follow PAY-0012 secret-management requirements.

---

# 112. Secret Access

Only workloads that require a provider credential SHALL receive access to it.

---

# 113. Trade Does Not Receive PSP Credentials

`baobab-trade` SHALL NOT receive provider credentials merely because it initiates customer Payments.

---

# 114. Subscriptions Does Not Receive PSP Credentials

The same applies to `baobab-subscriptions`.

---

# 115. Control Plane Does Not Become Secret Vault

Control Plane SHALL not hold payment-provider credentials as canonical context.

---

# 116. Credential Rotation

Provider credential rotation SHALL not require widening access to downstream engines.

---

# 117. GitHub / CI Security

Production financial credentials SHALL NOT be:

```text
committed to repository
embedded in container image
printed in CI logs
stored in example environment files
hardcoded in tests
```

---

# 118. Deployment Identity

CI/CD deployment identity SHALL remain separate from runtime financial-execution identity.

---

# 119. Deployment Permission ≠ Payment Permission

The ability to deploy `baobab-payments` SHALL NOT automatically grant authority to execute production Payouts or Refunds.

---

# 120. Runtime Environment

Production SHALL enforce:

```text
production IAM configuration
production secrets
production provider mappings
production capability bindings
```

without accepting development identities.

---

# 121. Sandbox Isolation

Sandbox credentials and permissions SHALL remain isolated from production.

---

# 122. Sandbox Authority

An identity authorised for sandbox testing SHALL NOT automatically receive production financial authority.

---

# 123. Simulated Operations

Simulated payment operations SHALL retain actor/workload attribution even though no real money moves.

---

# 124. Human MFA

Privileged human administrative access SHOULD require strong multi-factor authentication according to Baobab IAM security policy.

---

# 125. High-Risk Reauthentication

Baobab MAY require recent/step-up authentication for especially sensitive administrative operations.

---

# 126. Session Security

Privileged administrative sessions SHOULD have appropriately restrictive:

```text
lifetime
idle timeout
revocation
reauthentication
```

semantics.

---

# 127. Machine Credential Rotation

Workload credentials SHALL support controlled rotation without requiring shared long-lived secrets.

---

# 128. Least Privilege

Every principal SHALL receive the minimum authority required for its function.

---

# 129. No Role Explosion in Leaf Applications

Leaf digital estates SHALL NOT invent competing platform-wide payment authorization taxonomies.

Shared/IAM contracts SHALL provide canonical authorization vocabulary.

---

# 130. Business Roles

Business-facing roles MAY map into canonical permissions.

For example:

```text
Finance Approver
Operations Analyst
Payment Operations Administrator
```

but role names themselves SHALL not become the payment-domain authorization contract.

---

# 131. Role ≠ Permission

```text
Role
   │
   ▼
Permission Set
   │
   ▼
Contextual Authorization
```

is preferred over:

```text
if role == "admin":
    allow_everything()
```

---

# 132. Attribute-Based Context

Payments authorization SHOULD support contextual attributes such as:

```text
Tenant
Legal Entity
Market
operation
resource owner
amount threshold
currency
transaction type
environment
```

where relevant.

---

# 133. Relationship-Based Context

Where Baobab later requires richer organisation relationships, canonical Control Plane relationships MAY participate in authorization.

They SHALL not silently widen financial authority.

---

# 134. Parent Organisation

Parent-company authority over subsidiary financial operations SHALL be explicit.

It SHALL NOT be inferred solely from corporate hierarchy.

---

# 135. Group Finance

If Nabhold Group Finance later requires oversight across subsidiaries, its rights SHALL be intentionally granted.

Examples may include:

```text
cross-subsidiary read
consolidated reconciliation view
approval authority
```

Each is separate.

---

# 136. Consolidated Read ≠ Mutation

A group-level executive permitted to view consolidated payment data SHALL NOT thereby receive Refund/Payout execution authority.

---

# 137. Cross-Tenant Administration

Baobab platform operators MAY require operational visibility across tenants.

Such access SHALL be exceptional, least-privilege and auditable.

---

# 138. Platform Operator ≠ Tenant Finance User

Baobab infrastructure administration does not automatically grant authority to make a customer's financial decisions.

---

# 139. Database Access

Direct production database access SHALL NOT be the normal mechanism for financial mutation.

---

# 140. Manual SQL Mutation

Manual SQL modification of canonical payment state is prohibited as an ordinary operational procedure.

---

# 141. Recovery Operations

Where exceptional recovery is required, it SHALL use controlled tooling/workflows that preserve:

```text
authorization
validation
audit
correlation
state-machine invariants
```

---

# 142. Database Administrator

Database administrative privilege SHALL NOT be treated as business authorization.

---

# 143. Infrastructure Administrator

Likewise:

```text
Kubernetes/cloud administrator
    ≠
financial approver
```

---

# 144. Security Administrator

IAM/security administration SHALL not automatically grant payment execution authority.

---

# 145. Separation of Technical and Financial Authority

Baobab SHOULD preserve:

```text
Technical Administration
        ≠
Financial Authorization
```

where operationally feasible.

---

# 146. Reconciliation Mutation

PAY-0015 reconciliation corrections SHALL be controlled mutations.

---

# 147. Reconciliation Cannot Fabricate Provider Success

A reconciliation operator SHALL NOT mark a Payment captured merely to clear an exception without authoritative evidence.

---

# 148. Manual Matching

Manual reconciliation matching SHALL preserve:

```text
operator
reason
evidence
before state
after state
```

---

# 149. Dispute Evidence

PAY-0014 dispute evidence access SHALL be separately controlled because it may contain:

```text
customer data
commercial evidence
shipping data
communication records
financial evidence
```

---

# 150. Evidence Mutation

Submitted dispute evidence SHALL preserve provenance and history.

It SHALL not be silently replaced after submission.

---

# 151. FX Administration

PAY-0017 FX configuration is privileged.

---

# 152. FX Override

Manual FX-rate override, if supported at all, SHALL require explicit authorization and strong audit.

It SHALL NOT be an ordinary payment-execution permission.

---

# 153. Provider Certification Security

PAY-0022 provider certification SHALL include security review appropriate to the integration.

---

# 154. Certification Evidence

Certification evidence SHOULD identify:

```text
reviewer
evidence
provider/connector version
environment
scope
decision
validity period
```

---

# 155. Activation Authority

Provider activation SHALL require an identity authorised for the affected Legal Entity/context.

---

# 156. Global Activation Prohibited by Default

A provider SHALL NOT become globally active across all tenants/Legal Entities merely because one operator activates it for one context.

---

# 157. Configuration Versioning

Security-sensitive configuration SHOULD be versioned.

---

# 158. Policy Version

Where material, Payments SHOULD preserve which authorization/policy revision governed a high-risk mutation.

---

# 159. Audit Correlation

A privileged financial operation SHOULD be reconstructable as:

```text
Identity
   │
   ▼
Authentication
   │
   ▼
Authorization Decision
   │
   ▼
Approval
   │
   ▼
Financial Command
   │
   ▼
Canonical State Transition
   │
   ▼
Provider Operation
   │
   ▼
Canonical Event
   │
   ▼
ERP / Consumer Projection
```

---

# 160. Authorization Decision Logging

High-risk authorization decisions SHOULD provide enough structured evidence to explain:

```text
who
requested what
against which resource
under which Tenant/Legal Entity
which permissions applied
which policy applied
why allowed/denied
when
```

without exposing secrets.

---

# 161. Denial Logging

Security-relevant denied operations SHOULD be observable.

---

# 162. Enumeration Protection

Authorization errors SHOULD avoid unnecessarily revealing whether cross-tenant resources exist.

---

# 163. Error Semantics

Canonical authorization failures MAY distinguish internally:

```text
UNAUTHENTICATED
UNAUTHORIZED
CAPABILITY_NOT_BOUND
CONTEXT_MISMATCH
RESOURCE_NOT_ACCESSIBLE
APPROVAL_REQUIRED
APPROVAL_EXPIRED
POLICY_DENIED
```

while public responses remain appropriately information-minimising.

---

# 164. Rate Limiting

Sensitive financial/admin endpoints SHOULD support appropriate rate limiting and abuse controls.

---

# 165. Automated Abuse

A valid credential exhibiting anomalous behaviour MAY be:

```text
throttled
challenged
suspended
revoked
```

according to security policy.

---

# 166. Security Event Integration

Payments SHOULD emit or forward security-relevant operational signals for:

```text
repeated authorization failures
cross-tenant attempts
cross-Legal-Entity attempts
approval bypass attempts
unexpected provider-console mutation
break-glass activation
credential failure spikes
```

---

# 167. Incident Response

Security incidents MAY require immediate:

```text
principal revocation
provider suspension
route suspension
beneficiary suspension
credential rotation
```

without rewriting historical financial facts.

---

# 168. Availability Versus Security

Baobab SHALL prefer controlled financial unavailability over unauthorised money movement when authorization state cannot be established safely.

---

# 169. IAM Outage

If IAM validity cannot safely be established for a new privileged mutation, Payments SHALL fail closed.

---

# 170. Control Plane Outage

Payments MAY use appropriately governed cached canonical context where architecture permits and freshness is provable.

It SHALL NOT invent missing Tenant/Legal Entity authority.

---

# 171. Authorization Cache

Authorization/context caching SHALL be:

```text
scoped
short enough for risk
revocation-aware where feasible
version-aware
```

and SHALL not create cross-tenant leakage.

---

# 172. Cached Permission Is Not Permanent Authority

A revoked permission SHALL not remain effective indefinitely because a service cached it.

---

# 173. Multi-Region Authorization

Multi-region deployments SHALL preserve equivalent authorization semantics.

---

# 174. Regional Failover

Failover to another region SHALL NOT weaken:

```text
identity validation
Tenant isolation
Legal Entity isolation
approval
capability binding
provider activation
```

---

# 175. Split-Brain

Where authorization or financial state cannot be made coherent during split-brain conditions, new high-risk financial mutation SHOULD fail closed.

---

# 176. Disaster Recovery

PAY-0020 recovery SHALL restore:

```text
canonical financial state
idempotency state
approval state
authorization-relevant mappings
provider activation state
audit continuity
```

before unrestricted financial execution resumes.

---

# 177. No DR Superuser Shortcut

Disaster recovery SHALL NOT introduce a permanent bypass credential to restore financial execution.

---

# 178. API Boundary

Canonical Payments APIs SHALL require authentication and authorization middleware before financial domain mutation.

---

# 179. Domain Revalidation

Middleware authorization does not eliminate domain-layer validation.

The domain SHALL still verify:

```text
resource ownership
state
amount limits
idempotency
approval
eligibility
```

---

# 180. Defence in Depth

The preferred model is:

```text
Network Controls
      │
      ▼
Authentication
      │
      ▼
API Authorization
      │
      ▼
Canonical Context Validation
      │
      ▼
Domain Authorization
      │
      ▼
State Machine
      │
      ▼
Idempotency
      │
      ▼
Provider Eligibility
      │
      ▼
External Financial Operation
```

No single layer replaces another.

---

# 181. Service-to-Service Flow

```text
baobab-trade
      │
      │ workload token
      ▼
baobab-payments
      │
      ├─ validate identity
      ├─ validate audience
      ├─ validate payment:execute
      ├─ resolve Tenant
      ├─ resolve Legal Entity
      ├─ validate source ownership
      ├─ validate Capability Binding
      ├─ validate payment state
      ├─ validate idempotency
      └─ validate provider eligibility
                │
                ▼
             Execute
```

---

# 182. Human Payout Flow

```text
Finance User
     │
     ▼
Baobab IAM
     │
     ▼
Authenticated Identity
     │
     ▼
Request Payout
     │
     ▼
Contextual Authorization
     │
     ▼
PENDING APPROVAL
     │
     ▼
Independent Approver
     │
     ▼
Approval Validation
     │
     ▼
Payments Workload
     │
     ▼
Revalidate Security + Context
     │
     ▼
Execute Payout
```

---

# 183. Provider Activation Flow

```text
Provider Integrated
       │
       ▼
Technical Evidence
       │
       ▼
Certification Authority
       │
       ▼
CERTIFIED
       │
       ▼
Legal Entity Activation Request
       │
       ▼
Authorised Activator
       │
       ▼
Context Validation
       │
       ▼
ACTIVE
       │
       ▼
Eligible for PAY-0009 routing evaluation
```

---

# 184. Cross-Tenant Attack Example

```text
Principal from Tenant A
        │
        ▼
Requests Refund for Payment P9
        │
        ▼
P9 belongs to Tenant B
        │
        ▼
Context mismatch
        │
        ▼
DENY
        │
        ▼
Security audit / telemetry
```

Knowing `P9` does not create authority.

---

# 185. Legal-Entity Attack Example

```text
ZuriBeans workload
       │
       ▼
requests operation
       │
       ▼
Thamani Payment
       │
       ▼
Legal Entity mismatch
       │
       ▼
DENY
```

Group ownership does not change the result.

---

# 186. Break-Glass Flow

```text
Normal operation unavailable
       │
       ▼
Emergency condition declared
       │
       ▼
Break-glass request
       │
       ▼
Strong authentication
       │
       ▼
Authorisation / justification
       │
       ▼
Time-bound elevated access
       │
       ▼
Controlled operation
       │
       ▼
Immutable audit
       │
       ▼
Automatic expiry
       │
       ▼
Post-event review
```

---

# 187. Production Readiness Gate

```text
SECURITY / IAM / CONTROLLED MUTATION READINESS

IAM integration                              PASS
Workload identities                          PASS
Human identities                             PASS
Audience validation                          PASS
Token validation                             PASS
Revocation                                   PASS
Least privilege                              PASS
Canonical scopes                             PASS
Capability Binding validation                PASS
Tenant isolation                             PASS
Organisation context                         PASS
Legal Entity isolation                       PASS
Market context                               PASS
Resource ownership                           PASS
State-aware authorization                    PASS
Read/write separation                        PASS
Sensitive-read controls                      PASS
Maker-checker                                PASS
Approval binding                             PASS
Approval expiry                              PASS
Material-change invalidation                 PASS
Payout controls                              PASS
Refund controls                              PASS
Beneficiary mutation controls                PASS
Provider certification controls              PASS
Provider activation controls                 PASS
Routing administration controls              PASS
FX controls                                  PASS
Reconciliation controls                      PASS
Event replay controls                        PASS
Break-glass                                  PASS
Support access                               PASS
Immutable audit                              PASS
Reason/evidence capture                      PASS
Secret isolation                             PASS
CI/CD identity separation                    PASS
Sandbox/prod identity separation             PASS
Cross-tenant tests                           PASS
Cross-Legal-Entity tests                     PASS
Multi-region consistency                     PASS
DR authorization restoration                 PASS
Security telemetry                           PASS
Incident-response controls                   PASS
-------------------------------------------------
Security & Controlled Mutation Plane          READY
```

---

# 188. Required Test Matrix

Implementation SHALL test at least:

```text
valid workload + valid scope + valid context
missing authentication
expired token
wrong issuer
wrong audience
missing scope
missing Capability Binding
wrong Tenant
wrong Legal Entity
wrong Market where relevant
known resource ID from another Tenant
known resource ID from sibling Legal Entity
read allowed / write denied
refund scope without refundable Payment
payout create without approval
same user maker/checker where prohibited
expired approval
approval after material amount change
approval after destination change
provider activation without certification
provider activation for wrong Legal Entity
routing admin attempts ineligible provider
event replay without replay authority
event replay does not execute money movement
break-glass expiry
revoked workload credential
sandbox identity against production
production identity against forbidden context
concurrent financial mutation
stale authorization during execution
IAM outage
Control Plane context unavailable
multi-region failover
direct provider-console reconciliation anomaly
audit completeness
sensitive-data redaction
```

---

# 189. Invariants

The following invariants SHALL hold:

1. Authentication is not authorization.
2. Authorization is contextual.
3. Identity does not establish Tenant ownership by itself.
4. Tenant membership does not establish Legal Entity authority.
5. Legal Entity authority does not grant every financial mutation.
6. Resource-ID knowledge is not authorization.
7. Every financial resource operation performs server-side authorization.
8. Control Plane remains canonical for Tenant/Organisation/Legal Entity/Market context.
9. IAM remains authoritative for identity.
10. Payments remains authoritative for payment-resource ownership and state.
11. Keycloak objects do not redefine canonical business hierarchy.
12. Capability Binding is not user permission.
13. User permission is not Capability Binding.
14. Both may be required.
15. Scope alone is not complete authorization.
16. Resource ownership is validated.
17. Cross-tenant access fails closed.
18. Cross-Legal-Entity access fails closed.
19. Parent ownership does not imply subsidiary financial authority.
20. Sibling subsidiaries do not inherit authority.
21. External tenants receive equivalent isolation.
22. Read authority is separable from mutation authority.
23. Sensitive reads receive additional controls.
24. Workloads have attributable identities.
25. A universal platform financial credential is prohibited.
26. Long-lived static bearer credentials are not the preferred production model.
27. Token audience is validated.
28. Revocation is respected.
29. Anonymous production financial mutation is prohibited.
30. Trade does not receive provider credentials.
31. Subscriptions does not receive provider credentials.
32. ERP accounting authority does not imply payment execution authority.
33. Control Plane authority does not imply authority to fabricate financial state.
34. Technical administration is not financial approval.
35. Database administration is not financial approval.
36. Deployment authority is not payment authority.
37. IAM administration is not payment execution authority.
38. Permission cannot bypass the state machine.
39. Permission cannot bypass idempotency.
40. Permission cannot bypass provider eligibility.
41. Permission cannot bypass PCI/sensitive-data boundaries.
42. Permission cannot rewrite financial history.
43. Request is not approval.
44. Approval is not execution.
45. Maker-checker is enforced where configured.
46. Material changes invalidate approval where required.
47. Approval may expire.
48. Approval cannot silently be reused.
49. Payout destination change is separately authorised.
50. Provider certification and activation remain separate controlled mutations.
51. Routing administration cannot create provider eligibility.
52. Emergency suspension does not erase history.
53. Break-glass access is exceptional.
54. Break-glass access is time-bound where feasible.
55. Break-glass use is fully audited.
56. Support access is least privilege.
57. Impersonation preserves actor identity.
58. Privileged mutations have attributable audit identities.
59. Audit is not ordinary application logging.
60. Audit records are append-oriented.
61. Audit does not contain prohibited sensitive credentials.
62. Default privileged authorization is deny.
63. Unknown required context fails closed.
64. Ambiguous context fails closed.
65. Missing Capability Binding fails closed.
66. High-risk execution revalidates security state where appropriate.
67. Stale approval does not override later suspension/revocation.
68. Authorization checks are concurrency-safe.
69. Event receipt does not grant mutation authority.
70. Event replay does not re-execute money movement.
71. Provider webhook authentication is separate from IAM.
72. Provider credentials are least privilege.
73. Secrets are not committed to repositories.
74. CI/CD identity is distinct from runtime financial identity.
75. Sandbox identity does not imply production authority.
76. Human privileged access uses strong authentication policy.
77. Leaf estates do not invent platform-wide payment authorization semantics.
78. Role names are not canonical permissions.
79. Consolidated read authority does not imply mutation authority.
80. Direct SQL is not a normal financial mutation path.
81. Recovery preserves authorization and audit.
82. Reconciliation cannot fabricate financial success.
83. FX override is separately privileged.
84. Provider activation is scoped to authorised context.
85. Global activation is not implied.
86. Authorization policy revisions are traceable where material.
87. Denied cross-boundary operations are observable.
88. Security errors minimise information leakage.
89. Security incident response may suspend new execution without rewriting history.
90. IAM uncertainty fails closed for privileged mutation.
91. Cached authorization does not become permanent authority.
92. Regional failover cannot weaken authorization.
93. Split-brain does not justify unsafe financial mutation.
94. DR does not introduce a permanent bypass credential.
95. API authorization does not replace domain authorization.
96. Domain authorization does not replace idempotency.
97. Idempotency does not replace authorization.
98. Availability does not override authority.
99. Administrative convenience does not override Legal Entity isolation.
100. Every real-money mutation remains attributable, authorised, state-valid, duplicate-safe and auditable.

---

# 190. Consequences

## Positive

This decision provides:

- strong Tenant isolation;
- strong Legal Entity isolation;
- least-privilege financial access;
- explicit workload identities;
- controlled Refunds and Payouts;
- maker-checker support;
- provider governance;
- secure event replay;
- controlled recovery;
- immutable accountability;
- separation of technical and financial authority;
- external-tenant readiness;
- multi-region security consistency.

## Negative

It introduces additional complexity in:

- IAM integration;
- permission contracts;
- contextual authorization;
- approval workflows;
- administrative tooling;
- policy evaluation;
- audit persistence;
- incident operations;
- testing.

These costs are accepted because financial mutation cannot safely depend on coarse authentication or generic administrator privileges.

---

# 191. Alternatives Considered

## Trust Any Authenticated Baobab Service

Rejected.

A compromised or defective service would receive excessive financial authority.

---

## One Shared Payments API Key

Rejected.

It prevents meaningful attribution, isolation, revocation and least privilege.

---

## Trust Tenant ID Supplied by Caller

Rejected.

Canonical context must be server-authoritative.

---

## Use Keycloak Roles Alone

Rejected.

Financial authorization requires canonical business context, resource ownership and state.

---

## Let UI Hide Unauthorized Actions

Rejected.

UI controls are not security boundaries.

---

## Give Administrators Universal Financial Authority

Rejected.

Technical administration and financial authority must remain separable.

---

## Use Database Permissions for Financial Authorization

Rejected.

Database access does not encode business authority or state-machine semantics.

---

## Let HyperSwitch Determine Baobab Authorization

Rejected.

HyperSwitch does not own Baobab Tenant, Legal Entity or business authority.

---

## Allow Emergency Operations Without Audit

Rejected.

Emergency access requires more accountability, not less.

---

# 192. Relationship to PAY-0001

PAY-0001 establishes Payments as a headless engine behind Baobab contracts.

PAY-0018 ensures all access to that engine respects Baobab identity and authorization boundaries rather than HyperSwitch-native identity alone.

---

# 193. Relationship to PAY-0003

PAY-0003 separates canonical Tenant/Organisation from HyperSwitch organisational objects.

PAY-0018 makes canonical context part of authorization.

---

# 194. Relationship to PAY-0004

PAY-0004 establishes Legal Entity as the canonical business principal.

PAY-0018 makes Legal Entity authority a core financial-security boundary.

---

# 195. Relationship to PAY-0005

Market/currency/payment-method context may participate in authorization but does not replace Tenant or Legal Entity authority.

---

# 196. Relationship to PAY-0006

Trade receives only the Payments authority necessary to execute commerce payment workflows within its own context.

---

# 197. Relationship to PAY-0007

Subscriptions receives only the authority necessary to execute authorised monetary billing obligations.

---

# 198. Relationship to PAY-0008

IAM determines whether an operation may be requested.

The canonical state machine determines whether the requested transition is financially valid.

Both must permit the operation.

---

# 199. Relationship to PAY-0009

Routing administration cannot bypass hard eligibility boundaries.

---

# 200. Relationship to PAY-0010

An operation must be both:

```text
AUTHORISED
AND
DUPLICATE-SAFE
```

before financial execution.

---

# 201. Relationship to PAY-0011

Event ingestion, publication, consumption and replay each require appropriate security boundaries.

---

# 202. Relationship to PAY-0012

Authorization SHALL minimise access to sensitive payment information and provider secrets.

Administrative privilege does not override PCI/sensitive-data controls.

---

# 203. Relationship to PAY-0013

Refund creation and execution are controlled financial mutations with explicit ownership, amount and approval checks.

---

# 204. Relationship to PAY-0014

Dispute evidence access/submission requires dedicated authorization and immutable provenance.

---

# 205. Relationship to PAY-0015

Settlement and reconciliation operations receive dedicated read/mutation permissions.

Manual reconciliation does not grant authority to fabricate provider facts.

---

# 206. Relationship to PAY-0016

Payout creation, approval, beneficiary management and execution SHALL remain independently controllable.

Payout security is a primary application of maker-checker and separation of duties.

---

# 207. Relationship to PAY-0017

FX configuration, currency-pair activation and any manual FX override are privileged operations.

---

# 208. Relationship to PAY-0019

PAY-0019 SHALL operationalise monitoring for:

```text
authentication failures
authorization failures
cross-tenant attempts
cross-Legal-Entity attempts
privileged mutations
break-glass use
provider-console drift
approval backlog
security-related provider suspension
```

---

# 209. Relationship to PAY-0020

PAY-0020 SHALL ensure disaster recovery restores authorization-relevant state before unrestricted money movement resumes.

---

# 210. Relationship to PAY-0021

HyperSwitch upgrades SHALL not weaken Baobab's security boundary or silently introduce new administrative paths around Payments authorization.

---

# 211. Relationship to PAY-0022

Provider certification and Legal Entity activation are privileged, independently auditable mutations.

Certification authority SHALL not automatically imply global activation authority.

---

# 212. Complete Authorization Architecture

```text
                    REQUEST
                       │
                       ▼
                  BAOBAB IAM
                       │
                       ▼
             Authenticated Principal
                       │
                       ▼
                Token Validation
                       │
                       ▼
              Capability / Scope
                       │
                       ▼
               CONTROL PLANE
                       │
             ┌─────────┼─────────┐
             ▼         ▼         ▼
          Tenant  Organisation Legal Entity
                                  │
                                  ▼
                               Market
                                  │
                                  ▼
                           Capability Binding
                                  │
                                  ▼
                         BAOBAB PAYMENTS
                                  │
                     ┌────────────┼────────────┐
                     ▼            ▼            ▼
                  Resource       State        Policy
                  Ownership    Validation   Evaluation
                     │            │            │
                     └────────────┼────────────┘
                                  ▼
                         Approval Required?
                          ┌───────┴────────┐
                          ▼                ▼
                         NO               YES
                          │                │
                          │                ▼
                          │           Valid Approval
                          │                │
                          └────────┬───────┘
                                   ▼
                              Idempotency
                                   │
                                   ▼
                          Provider Eligibility
                                   │
                                   ▼
                              EXECUTION
                                   │
                                   ▼
                         Canonical State Fact
                                   │
                      ┌────────────┼────────────┐
                      ▼            ▼            ▼
                    Audit        Events        ERP
```

---

# 213. Final Decision

Baobab SHALL permanently distinguish:

```text
IDENTITY
    from
AUTHORITY

AUTHENTICATION
    from
AUTHORIZATION

ROLE
    from
PERMISSION

PERMISSION
    from
CONTEXTUAL AUTHORITY

CAPABILITY BINDING
    from
USER AUTHORIZATION

TENANT AUTHORITY
    from
LEGAL ENTITY AUTHORITY

READ AUTHORITY
    from
MUTATION AUTHORITY

REQUEST
    from
APPROVAL

APPROVAL
    from
EXECUTION

TECHNICAL ADMINISTRATION
    from
FINANCIAL AUTHORITY

DATABASE ACCESS
    from
BUSINESS AUTHORITY

DEPLOYMENT AUTHORITY
    from
PAYMENT AUTHORITY

BREAK-GLASS ACCESS
    from
UNLIMITED ACCESS
```

The authoritative security model is:

```text
IAM
determines authenticated identity
        │
        ▼
CONTROL PLANE
determines canonical business context
        │
        ▼
CAPABILITY BINDING
determines whether the capability exists there
        │
        ▼
IAM / AUTHORIZATION POLICY
determines whether the principal may request the operation
        │
        ▼
PAYMENTS
determines resource ownership + financial state
        │
        ▼
APPROVAL POLICY
determines whether additional authority is required
        │
        ▼
IDEMPOTENCY / ELIGIBILITY
determine whether execution is safe
        │
        ▼
HYPERSWITCH / PROVIDER
performs only the authorised external operation
        │
        ▼
PAYMENTS
records canonical financial truth
        │
        ▼
AUDIT + EVENTS + ERP
preserve accountability and downstream consequences
```

**IAM proves who is acting.  
The Control Plane establishes where that identity is acting.  
Capability Binding establishes whether Payments is available in that context.  
Authorization policy determines what the identity may request.  
Payments determines whether the resource belongs to that context and whether the financial operation is valid.  
Approval policy supplies additional authority where required.  
Idempotency and eligibility determine whether execution is safe.  
HyperSwitch and providers execute only the resulting authorised operation.  
Audit preserves who did what, where, when, under which authority and with what outcome.**

**No authenticated identity is inherently trusted with money movement. No platform administrator is inherently a financial approver. No corporate relationship silently grants authority over another Legal Entity. No provider console, database credential, deployment permission, role name, resource identifier, emergency condition or availability objective may substitute for explicit, contextual, attributable and auditable financial authority.**