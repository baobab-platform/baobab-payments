# ADR-PAY-0020 — Availability, Backup, Recovery & Disaster Resilience

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Reliability / Disaster Recovery |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0019; ADR-PAY-0022; applicable Shared, Control Plane, IAM, ERP and Infrastructure ADRs |
| **Related** | ADR-PAY-0021 |
| **Primary Domains** | Availability, resilience, backup, restore, RPO, RTO, failover, financial-state recovery, disaster recovery |

---

# 1. Context

`baobab-payments` participates directly in real-money movement.

Its state includes more than ordinary application data.

It may contain:

```text id="ycp3f5"
PaymentIntent
Payment
PaymentAttempt
Refund
Dispute
Settlement
Payout
Beneficiary reference
idempotency record
provider reference
canonical state revision
webhook inbox state
event outbox state
approval state
provider mapping
routing evidence
reconciliation state
audit evidence
```

Loss, duplication or inconsistency in this state can create real financial consequences.

A conventional recovery model such as:

```text id="l1bbmi"
restore database
restart service
resume traffic
```

is therefore insufficient.

---

# 2. Problem

A payment platform can recover technically while remaining financially unsafe.

For example:

```text id="4m0fzg"
Provider charged customer
        │
        ▼
Payments crashed before local commit
        │
        ▼
Backup restored from earlier point
        │
        ▼
Baobab no longer sees execution
        │
        ▼
Request replayed
        │
        ▼
Potential duplicate charge
```

Likewise:

```text id="qg56pr"
Payout executed externally
       │
       ▼
local canonical state lost
       │
       ▼
system restored
       │
       ▼
Payout appears pending
       │
       ▼
operator retries
       │
       ▼
duplicate outbound money movement
```

Recovery must therefore establish more than infrastructure health.

It must establish financial continuity.

---

# 3. Decision

Baobab Payments SHALL implement disaster resilience around four separate objectives:

```text id="20hrk5"
1. Technical Availability
2. Durable Canonical State
3. Financial Execution Safety
4. Financial Reconciliation
```

Recovery SHALL proceed through controlled stages:

```text id="vix34w"
FAILURE
   │
   ▼
CONTAIN
   │
   ▼
PRESERVE
   │
   ▼
RESTORE
   │
   ▼
VALIDATE
   │
   ▼
RECONCILE
   │
   ▼
PROVE EXECUTION SAFETY
   │
   ▼
RESUME CONTROLLED TRAFFIC
   │
   ▼
VERIFY
```

---

# 4. Governing Principle

> **Infrastructure recovery does not establish financial recovery. Real-money execution resumes only after Baobab can establish that canonical state, duplicate-safety state, external financial reality and execution authority are sufficiently coherent to execute safely.**

---

# 5. Availability Model

Baobab SHALL distinguish:

```text id="9q6cvi"
Process Availability
Service Availability
Capability Availability
Provider Availability
Financial Execution Availability
Reconciliation Availability
```

---

# 6. Process Availability

A running Payments process only proves that the process is alive.

It does not establish that financial execution is safe.

---

# 7. Service Availability

The Payments API may be reachable while individual financial capabilities are unavailable.

For example:

```text id="zvqx61"
Payment reads           AVAILABLE
New authorisations      SUSPENDED
Captures                SUSPENDED
Refunds                 AVAILABLE
Payouts                 SUSPENDED
Reconciliation          AVAILABLE
Webhook ingestion       AVAILABLE
```

This is a valid degraded operating mode.

---

# 8. Capability-Aware Availability

Baobab SHOULD prefer capability-specific degradation over unnecessary platform-wide failure.

---

# 9. Provider Availability

Provider availability remains separate from Baobab availability.

```text id="bhn7xs"
Baobab Payments Healthy
       +
Provider A Unavailable
       +
Provider B Eligible
       =
Potentially serviceable transaction
```

subject to PAY-0009.

---

# 10. Availability Does Not Override Eligibility

Disaster or outage conditions SHALL NOT justify routing through an otherwise ineligible provider.

---

# 11. Financial Safety Over Availability

Where Baobab cannot establish safe financial execution:

```text id="pj5bxm"
STOP NEW MONEY MOVEMENT
```

is preferable to uncertain execution.

---

# 12. High Availability

Production Payments SHOULD avoid unnecessary single points of failure across:

```text id="d7aqqv"
application instances
load balancing
database access
event publication
webhook ingestion
recovery workers
observability
secret retrieval
```

according to deployment maturity.

---

# 13. Stateless Application Tier

Payments API instances SHOULD remain horizontally replaceable wherever practical.

Canonical financial state SHALL NOT depend upon local process memory.

---

# 14. Local Memory

Local caches SHALL never be the only copy of:

```text id="r51r9n"
canonical payment state
idempotency claims
approval state
outbox intent
webhook deduplication state
financial mappings
```

---

# 15. PostgreSQL Authority

Baobab Payments SHALL persist its canonical operational state in its own PostgreSQL database.

This database remains separate from HyperSwitch's database.

---

# 16. Database Separation

```text id="xys9yn"
Baobab Payments PostgreSQL
            ≠
HyperSwitch PostgreSQL
```

Neither database SHALL be treated as a substitute backup for the other.

---

# 17. No Cross-Database Recovery Assumption

Restoring Baobab Payments does not restore HyperSwitch.

Restoring HyperSwitch does not restore Baobab Payments.

Recovery procedures SHALL account for both systems.

---

# 18. PostgreSQL Production Configuration

Production PostgreSQL SHALL use:

```text id="c13o2n"
supported versions
tested versions
pinned versions
documented backup configuration
documented recovery procedures
```

rather than unpinned `latest` images.

---

# 19. PostgreSQL 17

Where PostgreSQL 17 is the approved Baobab Payments persistence baseline, deployment, backup and recovery procedures SHALL be tested against the exact supported PostgreSQL 17 minor release used in production.

---

# 20. Backup Is Not Availability

Backups protect recoverability.

They do not replace high availability.

---

# 21. Replication Is Not Backup

Database replication SHALL NOT be treated as a backup.

Corruption, operator error or malicious deletion may replicate.

---

# 22. Backup Strategy

Payments SHALL maintain a documented backup strategy capable of restoring required financial state.

This SHOULD include appropriate combinations of:

```text id="k88bq2"
base backups
continuous WAL archiving
point-in-time recovery
configuration backup
secret-reference metadata
infrastructure configuration
```

according to deployment architecture.

---

# 23. Point-in-Time Recovery

Point-in-time recovery SHOULD be supported for canonical Payments persistence.

---

# 24. Recovery Point Objective

Payments SHALL define an explicit RPO.

RPO answers:

> How much committed Baobab state can the platform tolerate losing during a disaster?

---

# 25. Recovery Time Objective

Payments SHALL define an explicit RTO.

RTO answers:

> How long may restoration of the affected Payments capability take?

---

# 26. RPO and RTO Are Different

```text id="67xdfo"
RPO
=
acceptable state-loss window

RTO
=
acceptable restoration-time window
```

---

# 27. RPO Does Not Mean Financial Loss Is Acceptable

An infrastructure RPO does not mean externally executed financial transactions within that interval may simply be forgotten.

External financial facts SHALL be recovered through reconciliation.

---

# 28. Recovery Classes

Baobab SHOULD classify recovery objectives by capability.

Conceptually:

| Capability | Recovery Criticality |
|---|---|
| Webhook durable ingress | Critical |
| Canonical payment state | Critical |
| Idempotency state | Critical |
| Payout state | Critical |
| Refund state | Critical |
| Outbox | Critical |
| Provider mappings | Critical |
| Audit | Critical |
| Settlement ingestion | High |
| Reconciliation | High |
| Administrative reporting | Lower |
| Analytics | Lower |

Exact RPO/RTO values SHALL be defined operationally.

---

# 29. No Invented Universal RPO

This ADR SHALL NOT invent one arbitrary RPO/RTO for all environments.

Targets SHALL reflect:

```text id="lyh0wq"
financial risk
business requirements
provider contracts
infrastructure capability
regulatory requirements
customer commitments
cost
```

---

# 30. Recovery Objectives Are Tested

An RPO/RTO documented but never tested SHALL not be considered demonstrated.

---

# 31. Backup Scope

A recoverable Payments deployment requires more than primary payment rows.

Critical backup/recovery scope includes:

```text id="6ixpqb"
PaymentIntent
Payment
PaymentAttempt
Refund
Dispute
Settlement
Payout
idempotency records
webhook inbox
event outbox
canonical mapping references
approval state
reconciliation state
audit records
schema version
```

where these are stored by Payments.

---

# 32. Idempotency State Is Financial State

PAY-0010 idempotency records SHALL be treated as critical recovery data.

---

# 33. Why Idempotency Recovery Matters

If Payment state is restored but its idempotency state is lost:

```text id="qj7slx"
original request
      │
      ▼
external execution
      │
      ▼
disaster
      │
      ▼
Payment restored
idempotency record missing
      │
      ▼
caller retries
      │
      ▼
duplicate risk
```

Therefore idempotency recovery is mandatory.

---

# 34. Outbox State Is Financially Significant

PAY-0011 outbox state SHALL be recovered consistently with canonical state.

---

# 35. Atomic State/Outbox Recovery

Because canonical mutation and outbox intent are committed atomically, recovery SHALL preserve that relationship.

---

# 36. Webhook Inbox State

Webhook inbox and deduplication state SHALL be recoverable sufficiently to avoid treating previously processed provider observations as novel financial facts.

---

# 37. Event Identity

Canonical event identity SHALL survive recovery.

---

# 38. Event Replay

Recovery MAY republish committed canonical events.

It SHALL retain stable event identity and SHALL NOT recreate external financial execution.

---

# 39. Approval State

PAY-0018 approval state SHALL be recovered where a pending financial operation depends upon it.

---

# 40. Approval Revalidation

A restored approval SHALL still be revalidated for:

```text id="48y72v"
expiry
identity revocation
capability status
beneficiary status
provider status
material request changes
```

before execution.

---

# 41. Provider Mapping Recovery

PAY-0003/PAY-0004 mappings required to interpret historical provider resources SHALL remain recoverable.

---

# 42. Historical Mapping Preservation

A mapping retired for new execution may still be required for:

```text id="glkt1a"
Refund
Dispute
Settlement
Reconciliation
Audit
```

and SHALL not be discarded during recovery.

---

# 43. Provider Activation State

PAY-0022 certification and activation state SHALL be restored or authoritatively re-established before unrestricted routing resumes.

---

# 44. Routing Configuration

PAY-0009 routing policy SHALL be validated against restored canonical eligibility.

---

# 45. HyperSwitch Configuration

Restored HyperSwitch routing/merchant/profile/connector configuration SHALL be checked for drift against Baobab canonical mappings and activation state.

---

# 46. HyperSwitch Is a Dependency

HyperSwitch availability SHALL be independently assessed during recovery.

---

# 47. HyperSwitch Recovery

Recovery planning SHALL consider HyperSwitch components used by the deployed version, potentially including:

```text id="g5m1n6"
Router
Scheduler Producer
Scheduler Consumer
PostgreSQL
Redis
Superposition
Drainer
optional analytical/event infrastructure
```

according to actual deployment.

---

# 48. Version-Specific Dependency Inventory

The exact HyperSwitch recovery dependency set SHALL be maintained from the deployed HyperSwitch version rather than assumed permanently from this ADR.

---

# 49. HyperSwitch Database

HyperSwitch database backup and recovery SHALL follow HyperSwitch-supported operational practices.

Baobab SHALL NOT directly manipulate HyperSwitch tables as part of normal recovery.

---

# 50. Redis

If Redis is used by the deployed HyperSwitch architecture, recovery SHALL account for the consequences of Redis loss without assuming Redis is the canonical Baobab payment database.

---

# 51. Superposition

Where the deployed HyperSwitch version requires Superposition, its availability/configuration SHALL be included in dependency recovery.

---

# 52. HyperSwitch Scheduler

Pending/retry/scheduled operations in HyperSwitch SHALL be considered during disaster recovery to avoid duplicate or abandoned external execution.

---

# 53. HyperSwitch Restore ≠ Baobab Resume

Even when HyperSwitch is healthy:

```text id="39pyof"
HyperSwitch Healthy
      ≠
Baobab Safe To Resume
```

Baobab canonical recovery gates remain mandatory.

---

# 54. Provider State Survives Baobab Failure

A provider may continue to possess financial truth even when Baobab infrastructure is unavailable.

---

# 55. External Reality Cannot Be Rolled Back

```text id="tl8bq3"
Database PITR
    ≠
Provider PITR
```

Baobab can restore its database to an earlier point.

It cannot roll a bank or PSP back to that same point.

---

# 56. Fundamental Recovery Asymmetry

This creates the central payment recovery problem:

```text id="4ex9uh"
Internal state can move backwards during restore.

External financial reality does not.
```

Therefore reconciliation is mandatory after material restore.

---

# 57. Post-Restore Reconciliation

After restoration from a backup/PITR point that may precede external execution, Payments SHALL identify potentially affected transactions.

---

# 58. Recovery Window

The reconciliation window SHOULD begin sufficiently before the restore point to account for:

```text id="l8hl18"
clock skew
in-flight operations
provider delays
webhook delays
transaction commit boundaries
```

---

# 59. Reconciliation Sources

Recovery MAY use:

```text id="b7fb71"
HyperSwitch query APIs
provider query APIs
provider reports
provider webhooks
settlement reports
bank/rail evidence
canonical event history
ERP projections
```

according to authoritative availability.

---

# 60. Evidence Hierarchy

Recovery SHALL use the strongest available authoritative financial evidence.

No single diagnostic log line SHALL establish money movement.

---

# 61. Restored Missing Transaction

If provider evidence establishes a financial transaction that the restored database lacks:

```text id="b0s46z"
DO NOT
simply execute again.
```

The transaction SHALL enter controlled reconciliation/recovery.

---

# 62. Restored Stale State

If restored canonical state says:

```text id="6t9b60"
PROCESSING
```

but authoritative provider evidence says:

```text id="dx1isq"
CAPTURED
```

Payments SHALL converge through governed recovery/state-transition mechanisms.

---

# 63. No Manual History Fabrication

Recovery SHALL NOT use arbitrary SQL edits to force canonical state into apparent agreement.

---

# 64. Unknown State After Disaster

Any financial operation whose external result cannot be established SHALL enter or remain:

```text id="v3hp5p"
UNKNOWN
```

or equivalent controlled uncertainty state.

---

# 65. UNKNOWN Is Safer Than Guessing

```text id="s3n7kl"
UNKNOWN
    >
incorrect SUCCESS

UNKNOWN
    >
incorrect FAILURE
```

from a financial-safety perspective.

---

# 66. No Blind Replay

Requests from the disaster window SHALL NOT be blindly replayed.

---

# 67. No Blind Provider Failover

Uncertain pre-disaster provider attempts SHALL NOT be routed to a replacement provider until PAY-0009/PAY-0010 establish that another attempt is safe.

---

# 68. Payout Recovery

Payout recovery receives heightened safeguards.

An uncertain Payout SHALL not be re-executed merely because restored canonical state appears incomplete.

---

# 69. Refund Recovery

Likewise, an uncertain Refund SHALL not be duplicated.

---

# 70. Capture Recovery

A capture request whose external result is uncertain SHALL be reconciled before a new capture is attempted.

---

# 71. Void Recovery

Void/capture races spanning a disaster SHALL be reconciled against provider evidence before further financial mutation.

---

# 72. Dispute Recovery

Dispute deadlines and submitted evidence SHALL be restored/re-established promptly.

A disaster SHALL not silently erase evidence deadlines.

---

# 73. Settlement Recovery

Settlement files/reports received around a disaster window SHALL be tracked for:

```text id="gzvr1s"
received
persisted
processed
reconciled
```

to prevent omission or duplicate processing.

---

# 74. Reconciliation Recovery

PAY-0015 reconciliation checkpoints SHALL be recoverable.

---

# 75. Reconciliation Rerun

Reconciliation SHOULD be safe to rerun idempotently after recovery.

---

# 76. ERP Projection Recovery

Canonical payment facts may need to be replayed/reprojected to ERP.

This SHALL use stable event identity/idempotent ERP consumption.

---

# 77. ERP Is Not Payments Backup

ERP accounting records SHALL not be treated as a complete substitute for Payments operational state.

---

# 78. Payments Is Not ERP Backup

Likewise, Payments operational state is not the accounting ledger backup.

---

# 79. Multi-System Recovery

A severe incident may require coordinated recovery across:

```text id="op7qme"
Payments
HyperSwitch
Control Plane
IAM
Trade
Subscriptions
ERP
event infrastructure
secrets infrastructure
```

without merging their authority boundaries.

---

# 80. Recovery Order

A conceptual recovery dependency sequence is:

```text id="37w23n"
Infrastructure
     │
     ▼
Secrets / Key Access
     │
     ▼
IAM
     │
     ▼
Control Plane Context
     │
     ▼
Payments Persistence
     │
     ▼
HyperSwitch Dependencies
     │
     ▼
Webhook / Event Infrastructure
     │
     ▼
Provider Connectivity
     │
     ▼
Reconciliation
     │
     ▼
Financial Execution
```

Exact deployment order may vary.

---

# 81. Recovery Order Is Not Authority Transfer

Starting one component before another does not change canonical authority.

---

# 82. Webhook Ingestion During Degradation

Where safely possible, provider webhook ingestion SHOULD remain available even when new payment execution is suspended.

---

# 83. Why Webhooks Matter During Outage

Provider callbacks may contain evidence needed to resolve transactions already in flight.

---

# 84. Durable Webhook Buffering

Webhook ingress SHOULD durably preserve verified provider observations during downstream processing outages.

---

# 85. Do Not Drop Financial Observations

Load shedding SHALL prioritise preservation of critical financial observations.

---

# 86. Event Infrastructure Outage

If the event broker is unavailable:

```text id="s6yzn8"
canonical state commit
+
transactional outbox
```

SHOULD preserve publication intent.

---

# 87. Event Broker Recovery

When the broker returns, outbox publication SHALL resume idempotently.

---

# 88. Consumer Outage

A downstream consumer outage SHALL not force Payments to roll back already committed canonical financial truth.

---

# 89. ERP Outage

Payments MAY continue safe operations during an ERP outage where business policy permits, provided:

```text id="vdb0ct"
canonical facts remain durable
events remain recoverable
ERP projection can catch up
reconciliation remains possible
```

---

# 90. Control Plane Outage

Payments MAY use governed cached context only where architecture explicitly permits.

Unknown or ambiguous authority fails closed.

---

# 91. IAM Outage

New privileged mutations SHALL fail closed if authentication/authorization validity cannot be established safely.

---

# 92. Secret Manager Outage

If provider credentials cannot be safely retrieved, affected provider execution SHALL become unavailable rather than use insecure fallback credentials.

---

# 93. Observability Outage

Loss of observability does not automatically mean loss of canonical state.

However, inability to monitor high-risk execution MAY justify controlled degradation depending on severity.

---

# 94. Audit Outage

Privileged financial mutations that require durable audit SHALL not proceed if their required audit evidence cannot be preserved.

---

# 95. Region Failure

Baobab MAY support regional failover.

Regional failover SHALL preserve:

```text id="x43pgr"
Tenant isolation
Legal Entity isolation
canonical identity
idempotency
authorization
provider eligibility
financial-state coherence
```

---

# 96. Multi-Region Is Not Automatically Active-Active

The presence of multiple regions SHALL NOT imply that unrestricted financial writes may occur concurrently in every region.

---

# 97. Single-Writer Preference

For a financial aggregate, Baobab SHOULD prefer a clear authoritative write path unless a rigorously tested distributed consistency model supports safe concurrent writes.

---

# 98. Split-Brain

Split-brain financial execution is prohibited.

---

# 99. Split-Brain Rule

If two regions cannot establish which has authoritative financial-write ownership:

```text id="ahv22h"
STOP NEW FINANCIAL WRITES
```

for affected aggregates/context.

---

# 100. Availability Does Not Justify Double Writers

Reduced availability is preferable to duplicate money movement.

---

# 101. Regional Idempotency

Idempotency semantics SHALL survive regional failover.

---

# 102. Regional Failover and Keys

A retry reaching the failover region SHALL still resolve the original logical idempotency claim where required.

---

# 103. Regional Event Identity

Canonical event identity SHALL remain stable across failover.

---

# 104. Regional Mapping

The failover region SHALL use valid canonical engine-instance/provider mappings.

---

# 105. Regional Provider Eligibility

A provider configured in Region B is not automatically eligible merely because Region A failed.

---

# 106. Data Residency

Regional recovery SHALL respect applicable data-residency and privacy constraints.

---

# 107. Market ≠ Recovery Region

Canonical Market SHALL not be inferred from the region performing disaster recovery.

---

# 108. Recovery Region ≠ Legal Entity

Failover infrastructure does not redefine the business principal.

---

# 109. Cold Standby

A cold/warm standby SHALL not be declared production-ready unless its:

```text id="xou06q"
data
configuration
secrets
IAM
mappings
network
provider connectivity
observability
```

are recoverable/tested.

---

# 110. Hot Standby

A hot standby SHALL still respect financial-write ownership.

---

# 111. Database Failover

Database failover SHALL account for:

```text id="02khbm"
replication lag
committed transaction visibility
idempotency records
outbox records
webhook records
```

before writes resume.

---

# 112. Lost-Commit Ambiguity

After database failover, operations around the failover boundary MAY require reconciliation if commit visibility is uncertain.

---

# 113. Backup Encryption

Backups SHALL be encrypted according to infrastructure/security policy.

---

# 114. Backup Access

Backup access SHALL be least privilege.

---

# 115. Backup Is Sensitive

A database backup SHALL be treated with security classification appropriate to the underlying production data.

---

# 116. Backup Credentials

Backup credentials SHALL not be embedded in source repositories or container images.

---

# 117. Backup Integrity

Backups SHOULD be integrity-checked.

---

# 118. Restore Testing

Baobab SHALL periodically restore backups into controlled environments to demonstrate recoverability.

---

# 119. Backup Success ≠ Restore Success

```text id="e9ulw8"
Backup Job Green
     ≠
Recoverable System
```

---

# 120. Restore Verification

Restore tests SHOULD verify:

```text id="3cnvhk"
schema integrity
canonical data
idempotency data
outbox
webhook inbox
mappings
audit
reconciliation state
application compatibility
```

---

# 121. Backup Retention

Retention SHALL reflect:

```text id="ymk0k5"
RPO/RTO
financial/audit requirements
privacy
security
regulatory requirements
cost
```

---

# 122. Backup Deletion

Expired backups SHALL be securely disposed of according to policy.

---

# 123. Backup Residency

Backup storage location SHALL comply with applicable residency requirements.

---

# 124. Tokenised Data in Backups

PAY-0012 remains binding.

Tokenised data may still be sensitive and SHALL be protected appropriately.

---

# 125. No CVV Recovery

Recovery procedures SHALL NOT attempt to restore prohibited CVV/CVC/CID data because such data SHALL not have been retained.

---

# 126. Secret Backup

Provider secrets SHOULD normally be recovered through the approved secrets-management system rather than copied into Payments database backups.

---

# 127. Key Recovery

Encryption-key recovery SHALL be planned separately from encrypted-data recovery.

---

# 128. Unrecoverable Key

An encrypted backup without the required recoverable key material is not operationally recoverable.

---

# 129. Key Compromise

A security incident involving encryption or provider keys MAY require recovery with:

```text id="gm4ln9"
rotated keys
revoked credentials
provider recertification/revalidation
```

rather than simply restoring old secrets.

---

# 130. Configuration Backup

Recovery SHALL include version-controlled or otherwise durable recovery of non-secret configuration such as:

```text id="40lb5i"
deployment manifests
routing policy
feature/capability configuration
schema definitions
observability configuration
```

where applicable.

---

# 131. Infrastructure as Code

Production infrastructure SHOULD be reproducible from governed infrastructure-as-code where practical.

---

# 132. Container Images

Production recovery SHALL use immutable, identified, trusted images.

---

# 133. Image Tags

Recovery SHALL not depend on mutable `latest` image tags.

---

# 134. Software Bill of Materials

Where supported by Baobab infrastructure standards, production images SHOULD retain provenance/SBOM information useful during disaster/security recovery.

---

# 135. Deployment Artifact Availability

Critical deployment artifacts SHALL remain available even if the primary CI system is temporarily unavailable.

---

# 136. GitHub Is Not Runtime DR

Source control availability alone is not a disaster-recovery strategy for financial state.

---

# 137. CI/CD Outage

An outage of GitHub Actions or equivalent CI/CD SHALL not cause running Payments infrastructure to lose financial state.

---

# 138. Emergency Deployment

Emergency deployment mechanisms SHALL preserve:

```text id="2bcx76"
artifact provenance
authorization
audit
configuration integrity
```

and SHALL not introduce untracked binaries.

---

# 139. Recovery Modes

Payments SHOULD support explicit operational modes such as:

```text id="6tl98x"
NORMAL
READ_ONLY
RECONCILIATION_ONLY
NO_NEW_PAYMENTS
NO_NEW_PAYOUTS
PROVIDER_SUSPENDED
RECOVERY
```

or equivalent capability-level controls.

---

# 140. Recovery Mode

`RECOVERY` mode SHOULD prevent unrestricted money movement while allowing authorised recovery operations.

---

# 141. Read-Only Mode

Read-only mode MAY support investigation without permitting financial mutation.

---

# 142. Reconciliation-Only Mode

Reconciliation-only mode MAY permit:

```text id="rm97hb"
provider queries
settlement ingestion
webhook processing
canonical recovery
```

while blocking new customer-initiated money movement.

---

# 143. Recovery Mode Is Controlled

Changing operational mode is a privileged PAY-0018 mutation.

---

# 144. Recovery Mode Audit

Every mode transition SHALL be auditable.

---

# 145. Automatic Mode Transition

Automated transition to a safer degraded mode MAY occur for predefined severe conditions.

---

# 146. Automatic Reopening

Automatic restoration of unrestricted financial execution SHOULD be more conservative than automatic suspension.

---

# 147. Human Verification

Severe incidents SHOULD require explicit verification before normal real-money traffic resumes.

---

# 148. Recovery Gate

Before new financial execution resumes after a material disaster, Baobab SHALL verify at least:

```text id="0p4h9f"
Payments database restored
schema compatible
canonical state readable
idempotency state present
webhook inbox coherent
outbox coherent
event identity preserved
audit available
IAM functional
Control Plane context resolvable
capability bindings valid
merchant/profile mappings valid
provider certification valid
provider activation valid
routing policy valid
HyperSwitch healthy
provider credentials valid
provider connectivity validated
UNKNOWN operations identified
disaster-window transactions reconciled
settlement ingestion understood
ERP projection path available or safely recoverable
observability functional
security controls functional
```

---

# 149. Resume Decision

The resume decision SHALL be based on demonstrated safety, not merely elapsed outage duration.

---

# 150. Progressive Resumption

Where practical:

```text id="oc16kb"
Read
   │
   ▼
Webhook / Recovery
   │
   ▼
Reconciliation
   │
   ▼
Low-risk Financial Capability
   │
   ▼
Controlled Provider Traffic
   │
   ▼
Normal Operation
```

is preferred over instant full reopening.

---

# 151. Capability-Specific Resumption

Refunds may sometimes resume before new collections, or collections before Payouts, depending on verified safety.

---

# 152. Payouts Last

After severe state uncertainty, outbound Payout execution SHOULD receive especially conservative reopening treatment.

---

# 153. Provider-by-Provider Recovery

Providers MAY be restored independently.

---

# 154. Market-by-Market Recovery

Markets MAY be restored independently where canonical context and provider eligibility permit.

---

# 155. Legal-Entity Recovery

Legal Entities MAY be restored independently.

A ZuriBeans recovery issue need not automatically stop Thamani if isolation is demonstrated, and vice versa.

---

# 156. Tenant Blast Radius

External tenant incidents SHOULD be contained to the affected tenant where technically and financially safe.

---

# 157. No Assumed Isolation

Containment scope SHALL be based on demonstrated architecture, not optimism.

---

# 158. Recovery Reconciliation Window

Before resuming, Payments SHOULD reconcile the period:

```text id="8o1m4h"
last known trustworthy internal state
        │
        ▼
failure/disaster window
        │
        ▼
restore point
        │
        ▼
current external provider state
```

---

# 159. Financial Exposure Register

Material recovery SHOULD maintain an explicit list of unresolved financial operations.

---

# 160. Unresolved Operations

Each unresolved operation SHOULD record:

```text id="m5s2y1"
canonical ID
operation type
Tenant
Legal Entity
amount/currency
provider
last known canonical state
last known external state
uncertainty reason
recovery owner
```

without prohibited sensitive data.

---

# 161. No Hidden UNKNOWN Backlog

Unresolved financial uncertainty SHALL remain visible until resolved.

---

# 162. Recovery Completion

A disaster SHALL not be considered financially closed while material unresolved operations remain unowned.

---

# 163. Recovery Audit

Recovery actions SHALL be auditable.

---

# 164. Recovery Correlation

Recovery operations SHOULD preserve:

```text id="d05jfa"
incident_id
recovery_run_id
correlation_id
resource_id
operator/workload
```

where applicable.

---

# 165. Recovery Events

Material recovery may produce canonical correction/reconciliation events.

These SHALL represent facts, not rewrite prior event history.

---

# 166. Recovery Replay

Replaying canonical events SHALL not trigger new provider money movement.

---

# 167. Restore Environment

Backup restore testing SHALL use appropriately isolated environments.

---

# 168. Production Data in DR Tests

Production-derived data used in DR testing SHALL comply with privacy/security controls.

---

# 169. No Unsafe Sandbox Substitution

Sandbox provider success does not prove production recovery.

---

# 170. Production Connectivity Validation

Production provider connectivity SHOULD be validated using non-destructive/provider-supported mechanisms before reopening.

---

# 171. Disaster Scenarios

Recovery design SHALL consider at least:

```text id="oz4kr5"
single process failure
application-node loss
database primary failure
database corruption
accidental deletion
region outage
network partition
event-broker outage
webhook-processing outage
HyperSwitch outage
HyperSwitch database failure
provider outage
provider credential compromise
secret-manager outage
IAM outage
Control Plane outage
ERP outage
ransomware/security incident
bad deployment
schema migration failure
```

---

# 172. Bad Deployment

Application rollback SHALL not assume financial operations executed under the bad version can be rolled back.

---

# 173. Schema Failure

Database migration failures SHALL have tested forward/repair/restore procedures appropriate to the migration.

---

# 174. Backward Compatibility

Deployments SHOULD preserve sufficient contract/schema compatibility for safe rolling deployment and recovery.

---

# 175. Ransomware / Destructive Attack

Recovery from destructive attack SHALL include security containment before restoring financial execution.

---

# 176. Clean Recovery

Compromised credentials or binaries SHALL not simply be restored from backup without security validation.

---

# 177. Provider Credential Compromise

Affected provider credentials SHALL be revoked/rotated.

Historical transaction references SHALL remain available for reconciliation.

---

# 178. Disaster Versus Security Incident

A disaster may simultaneously be:

```text id="68bsf1"
availability incident
security incident
financial incident
```

and SHALL be managed accordingly.

---

# 179. Observability During DR

PAY-0019 observability SHALL remain available as early as practical during recovery.

---

# 180. Recovery Without Observability

Unrestricted real-money execution SHOULD NOT resume when critical safety monitoring remains unavailable.

---

# 181. DR Dashboard

Operations SHOULD have a recovery dashboard showing:

```text id="vmemh2"
database state
replication state
backup age
restore point
service state
HyperSwitch state
provider state
webhook backlog
outbox backlog
UNKNOWN count/age
reconciliation status
ERP projection lag
```

---

# 182. Backup Monitoring

Monitor:

```text id="ab5hwj"
backup success
backup age
WAL/archive continuity
backup size anomalies
integrity-check failures
restore-test status
```

---

# 183. RPO Monitoring

Where technically feasible, operations SHOULD be able to estimate current recoverability relative to RPO.

---

# 184. DR Alerting

Alert on:

```text id="4l3uqg"
backup failure
WAL archival failure
replication lag
restore-test failure
backup retention failure
recovery dependency failure
```

---

# 185. Recovery Exercises

Baobab SHALL conduct periodic recovery exercises.

---

# 186. Exercise Types

Exercises SHOULD include:

```text id="rbv0p7"
database restore
PITR
application rebuild
provider outage
HyperSwitch outage
regional failover
webhook backlog recovery
outbox recovery
event replay
ERP projection rebuild
UNKNOWN reconciliation
```

---

# 187. Financial Safety Exercise

At least some exercises SHALL explicitly test:

> Can Baobab avoid duplicate financial execution after losing recent internal state?

---

# 188. Restore Drill

A restore drill SHALL prove more than database startup.

It SHOULD demonstrate:

```text id="qfxad0"
application can read state
idempotency works
events can resume
webhooks deduplicate
mappings resolve
reconciliation works
financial execution gate works
```

---

# 189. Regional Failover Drill

Regional failover testing SHALL verify no split-brain writes.

---

# 190. Provider Recovery Drill

Provider recovery testing SHALL verify that failover/reopening obeys PAY-0009.

---

# 191. Payout Recovery Drill

Payout disaster exercises SHOULD verify that an uncertain outbound Payout is not duplicated.

---

# 192. Exercise Evidence

Recovery exercises SHOULD retain:

```text id="rkj5ov"
date
scope
scenario
RTO achieved
RPO achieved
failures
financial-safety findings
corrective actions
owners
```

---

# 193. Failed DR Exercise

A failed recovery exercise is a production-readiness finding, not merely a documentation issue.

---

# 194. Recovery Documentation

Runbooks SHALL exist for major recovery scenarios.

---

# 195. Minimum Recovery Runbooks

At minimum:

```text id="wb3j9s"
Payments PostgreSQL restore
point-in-time recovery
database failover
Payments service rebuild
HyperSwitch recovery
provider outage
provider credential rotation
event broker recovery
webhook backlog recovery
outbox recovery
regional failover
ERP projection recovery
settlement/reconciliation recovery
```

---

# 196. Runbook Safety

Every financial recovery runbook SHOULD identify:

```text id="u66jmf"
what may safely be retried
what must not be retried
how UNKNOWN is handled
how provider state is checked
how execution is suspended
how normal execution is resumed
```

---

# 197. Ownership

Recovery responsibilities SHALL have identified owners.

---

# 198. Recovery Roles

A major disaster MAY involve:

```text id="8k86xd"
Incident Commander
Payments Engineering
Platform / Infrastructure
Database Operations
Security
IAM
Finance / ERP
Payments Operations
Provider Liaison
Business Owner
```

---

# 199. Financial Authority During DR

Incident leadership does not automatically gain financial mutation authority.

PAY-0018 remains binding.

---

# 200. Break-Glass

Break-glass access MAY be used where necessary but remains:

```text id="jdhcf6"
time-bound
audited
least privilege
reviewed
```

---

# 201. Recovery Approval

Resumption of particularly high-risk capabilities such as Payouts MAY require explicit operational approval.

---

# 202. Recovery Decision Record

Material disaster recovery SHOULD record:

```text id="olcr4x"
incident
restore point
data-loss assessment
reconciliation scope
remaining uncertainty
resume decision
approver
time
```

---

# 203. Production Readiness Gate

```text id="kcf2iy"
AVAILABILITY / BACKUP / DR READINESS

High-availability design                    PASS
Capability-aware degradation                PASS
PostgreSQL HA                               PASS
Pinned supported PostgreSQL version         PASS
Automated backups                           PASS
WAL archiving / PITR                        PASS
Backup encryption                           PASS
Backup access control                       PASS
Backup integrity verification               PASS
Restore testing                             PASS
RPO defined                                 PASS
RTO defined                                 PASS
RPO/RTO tested                              PASS
Idempotency-state recovery                  PASS
Webhook inbox recovery                      PASS
Outbox recovery                             PASS
Event identity recovery                     PASS
Audit recovery                              PASS
Approval-state recovery                     PASS
Mapping recovery                            PASS
Provider activation recovery                PASS
HyperSwitch recovery plan                   PASS
Provider reconciliation                     PASS
Settlement recovery                         PASS
ERP reprojection                            PASS
UNKNOWN recovery                            PASS
Refund recovery                             PASS
Payout recovery                             PASS
No-blind-replay control                     PASS
Regional failover                           PASS
Split-brain prevention                      PASS
Regional idempotency                        PASS
Data-residency compliance                   PASS
Secret/key recovery                         PASS
Immutable deployment artifacts              PASS
Infrastructure reproducibility              PASS
Recovery operational modes                  PASS
Financial execution recovery gate           PASS
Progressive resumption                      PASS
DR dashboards                               PASS
Backup/DR alerting                          PASS
Recovery runbooks                           PASS
Periodic restore drills                     PASS
Regional failover drills                    PASS
Financial-safety drills                     PASS
Recovery audit                              PASS
------------------------------------------------
Availability & Disaster Resilience          READY
```

---

# 204. Required Test Matrix

Implementation SHALL eventually test at least:

```text id="npbg96"
single Payments instance loss
all application instances restarted
database primary loss
replica promotion
database PITR
backup corruption detection
restore from backup
lost recent internal state
idempotency record recovery
outbox recovery
webhook dedup after restore
duplicate client retry after restore
provider captured / local state lost
provider payout succeeded / local state lost
Refund succeeded / local state lost
UNKNOWN capture recovery
UNKNOWN Payout recovery
HyperSwitch outage
HyperSwitch database recovery
provider outage
provider credential rotation
secret-manager outage
event-broker outage
ERP outage
Control Plane outage
IAM outage
region outage
network partition
split-brain prevention
regional failover
regional retry using same idempotency identity
routing drift after restore
provider activation drift
settlement file replay
reconciliation rerun
event replay without financial re-execution
bad deployment rollback
failed schema migration
security compromise recovery
recovery mode enforcement
progressive reopening
Payout reopening approval
```

---

# 205. Invariants

The following invariants SHALL hold:

1. Infrastructure recovery is not financial recovery.
2. A running process does not prove safe payment execution.
3. Service availability is not provider availability.
4. Provider availability is not provider eligibility.
5. Availability does not override payment eligibility.
6. Financial safety takes precedence over availability.
7. Canonical financial state does not depend on process memory.
8. Local caches are not authoritative recovery stores.
9. Payments PostgreSQL and HyperSwitch PostgreSQL remain separate authorities.
10. One database is not the backup of the other.
11. Replication is not backup.
12. Backup is not high availability.
13. Backup success is not restore success.
14. RPO and RTO are distinct.
15. RPO does not permit forgetting external financial reality.
16. Recovery objectives are tested.
17. Idempotency state is critical financial recovery state.
18. Losing idempotency state can create duplicate-execution risk.
19. Outbox intent is recovered with canonical state.
20. Webhook deduplication state is recoverable.
21. Event identity survives recovery.
22. Event replay does not recreate financial execution.
23. Approval state is recoverable.
24. Restored approval is revalidated before execution.
25. Historical mappings remain available for lifecycle servicing.
26. Provider certification/activation is validated after recovery.
27. Routing configuration is validated after recovery.
28. HyperSwitch health does not establish Baobab readiness.
29. Internal state may be restored backwards.
30. External financial reality cannot be rolled back.
31. Material restore requires reconciliation.
32. Disaster-window transactions are explicitly identified.
33. Provider-executed transactions missing locally are not blindly retried.
34. Stale restored state converges through governed recovery.
35. Recovery does not rely on arbitrary SQL history edits.
36. Unknown outcomes remain UNKNOWN until evidence establishes a fact.
37. UNKNOWN is preferable to guessed financial success/failure.
38. Disaster-window requests are not blindly replayed.
39. Uncertain attempts do not blindly fail over.
40. Uncertain Payouts are not re-executed.
41. Uncertain Refunds are not duplicated.
42. Uncertain captures are reconciled before retry.
43. Dispute deadlines survive disaster recovery.
44. Settlement ingestion is idempotently recoverable.
45. Reconciliation is safely rerunnable.
46. ERP projection can be rebuilt idempotently.
47. ERP is not Payments backup.
48. Payments is not ERP backup.
49. Multi-system recovery preserves authority boundaries.
50. Webhook observations are preserved during degradation where possible.
51. Financial observations are prioritised during load shedding.
52. Broker outage does not erase committed event intent.
53. Consumer outage does not roll back payment truth.
54. IAM uncertainty fails closed for privileged mutation.
55. Control Plane uncertainty does not justify invented context.
56. Secret-manager outage does not justify insecure fallback credentials.
57. Required audit unavailability can block privileged mutation.
58. Regional failover preserves Tenant isolation.
59. Regional failover preserves Legal Entity isolation.
60. Multi-region does not automatically mean active-active financial writes.
61. Split-brain financial writes are prohibited.
62. Regional idempotency survives failover.
63. Event identity survives regional failover.
64. Recovery-region identity does not redefine Market.
65. Recovery region does not redefine Legal Entity.
66. Standby readiness is tested.
67. Database failover considers replication lag.
68. Failover-boundary ambiguity triggers reconciliation.
69. Backups are encrypted.
70. Backup access is least privilege.
71. Backup integrity is verified.
72. Restores are periodically tested.
73. Backup retention is governed.
74. Backup residency is governed.
75. Tokenised backup data remains protected.
76. Prohibited CVV data is not recovered because it was never retained.
77. Secrets are recovered through approved secret management.
78. Encrypted backups require recoverable key material.
79. Compromised credentials are not blindly restored.
80. Non-secret infrastructure configuration is recoverable.
81. Production recovery uses immutable identified images.
82. Recovery does not depend on mutable `latest`.
83. Source control is not financial-state DR.
84. CI/CD outage does not destroy runtime financial state.
85. Emergency deployments preserve provenance.
86. Recovery mode blocks unrestricted money movement.
87. Recovery-mode transitions are privileged and audited.
88. Automatic suspension may be faster than automatic reopening.
89. Real-money execution requires a recovery gate.
90. Resume decisions are safety-based.
91. Progressive reopening is preferred after material uncertainty.
92. Capabilities may recover independently.
93. Payout reopening receives enhanced caution.
94. Providers may recover independently.
95. Legal Entities may recover independently when isolation is demonstrated.
96. Tenant blast radius is minimised where safe.
97. Unresolved financial operations remain visible.
98. Disaster recovery is not financially complete while material uncertainty is unowned.
99. Recovery actions are auditable.
100. Recovery event replay does not cause money movement.
101. Sandbox recovery does not prove production recovery.
102. Production provider connectivity is validated safely.
103. Bad application versions do not roll back external financial reality.
104. Security compromise requires security containment before financial reopening.
105. Critical observability is restored before unrestricted execution.
106. Recovery exercises test financial safety, not merely infrastructure.
107. Regional drills verify split-brain prevention.
108. Payout drills verify duplicate prevention.
109. Failed DR exercises are production-readiness findings.
110. Runbooks explicitly identify unsafe retries.
111. Incident leadership does not bypass financial authorization.
112. Break-glass remains controlled during disaster.
113. Availability objectives never justify duplicate or unauthorised money movement.
114. Financial execution resumes only when Baobab can establish a sufficiently coherent and safe execution state.

---

# 206. Consequences

## Positive

This decision provides:

- durable payment-state recovery;
- explicit RPO/RTO governance;
- PostgreSQL recovery discipline;
- idempotency preservation;
- webhook/outbox recovery;
- HyperSwitch-aware disaster planning;
- safe regional failover;
- financial-state reconciliation;
- controlled degraded modes;
- progressive recovery;
- reduced duplicate-payment risk;
- reduced duplicate-Payout risk;
- auditable recovery;
- testable production resilience.

## Negative

It requires substantial operational investment in:

- database HA;
- backup infrastructure;
- WAL/PITR;
- restore automation;
- provider reconciliation;
- HyperSwitch recovery procedures;
- DR environments;
- regional architecture;
- operational modes;
- recovery tooling;
- regular exercises;
- cross-team runbooks.

These costs are accepted because restoring payment infrastructure without preserving financial correctness can make recovery more dangerous than the original outage.

---

# 207. Alternatives Considered

## Restore Database and Immediately Resume Traffic

Rejected.

External financial state may have advanced beyond the restore point.

---

## Treat Replication as Backup

Rejected.

Replication can reproduce corruption or deletion.

---

## Restore Only Payment Rows

Rejected.

Idempotency, webhook, outbox, mapping and reconciliation state are also financially significant.

---

## Replay All Requests After Restore

Rejected.

This can duplicate financial operations.

---

## Retry Every UNKNOWN Transaction

Rejected.

UNKNOWN specifically means the external result is not sufficiently known.

---

## Depend Entirely on HyperSwitch State

Rejected.

HyperSwitch does not own Baobab canonical business/payment authority.

---

## Depend Entirely on ERP State

Rejected.

ERP owns accounting, not operational payment execution.

---

## Active-Active Writes Everywhere

Rejected as the default.

Distributed financial-write authority requires much stronger consistency guarantees than geographic redundancy alone provides.

---

## Full Platform Shutdown for Every Provider Failure

Rejected.

Capability/provider-specific degradation provides better availability without weakening financial safety.

---

## Automatic Full Reopening After Infrastructure Recovery

Rejected.

Infrastructure recovery does not establish financial execution safety.

---

# 208. Relationship to PAY-0008

PAY-0008 defines canonical payment state.

PAY-0020 ensures that state survives or can be safely reconstructed after disaster without inventing financial outcomes.

---

# 209. Relationship to PAY-0009

Regional/provider failover remains subject to eligibility and safe-failover rules.

Disaster conditions do not widen the routing envelope.

---

# 210. Relationship to PAY-0010

Idempotency state is explicitly classified as critical recovery state.

Disaster recovery SHALL preserve duplicate safety across restart, restore and regional failover.

---

# 211. Relationship to PAY-0011

Webhook inbox, canonical event identity and transactional outbox state are part of recovery.

Event replay does not imply financial re-execution.

---

# 212. Relationship to PAY-0012

Backups, DR environments, logs and restored data remain subject to PCI/sensitive-data controls.

Recovery never justifies reconstructing or retaining prohibited credentials.

---

# 213. Relationship to PAY-0013

Refund state and refundable-amount protection SHALL survive recovery.

An uncertain Refund is recovered, not duplicated.

---

# 214. Relationship to PAY-0014

Dispute state, evidence and deadlines SHALL remain recoverable.

---

# 215. Relationship to PAY-0015

Settlement and reconciliation are central mechanisms for establishing financial truth after recovery.

---

# 216. Relationship to PAY-0016

Payout recovery is treated as especially high risk because duplicated execution moves funds outbound.

---

# 217. Relationship to PAY-0017

FX execution/reconciliation state SHALL survive recovery without silently changing transaction or settlement currency semantics.

---

# 218. Relationship to PAY-0018

Recovery remains subject to IAM, contextual authorization, controlled mutation, maker-checker and break-glass controls.

Disaster does not suspend financial governance.

---

# 219. Relationship to PAY-0019

PAY-0019 provides the observability necessary to:

```text id="lftgcz"
detect failure
measure recovery
identify UNKNOWN exposure
observe backlog
validate provider health
measure reconciliation
prove safe resumption
```

---

# 220. Relationship to PAY-0021

PAY-0021 SHALL ensure HyperSwitch upgrade/customisation strategy preserves:

```text id="ugtnlm"
backup compatibility
restore procedures
schema compatibility
connector behaviour
scheduler recovery
routing configuration
vault/token semantics
DR testing
```

across supported upgrades.

---

# 221. Relationship to PAY-0022

Provider certification SHALL include operational and resilience evidence.

Activation SHALL consider whether the provider can be:

```text id="m1d2cv"
monitored
suspended
recovered
reconciled
historically serviced
```

within Baobab's disaster model.

---

# 222. Complete Disaster-Recovery Model

```text id="pmk36g"
                     NORMAL OPERATION
                            │
                            ▼
                         FAILURE
                            │
                            ▼
                        CONTAIN
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
         Stop Unsafe     Preserve      Preserve
          Execution      Webhooks       Evidence
              │             │             │
              └─────────────┼─────────────┘
                            ▼
                         RESTORE
                            │
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
          Payments      HyperSwitch     Platform
            State        Dependencies   Dependencies
             │              │              │
             └──────────────┼──────────────┘
                            ▼
                         VALIDATE
                            │
                 ┌──────────┼──────────┐
                 ▼          ▼          ▼
             Context    Idempotency   Mappings
                 │          │          │
                 └──────────┼──────────┘
                            ▼
                       RECONCILE
                            │
               ┌────────────┼────────────┐
               ▼            ▼            ▼
           Provider      Settlement      ERP
               │            │            │
               └────────────┼────────────┘
                            ▼
                   RESOLVE UNKNOWN
                            │
                            ▼
                    EXECUTION SAFETY
                         GATE
                            │
                   ┌────────┴────────┐
                   ▼                 ▼
                 FAIL               PASS
                   │                 │
                   ▼                 ▼
            Continue Recovery   Progressive Resume
                                     │
                                     ▼
                              Controlled Traffic
                                     │
                                     ▼
                                  VERIFY
                                     │
                                     ▼
                             NORMAL OPERATION
```

---

# 223. Financial Recovery Model

```text id="8idkg7"
                 INTERNAL BAOBAB STATE
                          │
                          │ disaster
                          ▼
                    RESTORE POINT
                          │
                          │
             ┌────────────┴────────────┐
             ▼                         ▼
     Restored Canonical State    External Provider State
             │                         │
             │                         │
             └────────────┬────────────┘
                          ▼
                     RECONCILIATION
                          │
             ┌────────────┼────────────┐
             ▼            ▼            ▼
          MATCHED       DIVERGENT     UNKNOWN
             │            │            │
             ▼            ▼            ▼
          ACCEPT      CONTROLLED     RECOVERY
                       CORRECTION      WORKFLOW
                          │            │
                          └─────┬──────┘
                                ▼
                       CANONICAL TRUTH
                                │
                                ▼
                         SAFE TO RESUME?
```

---

# 224. Recovery Safety Formula

A Payments deployment SHALL not treat:

```text id="gyrghw"
Database Restored
+
Containers Running
```

as sufficient.

Instead:

```text id="ov6xms"
Safe Financial Recovery
=
Infrastructure Restored
+
Canonical State Restored
+
Idempotency Restored
+
Mappings Validated
+
Authorization Restored
+
HyperSwitch Validated
+
Provider Connectivity Validated
+
Disaster Window Reconciled
+
UNKNOWN Exposure Controlled
+
Event Delivery Recoverable
+
Settlement State Understood
+
Observability Restored
+
Execution Safety Gate Passed
```

---

# 225. Final Decision

Baobab SHALL permanently distinguish:

```text id="clywpn"
BACKUP
    from
HIGH AVAILABILITY

REPLICATION
    from
BACKUP

BACKUP SUCCESS
    from
RESTORE SUCCESS

PROCESS RECOVERY
    from
SERVICE RECOVERY

SERVICE RECOVERY
    from
FINANCIAL RECOVERY

DATABASE RESTORE
    from
EXTERNAL FINANCIAL REALITY

RPO
    from
PERMISSION TO FORGET MONEY MOVEMENT

RTO
    from
PERMISSION TO RESUME UNSAFE EXECUTION

REGIONAL REDUNDANCY
    from
ACTIVE-ACTIVE FINANCIAL AUTHORITY

PROVIDER AVAILABILITY
    from
PROVIDER ELIGIBILITY

RESTORED PAYMENT STATE
    from
RESTORED IDEMPOTENCY SAFETY

EVENT REPLAY
    from
FINANCIAL RE-EXECUTION

UNKNOWN
    from
FAILURE

INFRASTRUCTURE HEALTH
    from
EXECUTION SAFETY

INCIDENT RECOVERY
    from
FINANCIAL RECONCILIATION

DISASTER
    from
PERMISSION TO BYPASS GOVERNANCE
```

The governing recovery sequence is:

```text id="s7j34r"
FAILURE
   │
   ▼
STOP UNSAFE EXECUTION
   │
   ▼
PRESERVE FINANCIAL OBSERVATIONS
   │
   ▼
RESTORE INFRASTRUCTURE
   │
   ▼
RESTORE CANONICAL STATE
   │
   ▼
RESTORE IDEMPOTENCY + EVENT STATE
   │
   ▼
VALIDATE IAM + CONTROL PLANE + MAPPINGS
   │
   ▼
VALIDATE HYPERSWITCH + PROVIDERS
   │
   ▼
RECONCILE EXTERNAL FINANCIAL REALITY
   │
   ▼
RESOLVE / CONTAIN UNKNOWN OPERATIONS
   │
   ▼
PASS FINANCIAL EXECUTION SAFETY GATE
   │
   ▼
PROGRESSIVELY RESUME
   │
   ▼
VERIFY FINANCIAL CONVERGENCE
```

**Backups allow Baobab to restore internal state.  
Replication improves availability.  
Idempotency state protects restored execution from duplication.  
Webhook and outbox durability preserve financial observations and event intent.  
HyperSwitch recovery restores the payment-orchestration dependency.  
Provider evidence establishes what actually happened outside Baobab.  
Settlement and reconciliation expose financial reality that application state alone cannot prove.  
IAM and the Control Plane restore authority and canonical context.  
Observability demonstrates whether recovery is progressing safely.**

**Baobab does not resume real-money execution merely because infrastructure has returned. It resumes only when it can establish that restored canonical state, idempotency, mappings, authorization, provider state, external financial evidence and reconciliation are sufficiently coherent to prevent duplicate, unauthorised or misattributed money movement. When that cannot be established, Baobab remains degraded, read-only or reconciliation-only rather than guessing with money.**