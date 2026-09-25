# ADR-PAY-0014 — Disputes, Chargebacks & Evidence

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Payment Risk / Financial Operations |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0013; ADR-PAY-0022; ADR-SHARED-011; applicable Control Plane, IAM, Trade, ERP, CMS and Shared contracts |
| **Related** | ADR-PAY-0015 through ADR-PAY-0021 |
| **Primary Domains** | Disputes, chargebacks, evidence, representment, financial exposure, deadlines, reconciliation |

---

# 1. Context

A successfully authorised, captured, or settled Payment may later be challenged.

Depending on the payment method and provider, the challenge may originate from:

- cardholder;
- issuing bank;
- acquiring bank;
- payment network;
- wallet;
- bank-transfer scheme;
- mobile-money operator;
- PSP;
- regulatory or fraud process;
- another payment-rail participant.

The provider may describe these processes using terms such as:

```text
dispute
chargeback
inquiry
retrieval request
claim
fraud claim
pre-dispute
pre-arbitration
arbitration
representment
reversal
```

These terms are not universally equivalent.

Baobab requires provider-independent canonical semantics.

---

# 2. Problem

A naïve implementation might model:

```text
Payment
   │
   ▼
CHARGEBACK
```

as merely another Payment state.

That loses critical information.

A Payment may simultaneously be:

```text
successfully captured
settled
partially refunded
under dispute
subject to financial hold
awaiting evidence
```

These facts are not mutually exclusive.

Likewise:

```text
Dispute
    ≠
Refund
```

and:

```text
Chargeback
    ≠
Payment Failure
```

---

# 3. Decision

`baobab-payments` SHALL own a canonical, provider-independent:

```text
Dispute
```

aggregate and associated dispute lifecycle.

A Dispute SHALL remain distinct from:

```text
Payment
PaymentAttempt
Refund
Settlement
Payout
Order
Subscription
Invoice
Accounting Entry
```

Provider dispute/chargeback objects SHALL be translated into canonical Baobab dispute facts.

---

# 4. Governing Principle

> **A dispute challenges or alters the financial outcome of an existing payment; it does not rewrite the historical fact that the payment occurred.**

Therefore:

```text
Payment CAPTURED
      │
      ▼
Dispute OPENED
```

does not mean:

```text
Payment FAILED
```

The original Payment history remains intact.

---

# 5. Fundamental Distinctions

Baobab SHALL preserve:

```text
PAYMENT
   =
execution of collection

REFUND
   =
merchant/business-authorised return of funds

DISPUTE
   =
externally initiated challenge to payment

CHARGEBACK
   =
financial debit/reversal arising through dispute/rail process

REPRESENTMENT
   =
merchant response contesting eligible dispute

EVIDENCE
   =
information submitted in support of a dispute response

SETTLEMENT ADJUSTMENT
   =
financial settlement consequence

ACCOUNTING CONSEQUENCE
   =
ERP interpretation/posting
```

---

# 6. Explicit Non-Equivalences

The following SHALL NOT be collapsed:

```text
Dispute
   ≠
Payment

Dispute
   ≠
Refund

Chargeback
   ≠
Refund

Chargeback
   ≠
Payment Failure

Dispute Opened
   ≠
Chargeback Lost

Evidence Submitted
   ≠
Dispute Won

Provider Dispute State
   ≠
Canonical Baobab State

Chargeback Amount
   ≠ necessarily
Original Payment Amount

Dispute Deadline
   ≠
Business SLA

Settlement Debit
   ≠
Canonical Dispute Identity
```

---

# 7. Authority Boundaries

| Concern | Authority |
|---|---|
| Canonical Payment execution | Payments |
| Canonical Dispute lifecycle | Payments |
| External dispute processing | Provider / payment rail |
| Order/commerce facts | Trade |
| Subscription/billing facts | Subscriptions |
| Canonical business context | Control Plane |
| Workload/user identity | IAM |
| Evidence business facts | Source domain |
| Evidence artefact storage | Approved document/object storage |
| Evidence orchestration/submission | Payments |
| Accounting consequence | ERP |
| Settlement reconciliation | Payments + ERP boundary |
| Provider certification | PAY-0022 |

---

# 8. Dispute Aggregate

A Dispute SHALL have its own canonical identity.

Conceptually:

```text
Dispute
│
├── dispute_id
├── payment_id
├── payment_intent_id?
├── tenant_id
├── organisation_id?
├── legal_entity_id
├── market_id?
├── source_engine
├── source_reference?
├── external_dispute_reference
├── provider
├── dispute_type
├── reason_category
├── disputed_amount
├── currency
├── status
├── response_required
├── response_deadline?
├── evidence_status?
├── opened_at
├── updated_at
├── resolved_at?
├── correlation_id
├── simulated
└── revision
```

Exact contract fields SHALL be versioned through Shared canonical contracts.

---

# 9. Canonical Identity

Provider dispute IDs SHALL remain:

```text
ExternalReference
```

values.

They SHALL NOT replace:

```text
dispute_id
```

as canonical Baobab identity.

---

# 10. One Payment, Multiple Disputes

A Payment MAY have:

```text
zero
one
many
```

Disputes where the provider/rail permits this.

Therefore:

```text
Payment P1
   │
   ├── Dispute D1
   └── Dispute D2
```

is valid.

---

# 11. Disputed Amount

A Dispute SHALL carry its own disputed amount.

It SHALL NOT be assumed that:

```text
disputed amount
=
captured amount
```

Partial disputes are possible.

---

# 12. Currency

The Dispute SHALL preserve the relevant transaction/dispute currency explicitly.

Currency SHALL not be inferred from the current Market configuration.

---

# 13. Dispute Reason

Provider-specific reason codes SHALL be normalised into canonical reason categories where useful.

Potential categories may include:

```text
FRAUD
UNRECOGNISED
DUPLICATE
PRODUCT_NOT_RECEIVED
SERVICE_NOT_PROVIDED
PRODUCT_UNACCEPTABLE
CREDIT_NOT_PROCESSED
CANCELLED_RECURRING_PAYMENT
INCORRECT_AMOUNT
PROCESSING_ERROR
AUTHORIZATION
OTHER
```

Exact canonical vocabulary SHALL be governed through Shared contracts.

---

# 14. Preserve Provider Reason

Normalisation SHALL NOT discard the original provider reason code.

Conceptually:

```text
canonical_reason = PRODUCT_NOT_RECEIVED

provider_reason:
    provider = ProviderA
    code = ...
    description = ...
```

where safe and appropriate.

---

# 15. Dispute State Machine

A provider-independent lifecycle SHALL be maintained.

Conceptually:

```text
OPEN
 │
 ▼
RESPONSE_REQUIRED
 │
 ├──────────────► ACCEPTED
 │
 ▼
EVIDENCE_PREPARING
 │
 ▼
EVIDENCE_SUBMITTED
 │
 ▼
UNDER_REVIEW
 │
 ├──────────────► WON
 │
 └──────────────► LOST
```

Not every rail follows every state.

---

# 16. Canonical States

Canonical semantics SHOULD be capable of representing:

```text
OPEN
RESPONSE_REQUIRED
EVIDENCE_PREPARING
EVIDENCE_SUBMITTED
UNDER_REVIEW
ACCEPTED
WON
LOST
CLOSED
```

and, where necessary:

```text
EXPIRED
CANCELLED
UNKNOWN
```

Exact contract enumeration may evolve.

---

# 17. Provider Translation

Provider states SHALL pass through:

```text
Provider State
      │
      ▼
HyperSwitch Adapter
      │
      ▼
Baobab Normalisation
      │
      ▼
Canonical Dispute State
```

No leaf engine SHALL interpret provider-specific dispute states independently.

---

# 18. Historical Integrity

Opening a Dispute SHALL NOT rewrite:

```text
Payment CAPTURED
```

to:

```text
Payment FAILED
```

Likewise, losing a chargeback does not erase the historical capture.

---

# 19. Financial Exposure

Payments SHOULD separately represent dispute-related financial exposure.

Conceptually:

```text
Payment captured
       │
       ▼
Dispute opened
       │
       ▼
Potential exposure
       │
       ▼
Chargeback debit?
       │
       ▼
Representment?
       │
       ▼
Final financial outcome
```

---

# 20. Dispute Versus Chargeback

A Dispute represents the broader challenge/case lifecycle.

A Chargeback represents a financial action that may occur within or because of that lifecycle.

Therefore:

```text
Dispute
   │
   ├── case lifecycle
   │
   └── may cause
          │
          ▼
      Chargeback
```

---

# 21. Chargeback Aggregate

Baobab MAY model chargeback financial movements as:

- subordinate financial records under Dispute;
- canonical financial adjustments;
- settlement adjustments;

depending on implementation maturity.

It SHALL NOT hide chargeback movement solely inside Dispute status.

---

# 22. Chargeback Amount

The actual chargeback amount SHALL be explicit.

Potential components may include:

```text
disputed principal
chargeback fee
network fee
provider fee
currency adjustment
other adjustment
```

where available.

---

# 23. Principal Versus Fee

Baobab SHALL distinguish:

```text
customer transaction principal
```

from:

```text
provider/network chargeback fee
```

ERP determines accounting treatment.

---

# 24. Pre-Dispute

Where providers support early-warning or pre-dispute processes, Payments MAY represent them separately or as an early canonical Dispute stage.

The architecture SHALL preserve the distinction where financially material.

---

# 25. Inquiry

A provider inquiry or retrieval request is not necessarily a Chargeback.

It MAY require evidence while no financial debit has yet occurred.

---

# 26. Acceptance

A business may choose not to contest an eligible Dispute.

Conceptually:

```text
Dispute
   │
   ▼
ACCEPT
   │
   ▼
Provider / Rail
   │
   ▼
Financial outcome
```

Acceptance is a controlled mutation.

---

# 27. Contest

Where permitted:

```text
Dispute
   │
   ▼
CONTEST
   │
   ▼
Evidence Package
   │
   ▼
Provider
   │
   ▼
Rail Review
```

---

# 28. Business Decision Versus Provider Decision

Baobab may decide:

```text
contest this dispute
```

but Baobab does not decide:

```text
merchant won
```

until authoritative external evidence establishes the outcome.

---

# 29. Representment

Representment SHALL be treated as a controlled response to a Dispute.

It SHALL NOT create a new Payment.

---

# 30. Evidence

Evidence is information used to support the response to a Dispute.

Examples may include:

```text
order details
invoice
contract
customer communication
delivery confirmation
shipment tracking
proof of service
refund policy
cancellation policy
usage evidence
authentication evidence
transaction metadata
```

subject to provider/rail requirements.

---

# 31. Evidence Ownership

The business facts underlying evidence remain owned by their authoritative domains.

For example:

| Evidence | Authoritative Source |
|---|---|
| Order | Trade |
| Shipment | Trade / logistics integration |
| Invoice | ERP |
| Subscription | Subscriptions |
| Payment | Payments |
| Payment authentication | Payments/provider |
| Customer identity | authoritative customer/IAM domain |
| Legal Entity | Control Plane |
| Contract/document | authoritative document/business domain |

Payments SHALL not recreate those master records.

---

# 32. Evidence Orchestration

Payments owns:

```text
what dispute requires evidence
which evidence package is associated
submission state
provider submission
deadline tracking
provider response
```

It does not become master authority for the underlying business facts.

---

# 33. Evidence References

Payments SHOULD store references to evidence rather than unnecessary duplicate copies.

Conceptually:

```text
Dispute
   │
   ▼
EvidencePackage
   │
   ├── EvidenceReference
   ├── EvidenceReference
   └── EvidenceReference
```

---

# 34. Evidence Package

Conceptually:

```text
EvidencePackage
│
├── evidence_package_id
├── dispute_id
├── version
├── status
├── prepared_by
├── approved_by?
├── submitted_at?
├── provider_submission_reference?
└── EvidenceReference[]
```

---

# 35. Evidence Reference

Conceptually:

```text
EvidenceReference
│
├── evidence_id
├── type
├── source_engine
├── source_reference
├── artefact_reference?
├── content_hash?
├── created_at
├── classification
└── provenance
```

---

# 36. Evidence Storage

Large binary evidence SHALL NOT be stored directly in canonical payment tables by default.

Approved:

```text
document/object storage
```

SHOULD hold binary artefacts.

Payments stores controlled references and metadata.

---

# 37. CMS Boundary

`baobab-cms` SHALL NOT automatically become the dispute evidence repository merely because it manages content.

Dispute evidence storage requires appropriate:

```text
access control
immutability
retention
classification
audit
```

characteristics.

---

# 38. Evidence Immutability

Evidence submitted to a provider SHALL be reproducible.

Once submitted, the exact package SHOULD be preserved or cryptographically identifiable.

---

# 39. Content Hash

Where practical, evidence artefacts SHOULD carry cryptographic integrity metadata.

Conceptually:

```text
document
   │
   ▼
SHA-256 or approved equivalent
   │
   ▼
EvidenceReference
```

Exact cryptographic policy belongs to security architecture.

---

# 40. Evidence Versioning

Editing an evidence package after submission SHALL create a new version rather than silently rewriting what was previously submitted.

---

# 41. Evidence Provenance

For every material evidence item, Baobab SHOULD be able to answer:

```text
Where did this evidence originate?

Which canonical record supports it?

When was it collected?

Who or what collected it?

Was it changed?

Which version was submitted?
```

---

# 42. Evidence Authenticity

Baobab SHALL NOT fabricate evidence.

Generated summaries MAY assist operators, but the authoritative evidence SHALL remain traceable to actual source records.

---

# 43. AI-Assisted Evidence

Future AI assistance MAY:

```text
summarise
classify
identify potentially relevant records
draft explanatory narratives
```

but SHALL NOT manufacture factual evidence.

AI-generated material SHALL be distinguishable from source evidence where used.

---

# 44. Pulse Boundary

`baobab-pulse` MAY assist with analysis where authorised.

It SHALL NOT become authoritative for whether an order was delivered, a Payment occurred, or a customer communicated something.

---

# 45. Sensitive Data

Evidence packages may contain:

```text
personal information
addresses
communications
transaction information
delivery information
authentication data
```

PAY-0012 minimisation principles apply.

---

# 46. Payment Credentials

Evidence SHALL NOT contain raw:

```text
PAN
CVV
provider secrets
private keys
```

unless a specific lawful and compliant process explicitly requires information permitted under applicable standards.

---

# 47. Redaction

Evidence SHOULD be redacted to the minimum data required by the provider/rail.

---

# 48. Deadline

Dispute deadlines are financially significant.

Payments SHALL treat:

```text
response_deadline
```

as first-class operational data where supplied.

---

# 49. Provider Deadline

The provider/rail deadline is authoritative for the external case.

An internal SLA SHALL not replace it.

---

# 50. Deadline Normalisation

Deadlines SHALL preserve sufficient information to avoid timezone ambiguity.

---

# 51. Deadline Changes

If a provider updates the response deadline, Payments SHALL preserve the change and update the current canonical projection.

Historical deadline changes SHOULD remain auditable.

---

# 52. Deadline Monitoring

Payments SHALL monitor open Disputes approaching response deadlines.

---

# 53. Escalation

Approaching deadlines SHOULD produce appropriate:

```text
notification
workflow escalation
operational alert
```

according to tenant policy.

---

# 54. Missed Deadline

A missed deadline SHALL NOT be silently converted to:

```text
LOST
```

unless authoritative provider state establishes that result.

It may instead produce:

```text
response expired
```

or equivalent canonical fact.

---

# 55. Evidence Collection Workflow

Conceptually:

```text
Dispute Opened
      │
      ▼
Classify Reason
      │
      ▼
Determine Required Evidence
      │
      ▼
Resolve Authoritative Sources
      │
      ▼
Collect Evidence References
      │
      ▼
Review / Approval
      │
      ▼
Submit
      │
      ▼
Provider Review
      │
      ▼
Canonical Outcome
```

---

# 56. Provider Requirements

Evidence requirements MAY vary by:

```text
provider
payment rail
reason code
payment method
market
transaction type
```

Payments SHALL support provider-specific translation behind the canonical boundary.

---

# 57. Evidence Templates

Baobab MAY define provider-independent evidence categories while adapters translate them to provider-specific fields.

---

# 58. No Leaf Provider Logic

ZuriBeans, Thamani and future estates SHALL NOT contain logic such as:

```text
if provider == X:
    submit evidence field Y
```

Provider-specific transformation belongs in Payments adapters.

---

# 59. Trade Integration

When evidence requires commerce facts:

```text
Payments
   │
   ▼
Trade API / canonical projection
   │
   ▼
Order / shipment facts
   │
   ▼
Evidence Reference
```

No direct database coupling is permitted.

---

# 60. ERP Integration

ERP may provide:

```text
invoice
credit note
financial document
```

where appropriate.

Payments SHALL not query ERP database tables directly.

---

# 61. Subscription Integration

Subscription disputes may require:

```text
subscription agreement
billing period
cancellation record
usage record
renewal history
```

Subscriptions remains authoritative for these facts.

---

# 62. Evidence Snapshot

Evidence used in a dispute SHOULD represent the facts relevant at the time of the disputed transaction.

Current mutable state SHALL not silently replace historical facts.

---

# 63. Delivery Evidence

A later change to an order or shipment record SHALL not alter the evidence package already submitted.

---

# 64. Customer Communication

Customer communications used as evidence SHALL retain provenance and appropriate privacy controls.

---

# 65. Fraud Evidence

Fraud-related evidence SHALL be handled under restricted access appropriate to its sensitivity.

---

# 66. Dispute Opening

A canonical Dispute MAY originate from:

```text
verified provider webhook
provider polling/recovery
reconciliation
controlled administrative import
```

It SHALL NOT originate from an unverified arbitrary client request pretending a provider dispute exists.

---

# 67. Webhook Path

Preferred flow:

```text
Provider
   │
   ▼
HyperSwitch
   │
   ▼
Payments Webhook Ingress
   │
   ▼
PAY-0011 Verification
   │
   ▼
Resolve Payment
   │
   ▼
Create/Update Dispute
   │
   ▼
Outbox
   │
   ▼
Canonical Dispute Event
```

---

# 68. Unknown Dispute

A valid provider dispute referring to an unknown Payment SHALL enter reconciliation.

It SHALL not be discarded.

---

# 69. Cross-Tenant Safety

An external dispute reference SHALL NOT be sufficient to establish tenant ownership.

Payments resolves the associated canonical Payment and authoritative context.

---

# 70. Cross-Legal-Entity Safety

A dispute against a ZuriBeans Payment remains attributable to ZuriBeans even if another group subsidiary uses the same provider.

---

# 71. Shared Provider

Shared provider infrastructure SHALL NOT collapse:

```text
tenant
Legal Entity
merchant
dispute ownership
```

---

# 72. Merchant Context

The historical merchant/profile/provider context of the original Payment SHALL be preserved.

---

# 73. Provider Migration

Provider deactivation for new transactions SHALL not destroy access to historical Disputes.

PAY-0022 deactivation SHOULD preserve necessary:

```text
refund
dispute
settlement
reconciliation
```

operations.

---

# 74. Provider Exit

Provider offboarding SHALL account for unresolved Disputes and future dispute windows.

---

# 75. Historical Dispute Window

A provider relationship cannot be considered operationally terminated merely because new Payments stopped yesterday.

Historical transactions may remain disputable.

---

# 76. Refund Interaction

Before issuing a Refund against a disputed Payment, Payments SHALL evaluate existing financial exposure.

---

# 77. Refund Does Not Automatically Close Dispute

A merchant Refund does not necessarily cause an external Dispute to close.

Provider/rail state remains authoritative.

---

# 78. Dispute Does Not Automatically Block Refund

Some workflows may legitimately permit Refund during a dispute.

Others may create duplicate financial exposure.

Payments SHALL apply provider capability and policy rather than a universal assumption.

---

# 79. Duplicate Return Protection

Payments SHALL prevent avoidable:

```text
Refund
+
Chargeback
```

double loss.

This requires current financial exposure analysis.

---

# 80. Refund Before Dispute

If a full Refund already succeeded and a subsequent Dispute arrives, Payments SHALL preserve both facts and reconcile the external claim.

It SHALL not assume the dispute is invalid.

---

# 81. Refund Processing During Dispute

A pending Refund creates additional uncertainty.

Payments SHOULD avoid further financial action until sufficient state is known where duplicate return risk exists.

---

# 82. Chargeback Financial Movement

A chargeback debit may occur before the Dispute reaches its final state.

Therefore:

```text
financial movement
   ≠
final case outcome
```

---

# 83. Temporary Debit

A provider may debit the merchant during dispute processing and later return funds after a successful representment.

Both movements SHALL be observable.

---

# 84. Dispute Won

`WON` means the external dispute process has reached an authoritative outcome favourable to the merchant according to the provider/rail.

It does not mean:

```text
no financial adjustment ever occurred
```

---

# 85. Dispute Lost

`LOST` means the external process has reached an authoritative adverse outcome.

ERP still determines the accounting treatment.

---

# 86. Accepted Dispute

`ACCEPTED` means Baobab/the authorised merchant workflow chose not to contest, where the provider supports that operation.

It SHALL be distinct from an involuntary `LOST`.

---

# 87. Closed

`CLOSED` MAY represent an administratively/finally completed case where more specific final outcome is separately preserved.

`CLOSED` SHALL not erase:

```text
WON
LOST
ACCEPTED
```

semantics.

---

# 88. Unknown Outcome

When external outcome cannot be established:

```text
UNKNOWN
```

or equivalent uncertainty SHALL be represented rather than guessing.

---

# 89. Polling / Recovery

Payments MAY query provider/HyperSwitch dispute state to recover:

```text
missing webhooks
unknown state
stale case
deadline updates
```

---

# 90. Idempotency

Duplicate dispute notifications SHALL not create duplicate canonical Disputes.

PAY-0010 principles apply.

---

# 91. External Event Identity

Provider event IDs and dispute IDs SHALL participate in deduplication without becoming canonical identity.

---

# 92. Event Ordering

Provider dispute events MAY arrive:

```text
late
duplicated
out of order
```

PAY-0011 ordering and state-transition protections apply.

---

# 93. Monotonic Financial Facts

A later stale `OPEN` event SHALL not erase a confirmed final `LOST` or `WON` outcome.

---

# 94. Corrections

Where a provider legitimately reopens or changes a dispute outcome, Baobab SHALL record a new authoritative transition rather than rewriting history.

---

# 95. Canonical Events

Representative event families MAY include:

```text
dispute.opened
dispute.updated
dispute.response_required
dispute.evidence_preparing
dispute.evidence_submitted
dispute.accepted
dispute.won
dispute.lost
dispute.closed

chargeback.observed
chargeback.reversed
```

Exact names SHALL be governed through canonical contracts.

---

# 96. Events Are Facts

An event:

```text
dispute.evidence_submitted
```

means evidence submission was established.

It does not mean the dispute was won.

---

# 97. Outbox

Canonical dispute events SHALL use PAY-0011 transactional-outbox semantics.

---

# 98. Consumers

Trade, Subscriptions, ERP and other authorised consumers SHALL process dispute events idempotently.

---

# 99. Trade Consequence

Trade decides whether a Dispute causes:

```text
order review
customer restriction
fulfilment hold
risk review
```

according to business policy.

Payments does not mutate Order state.

---

# 100. Subscription Consequence

Subscriptions decides whether a Dispute affects:

```text
renewal
entitlement
delinquency
account status
```

Payments does not directly cancel the Subscription.

---

# 101. ERP Consequence

ERP determines accounting consequences of:

```text
chargeback principal
chargeback fee
reversal
representment recovery
settlement adjustment
```

---

# 102. Accounting Is Not Dispute State

An ERP journal entry SHALL not change canonical Dispute state.

---

# 103. Settlement Interaction

Dispute financial movements may appear in settlement reports.

PAY-0015 SHALL reconcile:

```text
Payment
Refund
Dispute
Chargeback
Chargeback Fee
Reversal
Settlement
```

---

# 104. Settlement Does Not Define Case Outcome

A settlement debit may be evidence of a chargeback movement.

It does not necessarily establish final Dispute resolution.

---

# 105. Evidence Submission Authorization

Evidence submission is a privileged mutation.

The caller/workload SHALL have appropriate authorization.

---

# 106. Dispute Acceptance Authorization

Accepting a Dispute is financially consequential and SHALL require explicit authorization.

---

# 107. Approval Thresholds

Tenants MAY configure approval policy based on:

```text
disputed amount
currency
reason
Legal Entity
market
role
risk
```

subject to canonical governance.

---

# 108. Separation of Duties

High-value dispute acceptance or evidence submission MAY require dual control.

---

# 109. No Caller-Controlled Tenant Context

A caller SHALL NOT switch dispute ownership by supplying:

```text
tenant_id
legal_entity_id
merchant_id
```

in conflict with the original Payment.

Historical canonical context wins.

---

# 110. Workload Authentication

Machine-to-machine dispute workflows SHALL use authenticated workload identity.

Static production bearer secrets remain prohibited under Shared/IAM architecture.

---

# 111. Provider Dashboard

Provider or HyperSwitch administrative consoles MAY be used operationally.

They SHALL NOT become the canonical dispute system of record for Baobab.

---

# 112. Manual Provider Action

If an operator performs an emergency dispute action directly at the provider:

```text
Provider
   │
   ▼
out-of-band mutation
```

Baobab SHALL reconcile it into canonical state.

---

# 113. Out-of-Band Detection

Out-of-band:

```text
evidence submission
dispute acceptance
chargeback adjustment
```

SHOULD be detectable through reconciliation.

---

# 114. Evidence Retention

Evidence SHALL be retained for the period required by applicable:

```text
payment-rail
contractual
legal
regulatory
audit
```

requirements.

---

# 115. Evidence Deletion

Evidence SHALL not be retained indefinitely without documented need.

Deletion SHALL consider unresolved disputes and subsequent appeal/arbitration windows.

---

# 116. Privacy

Dispute evidence may contain substantial personal information.

Applicable privacy requirements, including POPIA where relevant, SHALL be considered separately from payment-network requirements.

---

# 117. Data Residency

Evidence storage location MAY be constrained by:

```text
tenant
Legal Entity
jurisdiction
data-residency policy
provider requirement
```

The Baobab Market alone does not determine storage location.

---

# 118. Encryption

Evidence artefacts SHALL use appropriate:

```text
encryption in transit
encryption at rest
access control
```

according to classification.

---

# 119. Evidence Access

Access SHALL be limited to principals/workloads requiring the evidence for:

```text
dispute management
audit
legal/compliance
authorised operations
```

---

# 120. Signed Access

Where object storage is used, temporary/signed access mechanisms SHOULD be preferred over broadly public artefact URLs.

---

# 121. No Public Evidence URLs

Dispute evidence SHALL NOT be exposed through unauthenticated public URLs.

---

# 122. Logging

Evidence contents SHALL not be dumped into logs.

Logs SHOULD record:

```text
evidence_id
artefact reference
classification
submission state
hash
```

rather than content.

---

# 123. Observability

A Dispute SHOULD be traceable through:

```text
dispute_id
payment_id
payment_intent_id
tenant_id
legal_entity_id
market_id
provider
merchant/profile
external_dispute_reference
correlation_id
evidence_package_id
```

without sensitive evidence content.

---

# 124. Metrics

Representative metrics MAY include:

```text
disputes_opened_total
disputes_response_required_total
disputes_won_total
disputes_lost_total
disputes_accepted_total
dispute_amount_total
chargeback_amount_total
chargeback_fee_total
evidence_submission_total
evidence_submission_failure_total
dispute_deadline_at_risk_total
unknown_dispute_total
out_of_band_dispute_action_total
```

Exact telemetry names remain implementation-specific.

---

# 125. Operational Alerts

Alerts SHOULD cover:

```text
new high-value dispute
response deadline approaching
deadline missed
evidence submission failure
provider dispute API failure
unknown dispute state
chargeback spike
dispute-rate anomaly
unmatched dispute
out-of-band provider action
```

---

# 126. Dispute Rate

Dispute-rate measurement MAY be operationally and commercially important.

Its exact calculation SHALL identify:

```text
population
time period
payment method
provider
Legal Entity
market
```

rather than publishing ambiguous percentages.

---

# 127. Provider Monitoring

Provider dispute performance MAY inform future provider governance.

It SHALL NOT bypass PAY-0022 certification/activation processes.

---

# 128. Fraud Systems

Future fraud/risk systems MAY consume canonical dispute outcomes as feedback signals.

They SHALL not become canonical dispute authority.

---

# 129. No Automatic Guilt Inference

A Dispute does not prove customer fraud or merchant wrongdoing.

Baobab SHALL treat it as a financial/payment-rail case with an externally governed outcome.

---

# 130. No Automatic Customer Ban

Payments SHALL NOT automatically ban a customer because a Dispute exists.

Any such policy belongs to the appropriate business/risk authority.

---

# 131. Evidence Quality

Baobab MAY validate evidence packages for:

```text
required fields
supported file type
size
integrity
source provenance
deadline
```

before submission.

---

# 132. Provider Transformation

The adapter MAY transform canonical evidence categories into provider-specific payloads.

Example:

```text
Canonical:
PROOF_OF_DELIVERY

       │
       ▼

Provider A:
shipping_document

Provider B:
delivery_confirmation
```

The canonical concept remains provider-independent.

---

# 133. Provider-Specific Fields

Provider-specific fields MAY be stored in namespaced metadata when necessary.

They SHALL not pollute the canonical domain unnecessarily.

---

# 134. Evidence Narrative

Some providers require explanatory text.

Baobab MAY generate or accept a controlled evidence narrative.

The narrative SHALL remain attributable to its source/author.

---

# 135. No Fabricated Narrative

Automated narrative generation SHALL not introduce unsupported factual claims.

---

# 136. Evidence Completeness

Payments MAY calculate:

```text
evidence package complete
```

according to the provider/rail requirements known to it.

This does not imply evidence is persuasive or that the dispute will be won.

---

# 137. No Outcome Prediction

Baobab SHALL NOT treat evidence completeness as a guarantee of dispute outcome.

---

# 138. Dispute Acceptance Flow

```text
Provider Dispute
      │
      ▼
Verified / Canonicalised
      │
      ▼
Dispute OPEN
      │
      ▼
Authorised business decision
      │
      ▼
ACCEPT
      │
      ▼
Provider
      │
      ▼
Confirmed external state
      │
      ▼
Canonical ACCEPTED / final outcome
      │
      ▼
ERP settlement/accounting
```

---

# 139. Contest Flow

```text
Provider Dispute
      │
      ▼
Canonical Dispute
      │
      ▼
RESPONSE_REQUIRED
      │
      ▼
Evidence Requirements
      │
      ▼
Authoritative Sources
      │
 ┌────┼───────────┐
 ▼    ▼           ▼
Trade ERP   Subscriptions
      │
      ▼
Evidence Package
      │
      ▼
Review / Approval
      │
      ▼
Payments Adapter
      │
      ▼
HyperSwitch / Provider
      │
      ▼
UNDER_REVIEW
      │
   ┌──┴───┐
   ▼      ▼
 WON     LOST
```

---

# 140. Chargeback Flow

```text
Payment CAPTURED
      │
      ▼
Dispute OPENED
      │
      ▼
Provider debits merchant
      │
      ▼
Chargeback Financial Fact
      │
      ├────► ERP projection
      │
      ▼
Dispute continues
      │
      ▼
Representment
      │
   ┌──┴─────────────┐
   ▼                ▼
Merchant wins     Merchant loses
   │                │
   ▼                ▼
Funds may return  Debit remains
   │                │
   └───────┬────────┘
           ▼
Settlement reconciliation
```

---

# 141. Refund/Dispute Interaction

```text
                 Payment
                    │
                    ▼
                 CAPTURED
                    │
          ┌─────────┴─────────┐
          ▼                   ▼
       Refund              Dispute
          │                   │
          ▼                   ▼
  Merchant return       External challenge
          │                   │
          └─────────┬─────────┘
                    ▼
          Financial exposure
                    │
                    ▼
             Reconciliation
```

Neither branch erases the other.

---

# 142. Evidence Architecture

```text
                     DISPUTE
                        │
                        ▼
                EVIDENCE PACKAGE
                        │
        ┌───────────────┼──────────────┐
        ▼               ▼              ▼
      Trade            ERP       Subscriptions
        │               │              │
        ▼               ▼              ▼
 Order/Shipment      Invoice      Billing facts
        │               │              │
        └───────────────┼──────────────┘
                        ▼
                Evidence References
                        │
                        ▼
              Approved Object Store
                        │
                        ▼
                   PAYMENTS
                        │
                        ▼
             Provider Transformation
                        │
                        ▼
              HyperSwitch / Provider
```

---

# 143. Production Readiness Gate

```text
DISPUTE / CHARGEBACK READINESS

Canonical Dispute aggregate                 PASS
Provider dispute mapping                    PASS
Reason-code normalisation                   PASS
Dispute state machine                       PASS
Chargeback financial representation         PASS
Partial dispute support                     PASS
Multiple dispute support                    PASS
Refund/dispute interaction                  PASS
Duplicate-return protection                 PASS
Evidence model                              PASS
Evidence provenance                         PASS
Evidence versioning                         PASS
Evidence integrity                          PASS
Secure artefact storage                     PASS
Evidence access controls                    PASS
Provider evidence transformation            PASS
Deadline tracking                           PASS
Deadline alerting                           PASS
Dispute acceptance                          PASS
Representment                               PASS
Authorization                               PASS
Approval policy                             PASS
Cross-tenant isolation                      PASS
Cross-Legal-Entity isolation                PASS
Historical merchant context                 PASS
Provider deactivation handling              PASS
Webhook processing                          PASS
Idempotency                                 PASS
Out-of-order handling                       PASS
Recovery/polling                            PASS
Out-of-band reconciliation                  PASS
Canonical events                            PASS
Transactional outbox                        PASS
ERP projection                              PASS
Trade projection                            PASS
Subscription projection                     PASS
Settlement reconciliation                   PASS
Privacy/data minimisation                    PASS
Observability                               PASS
Audit                                       PASS
Sandbox/provider testing                     PASS
-------------------------------------------------
Dispute & Chargeback Plane                  READY
```

---

# 144. Invariants

The following invariants SHALL hold:

1. Dispute is not Payment.
2. Dispute is not Refund.
3. Chargeback is not Refund.
4. Chargeback is not Payment failure.
5. Dispute has its own canonical identity.
6. Provider dispute ID remains an external reference.
7. One Payment may have multiple Disputes.
8. Disputed amount is explicit.
9. Disputed amount need not equal captured amount.
10. Dispute currency is explicit.
11. Provider reason codes do not become canonical vocabulary directly.
12. Original provider reason is preserved where useful.
13. Payment capture history is not rewritten when a Dispute opens.
14. Losing a Dispute does not erase historical capture.
15. Chargeback movement and Dispute case state remain distinct.
16. Chargeback principal and fees remain distinguishable.
17. Evidence submitted does not mean Dispute won.
18. Dispute opened does not mean merchant lost.
19. Financial debit does not necessarily mean final outcome.
20. Business acceptance and involuntary loss remain distinguishable.
21. Payments owns canonical Dispute state.
22. Provider/rail owns external adjudication.
23. ERP owns accounting consequence.
24. Trade owns commerce consequence.
25. Subscriptions owns subscription consequence.
26. Control Plane owns canonical business context.
27. IAM owns identity/authorization.
28. Payments does not recreate authoritative business records as evidence masters.
29. Evidence retains source provenance.
30. Submitted evidence remains reproducible.
31. Evidence changes after submission create new version/history.
32. Evidence is not fabricated.
33. AI assistance cannot manufacture factual evidence.
34. Evidence artefacts are protected according to classification.
35. Raw payment credentials are excluded from evidence.
36. Evidence contents are not dumped into logs.
37. Provider deadlines are first-class operational data.
38. Internal SLA does not replace provider deadline.
39. Missing a deadline does not automatically imply canonical LOST without provider evidence.
40. Provider-specific evidence mapping remains behind Payments adapters.
41. Leaf estates contain no provider-specific dispute integration.
42. Dispute webhook input is untrusted until PAY-0011 validation.
43. Duplicate provider events do not create duplicate Disputes.
44. Out-of-order events cannot erase later authoritative state.
45. Unknown external Disputes enter reconciliation.
46. External IDs cannot establish tenant ownership.
47. Cross-tenant dispute access is prohibited.
48. Cross-Legal-Entity dispute access is prohibited.
49. Group ownership does not collapse dispute ownership.
50. Historical merchant/provider context is preserved.
51. Provider deactivation does not erase historical dispute obligations.
52. Provider offboarding accounts for unresolved dispute windows.
53. Refund does not automatically close a Dispute.
54. Dispute does not automatically justify Refund.
55. Refund/chargeback double-return risk must be evaluated.
56. Pending Refunds contribute to financial exposure.
57. Provider console actions do not replace canonical state.
58. Out-of-band dispute mutations require reconciliation.
59. Evidence submission is privileged.
60. Dispute acceptance is privileged.
61. Knowing a dispute/payment ID does not grant mutation authority.
62. Canonical events use transactional-outbox semantics.
63. Consumers process dispute events idempotently.
64. Settlement movement does not automatically determine dispute case outcome.
65. Dispute financial effects are reconciled against provider settlement.
66. Financial history remains auditable.
67. Current routing changes do not rewrite historical dispute context.
68. Sandbox Disputes remain `simulated=true`.
69. Simulated dispute records cannot become production records.
70. Provider-specific terminology never overrides canonical financial semantics.

---

# 145. Consequences

## Positive

This architecture provides:

- provider-independent dispute semantics;
- historical payment integrity;
- proper chargeback accounting boundaries;
- evidence provenance;
- deadline management;
- multi-provider portability;
- Legal Entity isolation;
- safe refund/dispute interaction;
- auditable representment;
- controlled evidence handling;
- provider-offboarding safety;
- settlement reconciliation;
- future fraud/risk integration.

## Negative

It requires:

- a dedicated Dispute aggregate;
- evidence orchestration;
- secure object/document storage;
- deadline workflows;
- provider-specific evidence adapters;
- reconciliation;
- financial-exposure tracking;
- operational review workflows;
- long-lived provider relationships for historical cases;
- privacy and retention controls.

These costs are accepted because disputes may occur long after payment execution and can materially alter financial exposure.

---

# 146. Alternatives Considered

## Add CHARGEBACK to Payment.status

Rejected.

A Payment can remain historically captured while simultaneously having a Dispute.

---

## Model Chargeback as Refund

Rejected.

The initiating authority and financial lifecycle are fundamentally different.

---

## Let Trade own Disputes

Rejected.

Trade owns commerce facts, not payment-rail case state.

---

## Let ERP own Disputes

Rejected.

ERP owns accounting consequences, not payment-provider dispute execution.

---

## Store all evidence inside Payments PostgreSQL

Rejected as the default.

Large artefacts require a more appropriate protected storage boundary.

---

## Use provider dashboards as system of record

Rejected.

This would fragment Baobab canonical financial state and governance.

---

## Automatically refund every disputed Payment

Rejected.

It can create duplicate financial loss and bypass dispute policy.

---

## Automatically contest every Dispute

Rejected.

Contest decisions may require business policy, evidence quality, amount thresholds and Legal Entity approval.

---

## Treat chargeback debit as final loss

Rejected.

Representment may later recover funds.

---

## Treat evidence submission as successful representment

Rejected.

Only the external provider/rail determines outcome.

---

# 147. Relationship to PAY-0013

PAY-0013 governs merchant/business-originated:

```text
Refund
Void
Cancellation
```

PAY-0014 governs externally initiated:

```text
Dispute
Chargeback
Claim
```

The interaction is:

```text
                PAYMENT
                   │
          ┌────────┴────────┐
          ▼                 ▼
      PAY-0013          PAY-0014
 Merchant return     External challenge
          │                 │
          └────────┬────────┘
                   ▼
            Financial exposure
                   │
                   ▼
              PAY-0015
         Settlement/Reconciliation
```

---

# 148. Relationship to PAY-0015

PAY-0015 SHALL reconcile dispute financial movements against settlement.

The settlement plane must be able to distinguish:

```text
payment capture
refund
chargeback principal
chargeback fee
chargeback reversal
representment recovery
other adjustment
```

without using a single undifferentiated balance adjustment.

---

# 149. Relationship to PAY-0018

PAY-0018 SHALL define detailed authorization for:

```text
dispute read
evidence preparation
evidence submission
dispute acceptance
administrative reconciliation
```

including separation of duties where appropriate.

---

# 150. Relationship to PAY-0019

PAY-0019 SHALL define operational monitoring for:

```text
deadline risk
submission failures
unmatched disputes
unknown states
provider API failures
dispute backlog
chargeback anomalies
```

---

# 151. Relationship to PAY-0020

Dispute and evidence records may outlive ordinary payment execution by a substantial period.

Backup/recovery architecture SHALL preserve:

```text
Dispute history
Evidence references
Evidence integrity metadata
Deadlines
Submission records
Provider references
Audit history
```

---

# 152. Relationship to PAY-0021

HyperSwitch upgrades SHALL be reviewed for changes to:

```text
dispute APIs
webhook schemas
reason codes
evidence fields
provider adapters
chargeback semantics
```

before adoption.

---

# 153. Relationship to PAY-0022

Provider certification SHALL consider dispute capability.

Where relevant, certification SHOULD determine whether the provider integration supports:

```text
dispute notification
dispute query
reason-code mapping
evidence submission
acceptance
representment
deadline reporting
chargeback financial data
chargeback reversal
settlement reconciliation
```

A provider capable of collecting money but incapable of meeting the Legal Entity's required dispute operations may be unsuitable for production activation.

---

# 154. Complete Dispute Architecture

```text
                         PAYMENT
                            │
                            ▼
                         CAPTURED
                            │
                            ▼
                  PROVIDER / PAYMENT RAIL
                            │
                            ▼
                      DISPUTE OPENED
                            │
                            ▼
                     BAOBAB PAYMENTS
                            │
                   Canonical Dispute
                            │
              ┌─────────────┴──────────────┐
              │                            │
              ▼                            ▼
       ACCEPT / RESPOND             EVIDENCE REQUIRED
                                           │
                               ┌───────────┼───────────┐
                               ▼           ▼           ▼
                             Trade        ERP    Subscriptions
                               │           │           │
                               └───────────┼───────────┘
                                           ▼
                                   Evidence Package
                                           │
                                           ▼
                                  Protected Storage
                                           │
                                           ▼
                                    Baobab Payments
                                           │
                                           ▼
                                  HyperSwitch/Provider
                                           │
                                           ▼
                                     Rail Decision
                                           │
                                  ┌────────┴────────┐
                                  ▼                 ▼
                                WON                LOST
                                  │                 │
                                  └────────┬────────┘
                                           ▼
                                Financial Adjustments
                                           │
                                           ▼
                                      PAY-0015
                               Settlement/Reconciliation
                                           │
                                           ▼
                                          ERP
```

---

# 155. Final Decision

The permanent Baobab dispute model SHALL be:

```text
PAYMENT
   │
   ▼
EXTERNAL CHALLENGE
   │
   ▼
CANONICAL DISPUTE
   │
   ├── reason
   ├── amount
   ├── deadline
   ├── financial exposure
   ├── provider references
   └── evidence requirements
            │
            ▼
     EVIDENCE PACKAGE
            │
            ▼
   AUTHORISED RESPONSE
            │
            ▼
 PROVIDER / PAYMENT RAIL
            │
            ▼
 AUTHORITATIVE OUTCOME
            │
       ┌────┴─────┐
       ▼          ▼
      WON        LOST
       │          │
       └────┬─────┘
            ▼
 FINANCIAL CONSEQUENCE
            │
            ▼
 SETTLEMENT / RECONCILIATION
            │
            ▼
           ERP
```

The permanent semantic boundaries are:

```text
PAYMENT
    ≠
DISPUTE

DISPUTE
    ≠
CHARGEBACK FINANCIAL MOVEMENT

CHARGEBACK
    ≠
REFUND

DISPUTE OPENED
    ≠
DISPUTE LOST

EVIDENCE SUBMITTED
    ≠
DISPUTE WON

SETTLEMENT DEBIT
    ≠
FINAL DISPUTE OUTCOME

BUSINESS EVIDENCE
    ≠
PAYMENTS-OWNED MASTER DATA

PROVIDER DISPUTE ID
    ≠
BAOBAB DISPUTE ID
```

**The provider or payment rail originates and adjudicates the external dispute.  
Baobab Payments owns the canonical operational representation of that dispute.  
Authoritative business domains own the facts used as evidence.  
Payments assembles, governs and submits the evidence without assuming ownership of those facts.  
The external payment process determines the dispute outcome.  
Payments records the resulting financial facts.  
ERP determines their accounting consequence.**

**A dispute never rewrites history merely because the financial outcome is contested. Baobab preserves the original Payment, the challenge, the evidence, every material financial movement, and the final externally established outcome as separate, auditable facts.**