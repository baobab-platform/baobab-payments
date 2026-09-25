# ADR-PAY-0015 — Settlement & ERP Reconciliation

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Settlement / Financial Reconciliation |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0014; ADR-PAY-0022; ADR-SHARED-011; applicable Control Plane, ERP, IAM and Shared contracts |
| **Related** | ADR-PAY-0016 through ADR-PAY-0021 |
| **Primary Domains** | Settlement, provider statements, reconciliation, fees, adjustments, financial projections, ERP handoff |

---

# 1. Context

Payment execution and settlement are different financial processes.

A provider may report:

```text
Payment CAPTURED
```

at one point and transfer the resulting funds to the merchant later.

Between those events there may be:

```text
processing fees
provider fees
acquirer fees
network fees
taxes on fees
refunds
chargebacks
chargeback fees
reserves
holds
rolling reserves
FX conversions
rounding
adjustments
settlement delays
payout deductions
```

Consequently:

```text
CAPTURED
    ≠
SETTLED
```

and:

```text
SETTLED
    ≠
ERP RECONCILED
```

Baobab requires a canonical mechanism for observing provider settlement and projecting verified financial facts into ERP without making `baobab-payments` an accounting ledger.

---

# 2. Problem

A simplistic implementation might assume:

```text
Payment captured
      │
      ▼
Mark invoice paid
      │
      ▼
Provider deposit
      │
      ▼
Done
```

This fails when:

```text
Gross collections        100,000
Refunds                    -5,000
Chargebacks                -2,000
Provider fees              -3,000
Other adjustments            -500
                         --------
Net settlement             89,500
```

The bank receives:

```text
89,500
```

while individual commercial and accounting records may involve many separate transactions.

A single provider deposit therefore cannot replace transaction-level financial evidence.

---

# 3. Decision

`baobab-payments` SHALL own the canonical operational representation of:

```text
provider settlement observations
settlement batches
settlement line items
payment-to-settlement relationships
refund-to-settlement relationships
dispute/chargeback settlement effects
provider fees and adjustments
settlement reconciliation state
ERP projection state
```

ERP SHALL remain authoritative for:

```text
general ledger
accounts receivable
accounts payable
revenue recognition
bank accounting
financial periods
journal entries
financial statements
accounting policy
```

---

# 4. Governing Principle

> **Payments establishes what the payment ecosystem says happened. ERP determines what those facts mean in the books.**

Therefore:

```text
Provider
   │
   ▼
Settlement Evidence
   │
   ▼
Baobab Payments
   │
   ▼
Canonical Financial Facts
   │
   ▼
ERP
   │
   ▼
Accounting Consequences
```

---

# 5. Fundamental Authority Boundary

```text
PAYMENTS
owns operational payment truth

ERP
owns accounting truth
```

Neither substitutes for the other.

---

# 6. Explicit Non-Equivalences

Baobab SHALL preserve:

```text
Payment Capture
      ≠
Settlement

Settlement
      ≠
Bank Deposit

Bank Deposit
      ≠
ERP Reconciliation

Provider Settlement Report
      ≠
General Ledger

Provider Fee
      ≠
Accounting Entry

Refund
      ≠
Settlement Adjustment

Chargeback
      ≠
Settlement

Payment Status
      ≠
Invoice Status

Settlement Batch
      ≠
ERP Journal

HyperSwitch
      ≠
Ledger

Baobab Payments
      ≠
Ledger
```

---

# 7. Authority Matrix

| Concern | Authority |
|---|---|
| Payment execution | Payments |
| Refund execution | Payments |
| Dispute operational lifecycle | Payments |
| Provider settlement observation | Payments |
| Canonical settlement representation | Payments |
| Provider financial-reference mapping | Payments |
| Reconciliation against payment facts | Payments |
| ERP projection delivery | Payments integration boundary |
| Accounting treatment | ERP |
| General ledger | ERP |
| Financial period | ERP |
| Revenue recognition | ERP |
| Bank-account reconciliation | ERP |
| Tenant / Legal Entity / Market | Control Plane |
| Identity / mutation authority | IAM |
| Provider certification | PAY-0022 |

---

# 8. Settlement Definition

A Settlement represents provider-side financial evidence that one or more payment-related financial movements have been included in a provider settlement process.

It SHALL be distinct from Payment.

---

# 9. Settlement Aggregate

Conceptually:

```text
Settlement
│
├── settlement_id
├── tenant_id
├── organisation_id?
├── legal_entity_id
├── market_id?
├── engine_instance_id
├── provider
├── merchant_reference
├── external_settlement_reference
├── settlement_currency
├── gross_amount?
├── fee_amount?
├── adjustment_amount?
├── net_amount
├── settlement_period_start?
├── settlement_period_end?
├── expected_value_date?
├── actual_value_date?
├── status
├── reconciliation_status
├── source_type
├── source_reference
├── correlation_id
├── simulated
├── observed_at
├── created_at
└── revision
```

The exact canonical contract SHALL evolve through Shared contracts.

---

# 10. Settlement Identity

Every Settlement SHALL have a Baobab canonical identity:

```text
settlement_id
```

Provider settlement/batch identifiers remain external references.

---

# 11. Settlement Line

A Settlement MAY contain multiple financial line items.

Conceptually:

```text
Settlement
   │
   ├── SettlementLine
   ├── SettlementLine
   ├── SettlementLine
   └── SettlementLine
```

---

# 12. Settlement Line Model

Conceptually:

```text
SettlementLine
│
├── settlement_line_id
├── settlement_id
├── line_type
├── canonical_reference?
├── external_reference?
├── amount
├── currency
├── fee?
├── effective_date?
├── provider_metadata?
└── reconciliation_status
```

---

# 13. Settlement Line Types

Canonical line categories SHOULD be capable of representing:

```text
PAYMENT
REFUND
CHARGEBACK
CHARGEBACK_REVERSAL
DISPUTE_ADJUSTMENT
FEE
RESERVE
RESERVE_RELEASE
FX_ADJUSTMENT
PAYOUT_ADJUSTMENT
OTHER_ADJUSTMENT
```

Exact enumeration belongs in canonical contracts.

---

# 14. Gross Settlement

Gross settlement represents the total relevant financial activity before deductions and additions.

It SHALL not automatically equal captured Payment totals because provider timing and inclusion rules may differ.

---

# 15. Net Settlement

Conceptually:

```text
Net Settlement
=
Gross Eligible Credits
-
Refunds
-
Chargebacks
-
Fees
-
Holds
-
Reserves
+
Reversals
+
Reserve Releases
± Adjustments
```

The exact provider calculation MAY vary.

Baobab SHALL preserve provider evidence rather than force every provider into an inaccurate arithmetic template.

---

# 16. Settlement Currency

Settlement currency SHALL be explicit.

It SHALL NOT be inferred from:

```text
Market
Payment currency
merchant country
provider country
```

---

# 17. Transaction Currency Versus Settlement Currency

Baobab SHALL support:

```text
Transaction Currency
       ≠
Settlement Currency
```

Example:

```text
Customer pays: UGX

Provider settles: USD
```

where legally, commercially and technically configured.

PAY-0017 governs detailed FX semantics.

---

# 18. No Silent FX

Payments SHALL NOT silently interpret differences between transaction and settlement amounts as fees when FX may be involved.

---

# 19. Provider Settlement Evidence

Settlement evidence MAY arrive through:

```text
provider API
HyperSwitch
provider webhook
settlement report
statement file
reconciliation API
controlled import
```

depending on provider capabilities.

---

# 20. HyperSwitch Boundary

HyperSwitch MAY provide settlement-related operational data where supported.

HyperSwitch SHALL NOT become Baobab's accounting ledger.

---

# 21. Raw Provider Report

A raw provider settlement file is external evidence.

It is not itself the canonical Baobab settlement model.

---

# 22. Normalisation

Settlement ingestion SHALL follow:

```text
External Settlement Evidence
          │
          ▼
Validate
          │
          ▼
Parse
          │
          ▼
Normalise
          │
          ▼
Canonical Settlement
          │
          ▼
Match
          │
          ▼
Reconcile
```

---

# 23. Evidence Preservation

Baobab SHOULD preserve sufficient immutable source evidence to explain how a canonical Settlement was derived.

This MAY include:

```text
external report reference
object-storage reference
content hash
provider
report type
retrieval timestamp
provider batch ID
schema/version
```

---

# 24. Large Settlement Files

Large provider files SHALL NOT be stored directly in canonical relational rows by default.

Protected object/document storage SHOULD be used where appropriate.

---

# 25. Source Integrity

Where settlement files are downloaded or received, integrity SHOULD be verifiable through appropriate:

```text
hash
signature
transport security
provider authentication
```

where supported.

---

# 26. Idempotent Ingestion

The same provider settlement evidence SHALL NOT create duplicate canonical Settlements.

PAY-0010 applies.

---

# 27. Immutable Source

Once processed, the exact source evidence used for financial reconciliation SHOULD remain reproducible for the applicable retention period.

---

# 28. Settlement State

Canonical Settlement lifecycle SHOULD support semantics such as:

```text
OBSERVED
INGESTING
INGESTED
MATCHING
PARTIALLY_MATCHED
MATCHED
RECONCILING
RECONCILED
EXCEPTION
SUPERSEDED
```

Exact enumeration may evolve.

---

# 29. Settlement State Versus Accounting State

`RECONCILED` in Payments means the settlement evidence has been reconciled against the operational financial facts according to Payments rules.

It SHALL NOT mean:

```text
ERP period closed
```

or:

```text
financial statements final
```

---

# 30. Reconciliation

Reconciliation is the process of comparing independently derived financial facts.

Baobab SHALL NOT call simple data copying reconciliation.

---

# 31. Three-Way Reconciliation

Where data is available, Baobab SHOULD support:

```text
CANONICAL PAYMENT FACTS
          │
          │
          ▼
PROVIDER SETTLEMENT FACTS
          │
          │
          ▼
ERP / BANK ACCOUNTING PROJECTION
```

The responsibilities remain separated.

---

# 32. Payments-Side Reconciliation

Payments SHALL reconcile:

```text
Payment
Refund
Dispute
Chargeback
Reversal
Provider Fee
Settlement
```

against provider settlement evidence.

---

# 33. ERP-Side Reconciliation

ERP SHALL reconcile accounting projections against:

```text
bank
cash
receivable
clearing
fee
tax
revenue
other accounting accounts
```

according to accounting policy.

---

# 34. Clearing Model

The integration SHOULD support an accounting clearing pattern.

Conceptually:

```text
Payment captured
      │
      ▼
Payment clearing position
      │
      ▼
Provider settlement
      │
      ▼
Bank receipt
      │
      ▼
Clear provider receivable
```

Exact accounts and journal rules belong to ERP configuration.

---

# 35. Payments Does Not Define Chart of Accounts

Payments SHALL NOT contain logic such as:

```text
provider_fee_account = 6100
bank_account = 1100
```

Chart-of-account configuration belongs to ERP.

---

# 36. Payments Does Not Post Arbitrary GL

`baobab-payments` SHALL NOT become a generic journal-entry engine.

---

# 37. Canonical Financial Facts

Payments SHALL emit facts such as:

```text
payment captured
refund succeeded
chargeback observed
chargeback reversed
provider fee observed
settlement observed
settlement reconciled
```

ERP determines entries.

---

# 38. ERP Integration Contract

Payments → ERP SHALL use a versioned integration contract.

It SHALL NOT depend on direct iDempiere database writes.

---

# 39. No Shared Database

Payments and ERP SHALL remain independently deployable and own separate persistence.

---

# 40. ERP Projection

A financial fact projected into ERP SHOULD include enough context to establish:

```text
Tenant
Legal Entity
Market where relevant
canonical transaction ID
source type
source reference
amount
currency
provider
merchant
settlement reference where relevant
effective timestamp
correlation
```

without exposing payment credentials.

---

# 41. Canonical IDs

ERP SHOULD retain canonical Baobab payment/refund/dispute/settlement IDs as external references for reconciliation.

---

# 42. ERP IDs

ERP document IDs SHALL remain ERP-owned external references from Payments' perspective.

---

# 43. Payment-to-ERP Mapping

Conceptually:

```text
Payment
   │
   ▼
Canonical Financial Event
   │
   ▼
ERP Integration
   │
   ▼
ERP Document / Accounting Entry
```

The ERP object does not become the canonical Payment.

---

# 44. Settlement-to-ERP Mapping

```text
Settlement
   │
   ▼
Settlement Financial Facts
   │
   ▼
ERP
   │
   ▼
Accounting / Bank Reconciliation
```

---

# 45. Payment Captured Projection

A captured Payment MAY cause an ERP financial projection before provider settlement.

The exact accounting treatment belongs to ERP.

---

# 46. Settlement Projection

Settlement later provides additional evidence about:

```text
cash movement
fees
adjustments
provider clearing
```

---

# 47. Projection Idempotency

The same canonical financial fact SHALL NOT create duplicate ERP financial consequences because of retries.

---

# 48. ERP Consumer Idempotency

ERP integration SHALL consume canonical financial facts idempotently.

---

# 49. Projection Identity

Each projected financial fact SHALL have a stable identity suitable for duplicate detection.

---

# 50. Projection State

Payments MAY track:

```text
NOT_PROJECTED
PENDING
PROJECTED
ACKNOWLEDGED
FAILED
RETRYING
```

or equivalent integration state.

This state is not accounting state.

---

# 51. ERP Unavailable

If ERP is unavailable:

```text
Payment fact
    │
    ▼
Durable outbox / integration queue
    │
    ▼
retry later
```

Payment execution SHALL not be rewritten as failed.

---

# 52. Payment Success Does Not Depend on ERP Availability

A provider-confirmed Payment can remain:

```text
CAPTURED
```

even if ERP projection is temporarily unavailable.

---

# 53. Settlement Success Does Not Depend on ERP Availability

A valid provider Settlement remains observed even if ERP cannot yet consume it.

---

# 54. Eventual Consistency

Payments and ERP SHALL converge through:

```text
durable events
idempotent consumers
retries
reconciliation
```

rather than distributed transactions.

---

# 55. No Distributed ACID

Baobab SHALL NOT attempt a distributed ACID transaction across:

```text
HyperSwitch
Provider
Payments
ERP
Bank
```

---

# 56. Matching

Settlement matching SHALL attempt to associate provider lines with canonical financial objects.

Possible keys include:

```text
provider payment ID
provider refund ID
provider dispute ID
merchant reference
canonical reference propagated externally
amount
currency
date
provider batch
```

according to provider capability.

---

# 57. Strong Identifiers Preferred

Exact provider/canonical references SHALL be preferred over fuzzy matching.

---

# 58. Fuzzy Matching

Amount/date/reference heuristics MAY assist reconciliation but SHALL NOT silently establish authoritative identity where ambiguity remains.

---

# 59. Ambiguous Match

An ambiguous settlement line SHALL enter:

```text
EXCEPTION
```

or equivalent review state.

---

# 60. Unmatched Settlement Line

A provider settlement line without a known canonical transaction SHALL not be discarded.

It SHALL enter reconciliation.

---

# 61. Missing Settlement

A captured Payment expected to settle but absent from expected provider settlement evidence SHOULD become a reconciliation exception according to provider timing policy.

---

# 62. Timing Policy

Expected settlement timing MAY vary by:

```text
provider
merchant
Legal Entity
market
currency
payment method
transaction type
business day
```

It SHALL be configuration-driven.

---

# 63. No Hardcoded T+N Globally

Baobab SHALL NOT assume all providers settle at:

```text
T+1
```

or any other universal interval.

---

# 64. Settlement Window

Provider certification SHOULD capture applicable settlement timing and reporting characteristics.

---

# 65. Tolerance

Reconciliation MAY require configured tolerances for legitimate:

```text
rounding
FX
provider calculation
minor-unit conversion
```

---

# 66. Tolerance Is Not Permission to Hide Differences

A tolerance SHALL be:

```text
explicit
bounded
currency-aware
provider-aware
auditable
```

---

# 67. No Universal Monetary Tolerance

Baobab SHALL NOT globally define:

```text
difference < 1.00 = ignore
```

across all currencies and markets.

---

# 68. Exact Money

All settlement arithmetic SHALL use exact monetary representation.

Binary floating point SHALL NOT be used for authoritative financial arithmetic.

---

# 69. Currency Precision

Currency-specific precision SHALL be respected.

Baobab SHALL not assume all currencies have two decimal places.

---

# 70. Fee

A provider fee SHALL be represented as a financial fact where relevant.

It SHALL NOT silently reduce the original Payment amount.

---

# 71. Payment Amount Integrity

If a customer paid:

```text
ZAR 1,000
```

and the provider retained:

```text
ZAR 30 fee
```

the canonical Payment remains:

```text
ZAR 1,000
```

The fee is a separate financial fact.

---

# 72. Net Deposit

The resulting:

```text
ZAR 970
```

deposit does not mean the customer paid ZAR 970.

---

# 73. Fee Categories

Payments MAY normalise fee categories such as:

```text
PROCESSING_FEE
REFUND_FEE
CHARGEBACK_FEE
FX_FEE
PAYOUT_FEE
OTHER_PROVIDER_FEE
```

while preserving provider-specific details.

---

# 74. Tax on Fees

Where provider reports tax on fees separately, Baobab SHOULD preserve that distinction.

ERP determines tax/accounting treatment.

---

# 75. Reserve

A provider reserve SHALL be distinct from a fee.

```text
Reserve
   ≠
Expense
```

Payments reports the provider financial fact.

ERP determines treatment.

---

# 76. Reserve Release

Release of previously withheld reserve SHALL be separately observable.

---

# 77. Hold

Temporary settlement holds SHALL not be treated as permanent fees.

---

# 78. Adjustment

Provider adjustments SHALL carry sufficient classification and source reference to support accounting review.

---

# 79. Unknown Adjustment

Unknown adjustments SHALL enter reconciliation exception rather than being silently classified as fee.

---

# 80. Refund Settlement

A successful Refund may appear as:

```text
negative settlement line
```

or another provider-specific representation.

Payments SHALL map it to the canonical Refund.

---

# 81. Refund API Success Versus Settlement

```text
Refund SUCCEEDED
       ≠
Refund settlement reconciled
```

Both facts may matter.

---

# 82. Chargeback Settlement

Chargeback principal and chargeback fee SHALL remain distinguishable where provider evidence allows.

---

# 83. Representment Recovery

Funds returned following a successful dispute representment SHALL be distinguishable from:

```text
new Payment
```

---

# 84. Reversal

Provider reversal settlement effects SHALL map to the corresponding canonical financial operation rather than becoming anonymous adjustments where possible.

---

# 85. Duplicate Settlement Line

Duplicate provider lines SHALL not create duplicate canonical financial consequences.

---

# 86. Corrected Provider Statement

Providers may issue corrected settlement reports.

Baobab SHALL preserve:

```text
original evidence
correction
supersession relationship
```

rather than silently replacing history.

---

# 87. Supersession

A corrected Settlement MAY supersede an earlier interpretation while retaining audit history.

---

# 88. Reconciliation Status

At line level, useful conceptual states include:

```text
UNMATCHED
MATCHED
RECONCILED
EXCEPTION
IGNORED_WITH_REASON
```

---

# 89. Ignore With Reason

A line SHALL not disappear from reconciliation merely because it is operationally irrelevant.

Where excluded, the reason SHOULD be recorded.

---

# 90. Reconciliation Exception

Conceptually:

```text
ReconciliationException
│
├── exception_id
├── settlement_id
├── settlement_line_id?
├── canonical_reference?
├── type
├── expected
├── observed
├── difference
├── currency
├── severity
├── status
├── resolution
├── resolved_by?
├── resolved_at?
└── audit
```

---

# 91. Exception Categories

Representative categories MAY include:

```text
UNMATCHED_PROVIDER_TRANSACTION
MISSING_SETTLEMENT
AMOUNT_MISMATCH
CURRENCY_MISMATCH
DUPLICATE
UNKNOWN_FEE
UNKNOWN_ADJUSTMENT
REFUND_MISMATCH
CHARGEBACK_MISMATCH
MERCHANT_MISMATCH
LEGAL_ENTITY_MISMATCH
ERP_PROJECTION_MISSING
```

---

# 92. Legal Entity Mismatch

A settlement line associated with the wrong Legal Entity is a high-severity integrity issue.

It SHALL NOT be automatically reconciled merely because the amount matches.

---

# 93. Merchant Mismatch

Historical merchant identity SHALL be validated.

Current merchant configuration SHALL not overwrite historical attribution.

---

# 94. Tenant Isolation

Settlement data SHALL remain tenant-scoped.

---

# 95. Legal Entity Isolation

ZuriBeans settlement SHALL remain attributable to ZuriBeans.

Thamani settlement SHALL remain attributable to Thamani.

Parent-group ownership does not merge settlement ownership.

---

# 96. Shared Provider Account

If a provider contract lawfully uses a shared operational account, Baobab SHALL still preserve Legal Entity attribution where the underlying business structure requires it.

Such arrangements require explicit governance.

---

# 97. External Tenants

External Baobab customers receive identical isolation guarantees.

---

# 98. Bank Account

Settlement destination bank accounts SHALL be associated with governed Legal Entity/provider configuration.

---

# 99. Bank Account Authority

A settlement bank account does not define the canonical Legal Entity.

Canonical context determines ownership; configuration must agree.

---

# 100. Bank Statement

ERP or treasury/accounting integration may ingest bank statements.

Payments SHOULD not become the authoritative bank-reconciliation engine unless explicitly expanded by future architecture.

---

# 101. Provider Settlement Versus Bank Receipt

Baobab SHALL permit:

```text
Provider says SETTLED
        │
        ▼
Bank receipt not yet observed
```

and:

```text
Bank receipt observed
        │
        ▼
Provider detail not yet reconciled
```

These are different evidence streams.

---

# 102. ERP Bank Reconciliation

ERP remains authoritative for bank-account reconciliation.

---

# 103. Settlement Date

Baobab SHALL distinguish where available:

```text
provider processing date
settlement period
settlement date
expected value date
actual bank value date
```

---

# 104. Financial Period

Payments SHALL NOT determine the ERP accounting period from provider settlement date alone.

---

# 105. Closed ERP Period

If a late settlement adjustment affects a closed ERP period, ERP determines the accounting treatment.

Payments preserves the actual event/effective dates.

---

# 106. Backdating

Payments SHALL NOT falsify event timestamps to fit an accounting period.

---

# 107. Late Settlement

Late provider settlement evidence SHALL be processed according to its actual observation/effective timestamps.

---

# 108. Historical Reconciliation

Reconciliation SHALL remain possible after:

```text
provider deactivation
merchant migration
market change
Legal Entity configuration change
```

---

# 109. Provider Deactivation

PAY-0022 deactivation for new transactions SHALL preserve required settlement/reconciliation access.

---

# 110. Provider Offboarding

A provider SHALL not be considered fully operationally offboarded until relevant:

```text
payments
refunds
disputes
settlements
reserves
adjustments
```

have reached an acceptable terminal/reconciled condition or governed migration/retention arrangement.

---

# 111. Long-Tail Settlement

Providers with long reserve/dispute windows may require historical access well beyond new-payment deactivation.

---

# 112. Settlement Credentials

Settlement-report credentials SHALL be managed under PAY-0012 secret controls.

---

# 113. Least Privilege

A workload allowed to execute payments SHALL not automatically receive permission to:

```text
download settlement reports
resolve reconciliation exceptions
alter settlement mappings
```

---

# 114. IAM

PAY-0018 SHALL govern privileged settlement/reconciliation mutations.

---

# 115. Suggested Capability Separation

Future canonical capabilities MAY distinguish:

```text
payment.settlement.read
payment.settlement.ingest
payment.reconciliation.read
payment.reconciliation.resolve
```

Exact naming belongs to Shared/IAM contracts.

---

# 116. Manual Resolution

Reconciliation exceptions MAY require human resolution.

Manual resolution SHALL be:

```text
authenticated
authorised
reasoned
audited
```

---

# 117. No Direct Database Correction

Operators SHALL NOT resolve settlement discrepancies through ad hoc SQL updates.

---

# 118. Adjustment Workflow

If a canonical correction is required, it SHALL occur through controlled application/domain operations.

---

# 119. Separation of Duties

High-value reconciliation adjustments MAY require approval separate from the initiating operator.

---

# 120. Audit

Settlement/reconciliation audit SHOULD preserve:

```text
settlement_id
provider reference
merchant
Legal Entity
source evidence
source hash
line identity
canonical match
amount
currency
difference
tolerance
operator/workload
resolution
timestamps
revision
correlation
```

---

# 121. Observability

The operational chain SHOULD be traceable:

```text
Payment
   │
   ▼
Provider Transaction
   │
   ▼
Settlement Line
   │
   ▼
Settlement
   │
   ▼
ERP Projection
   │
   ▼
ERP External Reference
```

---

# 122. Metrics

Representative metrics MAY include:

```text
settlements_ingested_total
settlement_lines_total
settlement_amount_total
settlement_unmatched_lines_total
settlement_exceptions_total
settlement_reconciliation_duration
missing_settlement_total
fee_amount_total
chargeback_settlement_total
refund_settlement_total
erp_projection_failure_total
erp_projection_lag
reconciliation_backlog
```

Exact names remain implementation-specific.

---

# 123. Alerting

Alerts SHOULD cover:

```text
settlement file missing
settlement ingestion failure
high unmatched-line count
Legal Entity mismatch
merchant mismatch
large amount variance
unexpected fee spike
unknown adjustment
ERP projection backlog
reconciliation backlog
provider settlement delay
```

---

# 124. No Sensitive Credentials in Observability

Settlement telemetry SHALL follow PAY-0012.

---

# 125. Reconciliation Dashboard

Operational tooling SHOULD allow authorised operators to see:

```text
expected
observed
difference
match
exception
source evidence
ERP projection status
```

without exposing sensitive payment credentials.

---

# 126. Automated Reconciliation

Deterministic, high-confidence matches SHOULD be automated.

---

# 127. Human Review

Ambiguous or policy-sensitive matches SHALL enter human review rather than being forced automatically.

---

# 128. Confidence Is Not Authority

A probabilistic matching score MAY assist operations.

It SHALL NOT override canonical identity or financial controls.

---

# 129. Reconciliation Rule Versioning

Matching/tolerance rules SHOULD be versioned.

A future operator SHOULD be able to understand which rule reconciled a historical line.

---

# 130. Configuration Snapshot

Settlement processing SHOULD preserve the relevant:

```text
provider configuration revision
merchant mapping revision
currency configuration
reconciliation-rule revision
```

where needed for reconstruction.

---

# 131. Drift Detection

Baobab SHOULD detect drift such as:

```text
provider settlement merchant
        ≠
mapped Baobab merchant

settlement currency
        ≠
configured settlement currency

provider account
        ≠
expected Legal Entity
```

---

# 132. Drift Is Not Automatically Corrected

Financial configuration drift SHALL fail safely or enter exception handling.

---

# 133. Settlement Readiness

A provider is not fully production-ready merely because it can authorise and capture payments.

Settlement/reconciliation capability is part of production readiness.

---

# 134. PAY-0022 Certification

Provider certification SHOULD evaluate:

```text
settlement reporting
transaction identifiers
refund representation
chargeback representation
fee transparency
settlement timing
currency behavior
report retrieval
correction handling
reconciliation support
```

---

# 135. Provider Without Detailed Settlement Data

If a provider cannot supply sufficient settlement evidence, the limitation SHALL be explicitly documented and assessed before activation.

---

# 136. Sandbox

Sandbox settlement records SHALL be marked:

```text
simulated = true
```

where generated by Baobab simulation.

---

# 137. Provider Sandbox

Real provider sandbox settlement data MAY test integration behavior but does not prove production settlement readiness.

---

# 138. Production Separation

Sandbox and production settlement records, credentials and provider accounts SHALL remain separated.

---

# 139. Testing

Settlement testing SHALL include at minimum:

```text
single successful Payment
multiple Payments in one settlement
partial Refund
multiple Refunds
chargeback
chargeback fee
chargeback reversal
provider fee
reserve
reserve release
unknown adjustment
duplicate line
missing Payment
missing Settlement
currency mismatch
amount mismatch
merchant mismatch
Legal Entity mismatch
corrected report
duplicate report ingestion
ERP unavailable
ERP duplicate delivery
late settlement
provider deactivation
```

---

# 140. Example — Simple Settlement

```text
Payment P1
Customer pays ZAR 1,000
        │
        ▼
CAPTURED
        │
        ▼
Provider fee ZAR 30
        │
        ▼
Settlement S1

Payment line       +1,000
Fee line              -30
                   ------
Net settlement       970
        │
        ▼
ERP

Payment fact         1,000
Fee fact                30
Settlement receipt     970
```

Payments does not convert the Payment into ZAR 970.

---

# 141. Example — Settlement with Refund

```text
Payment P1                 +1,000
Payment P2                 +2,000
Refund R1                    -300
Provider fees                 -90
                           -------
Net settlement              2,610
```

Each financial component remains separately traceable.

---

# 142. Example — Chargeback

```text
Original Payment
+1,000

Later settlement:

Chargeback principal
-1,000

Chargeback fee
-150

Net settlement impact
-1,150
```

The original Payment remains historically captured.

---

# 143. Example — Representment Recovery

```text
Settlement S1
Chargeback        -1,000

Dispute WON

Settlement S2
Recovery          +1,000
```

The recovery SHALL not be modelled as a new customer Payment.

---

# 144. Example — FX Settlement

```text
Customer Payment
UGX 500,000
      │
      ▼
Provider conversion
      │
      ▼
Settlement
USD ...
```

Baobab preserves:

```text
transaction amount
transaction currency
settlement amount
settlement currency
FX evidence
fees
```

where available.

PAY-0017 defines detailed FX semantics.

---

# 145. Example — ERP Outage

```text
Provider
   │
   ▼
Settlement S1
   │
   ▼
Payments RECONCILED
   │
   ▼
ERP projection
   │
   X
ERP unavailable
   │
   ▼
Durable retry
   │
   ▼
ERP recovers
   │
   ▼
Projection succeeds
```

Settlement truth is not rolled back.

---

# 146. Example — Unknown Provider Line

```text
Provider Settlement
       │
       ▼
Line X = ZAR 4,500
       │
       ▼
No canonical match
       │
       ▼
UNMATCHED
       │
       ▼
Reconciliation Exception
       │
       ▼
Authorised investigation
```

The line is not silently discarded or fabricated into a Payment.

---

# 147. End-to-End Settlement Flow

```text
Customer Payment
      │
      ▼
Baobab Payments
      │
      ▼
HyperSwitch
      │
      ▼
Provider
      │
      ▼
CAPTURED
      │
      ▼
Canonical Payment Fact
      │
      ├──────────────► ERP initial projection
      │
      ▼
Provider Settlement Process
      │
      ▼
Settlement Evidence
      │
      ▼
Baobab Payments
      │
      ▼
Canonical Settlement
      │
      ▼
Transaction Matching
      │
      ▼
Reconciliation
      │
 ┌────┴────────┐
 ▼             ▼
MATCH        EXCEPTION
 │             │
 ▼             ▼
ERP         Review
Projection     │
 │             ▼
 ▼          Resolution
ERP             │
 │              ▼
 ▼             ERP
Accounting
```

---

# 148. Multi-Legal-Entity Model

```text
                 BAOBAB PLATFORM
                       │
          ┌────────────┴────────────┐
          │                         │
          ▼                         ▼
      ZURIBEANS                  THAMANI
   Legal Entity A            Legal Entity B
          │                         │
          ▼                         ▼
 Merchant / Provider         Merchant / Provider
          │                         │
          ▼                         ▼
    Settlement A              Settlement B
          │                         │
          ▼                         ▼
   ERP attribution           ERP attribution
```

Even where infrastructure or provider is shared:

```text
Settlement ownership
       remains
Legal-Entity explicit
```

---

# 149. Reconciliation Architecture

```text
                PAYMENT FACTS
                     │
      ┌──────────────┼──────────────┐
      ▼              ▼              ▼
   Payments        Refunds       Disputes
      │              │              │
      └──────────────┼──────────────┘
                     ▼
              Expected Position
                     │
                     ▼
              PROVIDER EVIDENCE
                     │
                     ▼
               Settlement Lines
                     │
                     ▼
              Matching Engine
                     │
          ┌──────────┴──────────┐
          ▼                     ▼
        MATCH                EXCEPTION
          │                     │
          ▼                     ▼
      Reconciled             Review
          │                     │
          └──────────┬──────────┘
                     ▼
                ERP Projection
                     │
                     ▼
                    ERP
                     │
                     ▼
           Accounting / Bank
             Reconciliation
```

---

# 150. Production Readiness Gate

```text
SETTLEMENT & ERP RECONCILIATION

Canonical Settlement model                    PASS
Settlement line model                         PASS
Provider settlement ingestion                 PASS
Source evidence retention                     PASS
Integrity validation                          PASS
Idempotent ingestion                          PASS
Payment matching                              PASS
Refund matching                               PASS
Chargeback matching                           PASS
Fee representation                            PASS
Reserve representation                        PASS
Adjustment representation                     PASS
Settlement currency                           PASS
FX distinction                                PASS
Exact monetary arithmetic                     PASS
Tolerance policy                              PASS
Exception model                               PASS
Manual resolution workflow                    PASS
Legal Entity isolation                        PASS
Tenant isolation                              PASS
Merchant validation                           PASS
Corrected-statement handling                  PASS
Provider deactivation handling                PASS
ERP integration contract                      PASS
ERP idempotency                               PASS
ERP outage recovery                           PASS
Transactional outbox                          PASS
No direct ERP DB writes                       PASS
No GL ownership in Payments                   PASS
Reconciliation-rule versioning                PASS
Drift detection                               PASS
Observability                                 PASS
Audit                                         PASS
Sandbox/prod isolation                        PASS
Production settlement test                    PASS
---------------------------------------------------
Settlement & Reconciliation Plane             READY
```

---

# 151. Invariants

The following invariants SHALL hold:

1. Payment capture is not Settlement.
2. Settlement is not ERP reconciliation.
3. Settlement is not a general ledger entry.
4. Bank deposit is not Settlement identity.
5. HyperSwitch is not the accounting ledger.
6. Payments is not the accounting ledger.
7. ERP owns accounting truth.
8. Payments owns canonical operational settlement truth.
9. Every Settlement has a Baobab canonical identity.
10. Provider settlement IDs remain external references.
11. Settlement lines remain traceable.
12. Transaction currency and settlement currency remain distinct.
13. Settlement currency is explicit.
14. No silent FX is permitted.
15. Payment amount is not reduced to net settlement amount.
16. Provider fees are separate financial facts.
17. Chargeback fees are separate from chargeback principal.
18. Reserves are distinct from fees.
19. Holds are distinct from permanent deductions.
20. Refund settlement remains linked to canonical Refund.
21. Chargeback settlement remains linked to canonical Dispute/payment context.
22. Representment recovery is not a new Payment.
23. Provider adjustments are classified where possible.
24. Unknown adjustments enter exception handling.
25. Raw provider reports do not become canonical contracts.
26. Settlement ingestion is idempotent.
27. Duplicate lines do not create duplicate consequences.
28. Corrected reports do not silently erase original evidence.
29. Reconciliation compares independent facts.
30. Copying provider data into ERP is not reconciliation.
31. Exact identifiers are preferred over fuzzy matching.
32. Ambiguous matches are not silently accepted.
33. Unmatched provider lines are preserved.
34. Missing expected settlement can generate an exception.
35. Settlement timing is provider/configuration-specific.
36. No universal T+N assumption exists.
37. Tolerances are explicit and bounded.
38. No universal monetary tolerance exists.
39. Financial arithmetic uses exact decimal/minor-unit semantics.
40. Currency precision is respected.
41. Tenant settlement data is isolated.
42. Legal Entity settlement ownership is explicit.
43. Group ownership does not merge settlement ownership.
44. External tenants receive equivalent isolation.
45. Settlement bank account does not define canonical Legal Entity.
46. Historical merchant context is preserved.
47. Current configuration does not rewrite historical settlement attribution.
48. Provider deactivation preserves required historical reconciliation.
49. Provider offboarding accounts for outstanding settlements/reserves/disputes.
50. Payment execution does not depend on ERP availability.
51. Settlement observation does not depend on ERP availability.
52. ERP integration is eventually consistent.
53. ERP consumption is idempotent.
54. Payments does not write directly to ERP database tables.
55. Payments does not define the chart of accounts.
56. Payments does not determine accounting periods.
57. Payments does not fabricate timestamps for accounting convenience.
58. ERP IDs remain external to Payments.
59. Canonical Baobab IDs remain externally referenceable from ERP.
60. Reconciliation exceptions are auditable.
61. Manual resolution requires authorization.
62. Direct SQL correction is prohibited.
63. High-risk financial corrections may require separation of duties.
64. Reconciliation rules are versionable.
65. Drift is detected rather than silently corrected.
66. Settlement credentials follow PAY-0012.
67. Settlement operations follow PAY-0018 authorization.
68. Settlement events follow PAY-0011 delivery semantics.
69. Sandbox settlement data remains separate from production.
70. Provider sandbox settlement success does not prove production readiness.

---

# 152. Consequences

## Positive

This decision provides:

- clear payment/accounting separation;
- provider-independent settlement modelling;
- robust ERP integration;
- Legal Entity attribution;
- fee transparency;
- chargeback reconciliation;
- refund reconciliation;
- FX-ready settlement modelling;
- provider-offboarding safety;
- correction handling;
- operational exception management;
- auditable financial lineage;
- resilient ERP outage handling.

## Negative

It requires:

- settlement ingestion infrastructure;
- provider-specific parsers/adapters;
- canonical settlement storage;
- matching logic;
- exception workflows;
- historical source retention;
- ERP integration;
- long-lived reconciliation processes;
- operational dashboards;
- accounting-domain coordination.

These costs are accepted because financial correctness cannot be inferred reliably from Payment status alone.

---

# 153. Alternatives Considered

## Treat CAPTURED as SETTLED

Rejected.

Capture and settlement are distinct provider processes.

---

## Let HyperSwitch be the ledger

Rejected.

HyperSwitch is payment orchestration infrastructure, not Baobab's accounting authority.

---

## Let Payments maintain the general ledger

Rejected.

That duplicates ERP authority.

---

## Send only net settlement to ERP

Rejected.

This destroys traceability of:

```text
payments
refunds
fees
chargebacks
adjustments
```

---

## Let ERP query HyperSwitch directly

Rejected.

It bypasses canonical Payments semantics and creates provider coupling.

---

## Directly write iDempiere tables

Rejected.

It violates service ownership and independent deployment.

---

## Assume every provider settles T+1

Rejected.

Settlement schedules vary.

---

## Ignore small differences automatically

Rejected.

Tolerance requires explicit currency/provider-aware policy.

---

## Treat every unmatched line as provider fee

Rejected.

Unknown financial movements require investigation.

---

## Rewrite old settlement records after corrected reports

Rejected.

Financial evidence requires historical integrity.

---

# 154. Relationship to PAY-0008

PAY-0008 defines:

```text
Payment lifecycle
```

PAY-0015 establishes that:

```text
CAPTURED
   │
   ▼
Settlement lifecycle begins independently
```

Payment state does not become `SETTLED` merely to avoid modelling Settlement properly.

---

# 155. Relationship to PAY-0010

Settlement ingestion, ERP projection and reconciliation commands SHALL be idempotent.

Duplicate delivery SHALL not duplicate:

```text
Settlement
fee
adjustment
ERP projection
```

---

# 156. Relationship to PAY-0011

Settlement facts published as events SHALL follow:

```text
transactional outbox
at-least-once delivery
consumer idempotency
schema versioning
correlation
```

---

# 157. Relationship to PAY-0012

Settlement reports may contain commercially or personally sensitive information.

They SHALL follow:

```text
data minimisation
access control
secure storage
secret management
retention
```

PAY-0012 remains authoritative for payment-sensitive data.

---

# 158. Relationship to PAY-0013

Refund execution and Refund settlement remain separate:

```text
Refund SUCCEEDED
      │
      ▼
Provider Settlement
      │
      ▼
Reconciliation
```

---

# 159. Relationship to PAY-0014

Dispute case outcome and chargeback settlement remain distinct:

```text
Dispute
   │
   ├── case lifecycle
   │
   ▼
Chargeback / Recovery
   │
   ▼
Settlement
   │
   ▼
ERP
```

---

# 160. Relationship to PAY-0016

Payouts SHALL have their own execution semantics.

PAY-0015 will reconcile payout-related provider settlement/financial movements where applicable without treating merchant collection settlement as beneficiary payout.

---

# 161. Relationship to PAY-0017

PAY-0017 SHALL govern:

```text
FX source
FX rate
conversion timestamp
transaction currency
settlement currency
cross-border semantics
```

PAY-0015 preserves those facts for reconciliation.

---

# 162. Relationship to PAY-0018

PAY-0018 SHALL define detailed authorization for:

```text
settlement ingestion
settlement read
reconciliation read
exception resolution
manual adjustment
provider settlement configuration
```

---

# 163. Relationship to PAY-0019

PAY-0019 SHALL define SLOs and monitoring for:

```text
settlement ingestion latency
matching latency
reconciliation backlog
ERP projection lag
provider settlement delays
exception growth
```

---

# 164. Relationship to PAY-0020

Backup and recovery SHALL preserve:

```text
Settlement
SettlementLine
source evidence
canonical matches
exceptions
ERP projection state
audit history
```

without causing duplicate ERP consequences after recovery.

---

# 165. Relationship to PAY-0021

HyperSwitch upgrades SHALL be evaluated for changes affecting:

```text
settlement APIs
provider references
report formats
fee representation
refund references
dispute references
connector semantics
```

---

# 166. Relationship to PAY-0022

Provider certification SHALL not stop at:

```text
Can the provider collect money?
```

It must also establish, where applicable:

```text
Can Baobab determine what was settled?

Can individual financial movements be reconciled?

Can fees be identified?

Can refunds be matched?

Can chargebacks be matched?

Can settlement currency be identified?

Can corrections be processed?

Can the Legal Entity account for the resulting funds?
```

---

# 167. Canonical Financial Chain

The complete chain is:

```text
COMMERCIAL OBLIGATION
        │
        ▼
PAYMENT INTENT
        │
        ▼
PAYMENT
        │
        ▼
PAYMENT ATTEMPT
        │
        ▼
HYPERSWITCH / PROVIDER
        │
        ▼
CAPTURE
        │
        ▼
CANONICAL PAYMENT FACT
        │
        ▼
PROVIDER SETTLEMENT
        │
        ▼
CANONICAL SETTLEMENT
        │
        ▼
RECONCILIATION
        │
        ▼
ERP PROJECTION
        │
        ▼
ACCOUNTING
        │
        ▼
BANK RECONCILIATION
```

Each boundary has a different authority.

---

# 168. Final Decision

Baobab SHALL permanently distinguish:

```text
PAYMENT
       ≠
SETTLEMENT

CAPTURE
       ≠
SETTLEMENT

SETTLEMENT
       ≠
BANK RECEIPT

SETTLEMENT
       ≠
ERP JOURNAL

PROVIDER FEE
       ≠
PAYMENT AMOUNT

CHARGEBACK
       ≠
PAYMENT FAILURE

REPRESENTMENT RECOVERY
       ≠
NEW PAYMENT

PAYMENTS RECONCILIATION
       ≠
ERP ACCOUNTING

HYPERSWITCH
       ≠
LEDGER

BAOBAB PAYMENTS
       ≠
LEDGER
```

The authoritative flow is:

```text
Provider
   │
   ▼
External Financial Evidence
   │
   ▼
BAOBAB PAYMENTS
   │
   ├── normalises
   ├── correlates
   ├── matches
   ├── detects differences
   ├── preserves provider evidence
   └── establishes canonical operational facts
   │
   ▼
ERP
   │
   ├── determines accounting treatment
   ├── owns ledger entries
   ├── owns financial periods
   ├── owns accounting policy
   └── reconciles bank/cash
   │
   ▼
Accounting Truth
```

**The provider tells Baobab what the payment ecosystem settled.  
Baobab Payments determines which canonical payment, refund, chargeback, fee or adjustment that evidence belongs to.  
Reconciliation establishes whether independent operational facts agree.  
Payments reports those facts without inventing accounting meaning.  
ERP determines their accounting consequences and remains authoritative for the books.**

**A payment is not settled merely because it was captured. A provider settlement is not reconciled merely because a deposit arrived. And neither HyperSwitch nor `baobab-payments` becomes a ledger merely because it possesses financial data.**