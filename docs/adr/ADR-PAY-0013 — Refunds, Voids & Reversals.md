# ADR-PAY-0013 — Refunds, Voids & Reversals

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Payment Lifecycle / Financial Operations |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0012; ADR-PAY-0022; ADR-SHARED-011; applicable Control Plane, IAM, Trade, Subscription, ERP and Shared contracts |
| **Related** | ADR-PAY-0014; ADR-PAY-0015; ADR-PAY-0017; ADR-PAY-0018; ADR-PAY-0019; ADR-PAY-0020 |
| **Primary Domains** | Refunds, authorization cancellation, voids, reversals, partial refunds, asynchronous outcomes, reconciliation |

---

# 1. Context

A successful payment does not end the financial lifecycle.

After authorization or capture, Baobab may need to:

- cancel an authorization;
- void an unsettled transaction;
- release reserved funds;
- return captured funds;
- issue a partial refund;
- issue multiple partial refunds;
- respond to a provider-initiated reversal;
- recover from an ambiguous transaction;
- reconcile a provider-side reversal;
- process a business cancellation;
- process a subscription credit or cancellation;
- handle a payment subsequently reversed by the payment rail.

Different providers frequently use terms such as:

```text
void
cancel
reverse
refund
reversal
authorization reversal
refund reversal
```

in inconsistent ways.

Baobab therefore requires canonical semantics independent of provider terminology.

---

# 2. Problem

Without a canonical model, an implementation may incorrectly assume:

```text
Order cancelled
      =
Payment refunded
```

or:

```text
Payment voided
      =
Payment refunded
```

or:

```text
Provider reversal
      =
Customer refund
```

These equivalences are unsafe.

The appropriate financial operation depends on the lifecycle of the original payment.

---

# 3. Decision

Baobab SHALL model:

```text
CANCELLATION / VOID
```

and:

```text
REFUND
```

as distinct canonical operations.

Provider-specific:

```text
REVERSAL
```

terminology SHALL be translated into the appropriate Baobab canonical semantic based on what financially occurred.

A provider word alone SHALL NOT determine the canonical operation.

---

# 4. Governing Principle

> **Before capture, Baobab stops or releases money movement. After capture, Baobab returns money through a Refund. Provider terminology never overrides canonical financial semantics.**

Conceptually:

```text
              PAYMENT LIFECYCLE

CREATED
   │
   ▼
AUTHORIZED
   │
   ├──── Cancel / Void ───► authorization released
   │
   ▼
CAPTURED
   │
   └──── Refund ──────────► funds returned
```

The exact operation remains subject to provider and rail capabilities.

---

# 5. Domain Separation

Baobab SHALL distinguish at least:

```text
Payment cancellation
Authorization void/release
Refund
Provider reversal
Dispute/chargeback
Settlement adjustment
Accounting adjustment
Commercial cancellation
Credit
```

These are different concepts.

---

# 6. Explicit Non-Equivalences

The following SHALL NOT be treated as equivalent:

```text
Order Cancellation
        ≠
Payment Cancellation

Payment Cancellation
        ≠
Refund

Void
        ≠
Refund

Refund Requested
        ≠
Refund Completed

Refund
        ≠
Chargeback

Refund
        ≠
Accounting Credit Note

Provider Reversal
        ≠ automatically
Refund

Subscription Cancellation
        ≠
Refund

Invoice Credit
        ≠
Refund

Payment Captured
        ≠
Payment Settled
```

---

# 7. Authority Boundaries

| Question | Authority |
|---|---|
| Should an order be cancelled? | Trade |
| Should a subscription be cancelled? | Subscriptions |
| Is a commercial refund owed? | Originating business domain |
| What amount is commercially refundable? | Originating business domain |
| Can the requested financial operation be executed? | Payments |
| Which provider operation implements it? | Payments |
| Which provider route is applicable? | Payments |
| Did the refund/void actually succeed? | Payments |
| What does it mean for the order? | Trade |
| What does it mean for the subscription/invoice? | Subscriptions |
| What accounting entries are required? | ERP |
| What tenant/legal entity/market applies? | Control Plane |
| Who may request/approve the operation? | IAM + domain authorization policy |

---

# 8. Commercial Decision Versus Financial Execution

The originating business engine determines:

```text
WHY
```

money should be returned or released.

Payments determines:

```text
HOW
```

the authorised financial operation can be executed and:

```text
WHAT HAPPENED
```

during that execution.

---

# 9. Trade Example

Trade may determine:

```text
Order O123
cancelled by authorised business policy

Refundable amount:
ZAR 1,000
```

Trade then requests the appropriate payment operation.

It does not directly mutate the Payment to:

```text
REFUNDED
```

---

# 10. Subscription Example

Subscriptions may determine that:

```text
subscription cancelled
unused period credited
customer entitled to ZAR 250 refund
```

Subscriptions creates or identifies the relevant commercial refund obligation.

Payments executes the financial refund.

---

# 11. ERP Boundary

ERP SHALL NOT originate provider refund execution merely because an accounting credit exists.

Likewise:

```text
Credit Note
      ≠
Refund
```

A credit note may exist without money being returned immediately.

A Refund may produce accounting consequences that ERP records.

---

# 12. Cancellation

A canonical payment cancellation means that Baobab intends to prevent an incomplete payment operation from progressing where the lifecycle and provider permit it.

Cancellation MAY apply to states such as:

```text
CREATED
REQUIRES_ACTION
PROCESSING
```

depending on PAY-0008 and provider capabilities.

---

# 13. Cancellation Does Not Guarantee External Reversal

Cancelling a Baobab operation does not necessarily mean the external provider has successfully stopped money movement.

Therefore:

```text
cancel requested
      ≠
cancel confirmed
```

---

# 14. Authorization Void

Where funds have been authorised but not captured, Payments SHOULD attempt the provider/rail operation that releases or cancels the authorization when appropriate.

Conceptually:

```text
AUTHORIZED
    │
    ▼
VOID REQUESTED
    │
    ▼
Provider / HyperSwitch
    │
    ├── success ──► authorization released
    │
    ├── pending ──► await confirmation
    │
    └── failure ──► authorization remains
```

---

# 15. Void Is Not Refund

A void normally concerns money that has not reached the same captured financial state as a refundable payment.

Therefore:

```text
AUTHORIZED
   │
   ▼
VOID
```

is conceptually distinct from:

```text
CAPTURED
   │
   ▼
REFUND
```

---

# 16. Late Void

Providers may support voiding a captured-but-not-yet-settled transaction.

Where such capability exists, Payments MAY use it if the canonical financial semantics are correctly represented.

The provider word `void` SHALL not determine the canonical Baobab meaning by itself.

---

# 17. Canonical Outcome Over Provider Vocabulary

If Provider A calls an operation:

```text
reversal
```

and Provider B calls an economically equivalent operation:

```text
void
```

Baobab SHALL map both according to their actual effect.

---

# 18. Reversal

`Reversal` SHALL primarily be treated as an external/provider financial phenomenon rather than an ambiguous universal Baobab command.

Possible external meanings include:

```text
authorization reversal
payment reversal
refund reversal
bank reversal
settlement reversal
```

Payments SHALL classify the observation before applying canonical state.

---

# 19. Provider Reversal Translation

Conceptually:

```text
Provider Event
"transaction_reversed"
        │
        ▼
Determine original transaction state
        │
        ▼
Determine financial effect
        │
   ┌────┼─────────────┐
   ▼    ▼             ▼
Auth   Capture       Refund
release undone       undone
   │    │             │
   ▼    ▼             ▼
Canonical semantic translation
```

---

# 20. Refund

A Refund is a distinct canonical financial aggregate representing an attempt to return some or all previously captured funds.

It SHALL NOT be represented solely as:

```text
Payment.status = REFUNDED
```

---

# 21. Refund Aggregate

Conceptually:

```text
Refund
│
├── refund_id
├── payment_id
├── payment_intent_id?
├── source_engine
├── source_reference
├── tenant_id
├── organisation_id?
├── legal_entity_id
├── market_id?
├── amount
├── currency
├── reason?
├── status
├── provider_reference?
├── idempotency_reference
├── correlation_id
├── simulated
├── requested_at
├── completed_at?
└── revision
```

The exact schema belongs in versioned canonical contracts.

---

# 22. Refund Identity

Every Refund SHALL have its own canonical identity.

Therefore:

```text
Payment P1
   │
   ├── Refund R1
   ├── Refund R2
   └── Refund R3
```

is valid.

---

# 23. Multiple Refunds

A single Payment MAY have:

```text
zero
one
many
```

Refunds.

This is necessary for partial returns, multiple order-line returns, adjustments and staged refund processing.

---

# 24. Partial Refund

Baobab SHALL support partial refunds where the underlying provider/payment method supports them.

Example:

```text
Original captured amount
ZAR 1,000

Refund R1
ZAR 200

Refund R2
ZAR 300

Remaining refundable amount
ZAR 500
```

---

# 25. Refundable Amount

Payments SHALL enforce:

```text
Successful Refund Total
        ≤
Refundable Captured Amount
```

subject to provider semantics and previously reversed/refunded amounts.

---

# 26. Refund Amount Authority

The originating business domain determines the commercially requested refund amount.

Payments independently validates that the amount is financially executable.

---

# 27. No Over-Refund

Payments SHALL prevent:

```text
total successful refunds
>
financially refundable amount
```

unless a future explicitly approved payment product introduces a different financial construct.

---

# 28. Concurrent Refund Requests

Concurrent refund requests SHALL be protected against over-refunding.

Example:

```text
Remaining refundable = 500

Request A = 400
Request B = 400
```

Both SHALL NOT independently succeed for a total of 800.

---

# 29. Transactional Refund Reservation

Payments SHOULD use appropriate concurrency control to reserve or account for refund capacity before external execution.

Possible implementation techniques include:

```text
database locking
optimistic concurrency
refund allocation records
serializable domain operations
```

The implementation MAY vary.

The invariant may not.

---

# 30. Currency

Refund currency SHALL normally equal the original transaction currency.

A Refund SHALL NOT silently convert currencies.

PAY-0017 governs exceptional FX/cross-border semantics.

---

# 31. Refund and Settlement Currency

If:

```text
transaction currency
      ≠
settlement currency
```

the Refund remains canonically associated with the original payment's transaction currency unless an explicitly modelled payment-rail rule requires otherwise.

---

# 32. Refund Reason

A Refund MAY carry a canonical reason or reason category.

Examples:

```text
CUSTOMER_RETURN
ORDER_CANCELLATION
SERVICE_NOT_PROVIDED
DUPLICATE_PAYMENT
BILLING_ADJUSTMENT
GOODWILL
OTHER
```

Exact enumeration SHALL be governed through Shared contracts.

---

# 33. Reason Is Not Authority

Providing:

```text
reason = CUSTOMER_RETURN
```

does not establish permission to refund.

Authorization is evaluated independently.

---

# 34. Refund State Machine

A Refund SHALL have its own lifecycle.

Conceptually:

```text
REQUESTED
    │
    ▼
PROCESSING
    │
 ┌──┼───────────┐
 ▼  ▼           ▼
SUCCEEDED     FAILED
   │
   └──► provider-specific later developments
```

PAY-0008 principles apply to canonical state transitions.

---

# 35. Asynchronous Refunds

Refund execution MAY be asynchronous.

Therefore:

```text
refund API accepted
      ≠
refund succeeded
```

---

# 36. Refund Requested Event

Payments MAY emit:

```text
refund.requested
```

once the canonical Refund request has been accepted into the lifecycle.

This is not proof that funds have returned.

---

# 37. Refund Processing Event

Payments MAY emit:

```text
refund.processing
```

where the state is meaningful to consumers.

---

# 38. Refund Success Event

Only established successful financial execution SHALL produce:

```text
refund.succeeded
```

or the final canonical equivalent.

---

# 39. Refund Failure Event

A confirmed failed refund MAY produce:

```text
refund.failed
```

The failure SHALL not cause the Refund record to disappear.

---

# 40. Unknown Refund Outcome

Provider timeout does not establish failure.

Conceptually:

```text
Refund request
     │
     ▼
Provider timeout
     │
     ▼
UNKNOWN / PROCESSING
     │
     ▼
Recover state
```

Payments SHALL NOT blindly submit another refund.

---

# 41. Duplicate Refund Safety

PAY-0010 applies fully to refunds.

The same logical refund request repeated with the same idempotency identity SHALL NOT create another financial refund.

---

# 42. Distinct Refunds

A genuinely new partial refund requires a new logical refund identity/idempotency scope.

This distinguishes:

```text
network retry of R1
```

from:

```text
new Refund R2
```

---

# 43. Correlation

Refunds SHALL retain correlation to:

```text
original Payment
PaymentIntent
source business operation
order/invoice/subscription where applicable
provider payment
provider refund
```

without copying ownership of those domains.

---

# 44. Provider Reference

Provider refund IDs SHALL be retained as external references.

They SHALL NOT replace:

```text
refund_id
```

as Baobab canonical identity.

---

# 45. Refund Provider

A Refund SHOULD normally execute through the payment path capable of refunding the original Payment.

Payments SHALL NOT freely reroute a refund as though it were a new collection.

---

# 46. Refund Routing

Conceptually:

```text
Original Payment
      │
      ▼
Provider Execution Context
      │
      ▼
Refund
      │
      ▼
Compatible refund route
```

PAY-0009 routing does not imply arbitrary provider substitution for refunds.

---

# 47. Provider Migration

If the original provider is no longer active for new collections, Payments SHALL preserve sufficient historical capability to process legitimate:

```text
refunds
disputes
reconciliation
```

where contractually and technically possible.

---

# 48. Provider Deactivation

PAY-0022 provider deactivation SHALL distinguish:

```text
NO NEW PAYMENTS
```

from:

```text
NO HISTORICAL OPERATIONS
```

A provider may need to remain accessible for refunds even after new-payment routing is disabled.

---

# 49. Merchant Context

Refund execution SHALL preserve the original Legal Entity and merchant context unless an explicitly supported migration process establishes another lawful path.

---

# 50. Cross-Legal-Entity Refund Prohibited

A payment captured for ZuriBeans SHALL NOT be refunded through Thamani's merchant identity merely because both belong to the same corporate group.

---

# 51. External Tenant Isolation

A refund for one external tenant SHALL NOT execute under another tenant's provider/merchant context.

---

# 52. Market Context

The Refund SHALL retain the original relevant Market context.

A later change to market configuration SHALL not rewrite the historical Payment.

---

# 53. Digital Estate

A digital estate may request a commercial refund through its authoritative business engine.

The estate itself does not become payment authority.

---

# 54. Refund Credentials

A Refund SHALL NOT require retrieval of the customer's raw card credentials.

Preferred flow:

```text
Refund
   │
   ▼
Original Payment
   │
   ▼
Provider Payment Reference
   │
   ▼
Provider Refund Operation
```

PAY-0012 remains authoritative.

---

# 55. Refund Payment Method

The original payment-method context SHALL be preserved for refund execution and audit.

---

# 56. Refund to Original Method

Where payment-rail rules require refund to the original payment method, Payments SHALL enforce that rule.

A refund SHALL not be redirected to another beneficiary merely because a caller supplies new payment details.

---

# 57. Alternative Refund Destination

If a legitimate business model requires a different refund destination, that SHALL be explicitly modelled as a different controlled financial operation rather than silently altering standard Refund semantics.

---

# 58. Refund Versus Payout

Sending money to an arbitrary beneficiary is not a Refund.

It may be:

```text
Payout
Disbursement
Transfer
```

under PAY-0016.

---

# 59. Refund Versus Store Credit

Store credit is not necessarily a financial Refund.

Trade or the relevant commercial domain owns store-credit semantics.

---

# 60. Refund Versus Credit Note

An ERP credit note adjusts accounting/commercial receivable treatment.

It is not proof that money was returned.

---

# 61. Refund Versus Chargeback

A customer Refund is initiated through the merchant/payment workflow.

A Chargeback is an externally initiated dispute process.

PAY-0014 governs disputes and chargebacks.

---

# 62. Refund After Chargeback

Payments SHALL prevent unsafe duplicate return of funds when a payment is already subject to a successful chargeback or equivalent external reversal.

The interaction SHALL be evaluated using current financial exposure.

---

# 63. Chargeback During Refund

A dispute may arise while a Refund is processing.

Payments SHALL not assume the two processes are mutually exclusive.

Reconciliation SHALL detect financial overlap.

---

# 64. Commercial Cancellation Before Capture

If Trade cancels an order before payment capture:

```text
Trade
  │
  ▼
Order cancelled
  │
  ▼
Payment state evaluated
  │
  ├── no authorization ─► cancel local payment lifecycle
  │
  └── authorized ───────► void/release if appropriate
```

A Refund is unnecessary if no funds were captured.

---

# 65. Commercial Cancellation After Capture

If the payment has already been captured:

```text
Trade
  │
  ▼
Order cancelled
  │
  ▼
Refund obligation
  │
  ▼
Payments Refund
```

subject to commercial policy.

---

# 66. Commercial Cancellation During Unknown State

If Trade requests cancellation while payment outcome is ambiguous:

```text
PROCESSING / UNKNOWN
```

Payments SHALL first establish or safely coordinate the financial state.

It SHALL not blindly:

```text
cancel
+
refund
```

simultaneously.

---

# 67. Race: Capture Versus Void

A capture and void may race.

Conceptually:

```text
AUTHORIZED
   │
 ┌─┴─────────┐
 │           │
Capture     Void
 │           │
 └─────┬─────┘
       ▼
Provider outcome determines
actual financial result
```

Payments SHALL serialize or reconcile conflicting commands.

---

# 68. Race: Capture Completes During Cancellation

If capture completes after cancellation was requested, Payments SHALL recognise the resulting captured financial state.

The business domain may then require a Refund.

Payments SHALL not pretend the capture never happened.

---

# 69. Race: Refund Versus Another Refund

Concurrent refund commands SHALL observe a consistent refundable balance.

---

# 70. Race: Refund Versus Dispute

Refund/dispute overlap SHALL be reconciled against actual provider financial state before additional money movement is initiated.

---

# 71. Authorization Expiry

An authorization may expire naturally.

Natural expiry SHALL be distinguished from a successful explicit void where operationally relevant.

---

# 72. Provider Auto-Reversal

Some providers/rails may automatically reverse expired or failed authorizations.

Such external observations SHALL be canonicalised without manufacturing a merchant-requested Refund.

---

# 73. Provider-Initiated Reversal

A provider may reverse a previously successful transaction due to:

```text
technical correction
bank rejection
rail rule
settlement failure
risk action
```

Such reversal SHALL be treated as a distinct external financial fact.

---

# 74. Reversal Is Not Business Refund

If the provider reverses funds without a Baobab Refund request:

```text
Provider Reversal
      ≠
Customer Refund Requested by Trade
```

The commercial engines SHALL receive the canonical financial fact and decide the business consequence.

---

# 75. Reversal After Payment Success

Conceptually:

```text
CAPTURED
   │
   ▼
provider later reverses
   │
   ▼
Payments validates external fact
   │
   ▼
canonical reversal fact
   │
   ├──► Trade
   ├──► Subscriptions
   └──► ERP
```

PAY-0008 SHALL govern the exact canonical lifecycle representation.

---

# 76. Refund Reversal

A provider may report that a previously successful or pending Refund was reversed or failed subsequently.

Baobab SHALL model the actual financial effect.

It SHALL not simply delete the Refund.

---

# 77. Immutable History

Financial history SHALL remain append-only/auditable in semantic terms.

Example:

```text
Refund requested
Refund processing
Refund succeeded
Provider reversal observed
```

SHALL not be rewritten into:

```text
Refund never existed
```

---

# 78. Event Semantics

Representative canonical events may include:

```text
payment.cancel_requested
payment.cancelled

payment.void_requested
payment.voided

refund.requested
refund.processing
refund.succeeded
refund.failed

payment.reversal_observed
refund.reversal_observed
```

Exact names SHALL be governed by Shared canonical contracts.

---

# 79. Provider Events Are Observations

A provider webhook stating:

```text
REFUNDED
```

does not automatically become:

```text
refund.succeeded
```

until PAY-0011 verification, mapping, deduplication and state validation complete.

---

# 80. Outbox

Refund/void canonical events SHALL use PAY-0011 transactional-outbox semantics.

---

# 81. Consumer Idempotency

Trade, Subscriptions and ERP SHALL consume refund/reversal events idempotently.

---

# 82. Order Projection

Trade MAY project:

```text
refund requested
partially refunded
fully refunded
```

for commerce UX.

The canonical financial Refund remains owned by Payments.

---

# 83. Payment Fully Refunded

Payments MAY derive:

```text
fully refunded
```

when:

```text
successful refund amount
=
refundable captured amount
```

This does not erase the original successful Payment.

---

# 84. Payment Partially Refunded

Payments MAY derive:

```text
partially refunded
```

when successful refund total is greater than zero but less than the refundable captured amount.

---

# 85. Derived State

`PARTIALLY_REFUNDED` or `FULLY_REFUNDED`, if represented on Payment, SHOULD be treated carefully as derived lifecycle information.

Individual Refund aggregates remain authoritative for refund execution history.

---

# 86. Refund Failure Does Not Change Capture

A failed Refund does not make the original captured Payment unsuccessful.

The Payment remains captured unless another financial event changes that fact.

---

# 87. Refund Success Does Not Erase Capture

Similarly:

```text
Payment captured
Refund succeeded
```

are both historical truths.

---

# 88. Accounting Projection

ERP receives canonical facts such as:

```text
payment captured
refund succeeded
provider reversal
settlement adjustment
```

and determines accounting treatment.

---

# 89. Settlement

A Refund may occur:

```text
before original settlement
after original settlement
within same settlement batch
in a later settlement batch
```

Payments SHALL not assume refund timing relative to settlement.

---

# 90. Refund Settlement

A Refund being accepted or successful does not necessarily mean its settlement impact has already appeared.

PAY-0015 governs settlement and reconciliation.

---

# 91. Fees

Provider refund fees or non-returned processing fees SHALL be represented separately from customer refund principal where required.

Payments SHALL not silently alter the canonical customer refund amount to hide provider fees.

---

# 92. Provider Fee Authority

Provider fee observations may originate from:

```text
provider response
settlement report
reconciliation data
```

ERP determines accounting consequence.

---

# 93. Refund Timing

Payments MAY enforce provider-specific refund windows.

These constraints belong to provider capability/eligibility configuration rather than leaf estate code.

---

# 94. Expired Refund Window

If a provider no longer permits a standard Refund:

```text
Refund request
      │
      ▼
provider capability validation
      │
      X
not eligible
```

Payments SHALL return a canonical failure/ineligibility result.

It SHALL NOT automatically invent a Payout.

---

# 95. Provider Capability

PAY-0022 certification SHALL capture whether a provider supports:

```text
void
full refund
partial refund
multiple refunds
asynchronous refund
refund status query
refund webhook
```

where relevant.

---

# 96. Capability Validation

Before execution:

```text
Requested Operation
       │
       ▼
Original Payment Context
       │
       ▼
Provider Capability
       │
       ▼
Eligibility
```

must be validated.

---

# 97. Refund Authorization

Refunds are controlled financial mutations.

A caller SHALL require appropriate IAM/workload authority.

---

# 98. Suggested Capability Boundary

The existing canonical capability:

```text
payment.refund.create
```

remains the baseline refund capability.

Additional capabilities MAY later distinguish:

```text
payment.void.create
payment.cancel.create
payment.refund.approve
```

if governance requires them.

Exact names belong in Shared/IAM contracts.

---

# 99. Refund Approval

Large or exceptional Refunds MAY require business approval before execution.

Approval policy MAY depend on:

```text
tenant
Legal Entity
amount
currency
role
market
reason
commercial domain
```

---

# 100. Approval Is Not Execution

An approved Refund is not a successful Refund.

Conceptually:

```text
Commercial Approval
       │
       ▼
Refund Request
       │
       ▼
Payments Execution
       │
       ▼
Provider Outcome
```

---

# 101. Separation of Duties

Where required, the person requesting a Refund SHOULD be distinct from the person approving it.

Payments SHALL support enforcement through IAM/policy integration where applicable.

---

# 102. System-Initiated Refund

Automated Refunds MAY be allowed when a governed business workflow has authority.

The workload identity SHALL still be authenticated, authorised and audited.

---

# 103. Cross-Tenant Authorization

Knowing:

```text
payment_id
```

does not authorise a Refund.

Payments SHALL verify tenant/legal-entity ownership.

---

# 104. Caller-Supplied Provider IDs

A caller SHALL NOT choose:

```text
HyperSwitch Merchant
Business Profile
Connector
PSP
provider refund endpoint
```

to redirect refund execution.

Payments resolves these from trusted historical context.

---

# 105. Idempotency

Every mutating refund/void operation SHALL require or derive a stable idempotency identity.

---

# 106. Refund Idempotency Scope

A Refund idempotency identity SHALL bind sufficiently to:

```text
tenant
Legal Entity
original Payment
logical refund operation
amount
currency
```

as defined by PAY-0010.

---

# 107. Idempotency Conflict

If the same idempotency key is reused with a materially different refund request, Payments SHALL reject the conflict.

---

# 108. Retry

Technical retry of a Refund SHALL preserve the same logical Refund identity.

---

# 109. Provider Retry

Payments determines whether provider retry is safe.

Trade SHALL NOT repeatedly call refund APIs until one “looks successful.”

---

# 110. Ambiguous Outcome

If provider execution is ambiguous:

```text
DO NOT
create a new Refund
```

until the original Refund's state has been recovered.

---

# 111. Recovery

Refund recovery MAY use:

```text
HyperSwitch query
provider query
webhook
settlement/reconciliation evidence
```

depending on capability.

---

# 112. Webhook

Refund webhooks follow PAY-0011:

```text
receive
verify
deduplicate
map
validate
mutate
outbox
publish
```

---

# 113. Orphan Refund Webhook

A valid provider refund callback without a known canonical Refund SHALL enter controlled reconciliation.

Payments SHALL not automatically create a customer refund merely from an unknown external reference.

---

# 114. External Refund

If an operator or provider created a refund outside Baobab, reconciliation may discover it.

Such a financial fact SHALL be imported through a controlled reconciliation path.

---

# 115. Out-of-Band Refund

An out-of-band refund is a governance anomaly unless explicitly supported.

It SHOULD generate operational/audit visibility.

---

# 116. Manual Provider Console Actions

Production operators SHOULD NOT routinely issue refunds directly through HyperSwitch or PSP dashboards outside Baobab governance.

Where emergency manual action is unavoidable, it SHALL be reconciled into canonical Payments state.

---

# 117. Control Center

HyperSwitch Control Center remains operational tooling.

It does not replace canonical Refund authorization or Payments state.

---

# 118. Historical Provider Configuration

Payments SHALL preserve enough historical execution context to determine:

```text
which provider
which merchant
which profile
which external payment
```

must receive a refund request.

---

# 119. Historical Context Immutability

Changing today's routing configuration SHALL not change which provider handled yesterday's Payment.

---

# 120. Provider Failover

Refunds SHALL NOT use ordinary new-payment failover logic blindly.

If Provider A captured the payment:

```text
Provider A
    │
    ▼
original transaction
```

Provider B cannot ordinarily refund that transaction.

---

# 121. Connector Failure During Refund

If the historical provider route is unavailable, Payments SHALL:

```text
retry safely
recover
alert
escalate
```

rather than reroute to an unrelated provider.

---

# 122. Refund Availability SLO

Refund processing SHALL have operational SLOs distinct from checkout latency where appropriate.

PAY-0019 governs exact SLOs.

---

# 123. Refund Backlog

Payments SHALL monitor:

```text
refunds requested
refunds processing
refunds unknown
refunds failed
refunds awaiting recovery
```

---

# 124. Observability

Every Refund SHOULD be traceable through:

```text
refund_id
payment_id
payment_intent_id
correlation_id
source_engine
source_reference
tenant_id
legal_entity_id
market_id
provider
external_payment_reference
external_refund_reference
```

subject to PAY-0012 minimisation.

---

# 125. Metrics

Representative metrics include:

```text
refund_requests_total
refund_success_total
refund_failures_total
refund_processing_duration
refund_unknown_total
refund_recovery_total
refund_amount_total
void_requests_total
void_success_total
void_failure_total
reversal_observed_total
refund_idempotency_conflicts_total
refund_overallocation_rejected_total
```

Exact telemetry names remain implementation-specific.

---

# 126. Alerting

Operational alerts SHOULD cover:

```text
refund failure spike
refund processing backlog
refund unknown-state growth
provider refund endpoint failure
reversal spike
refund reconciliation mismatch
over-refund attempt
unauthorised refund attempt
manual out-of-band refund
```

---

# 127. Audit

Refund audit evidence SHOULD include:

```text
refund_id
payment_id
tenant
Legal Entity
amount
currency
reason
source business reference
requesting principal/workload
approving principal where applicable
idempotency identity
provider
provider reference
state transitions
timestamps
correlation
```

---

# 128. Sensitive Data

Refund operations SHALL follow PAY-0012.

Refund logs/events SHALL not contain raw payment credentials.

---

# 129. Data Retention

Refund and reversal records are financial history.

They SHALL be retained according to applicable financial, legal, dispute and audit policy.

---

# 130. API Design

Canonical APIs SHOULD expose intent rather than provider terminology.

Conceptually:

```text
POST /payments/{payment_id}/refunds
```

rather than:

```text
POST /stripe/refund
POST /provider-x/reversal
```

Exact paths are implementation-specific.

---

# 131. Void API

If explicit void is exposed:

```text
POST /payments/{payment_id}/void
```

or equivalent SHALL represent the canonical operation.

Provider terminology remains behind the adapter.

---

# 132. Cancellation API

Local Payment cancellation and external authorization void MAY require separate canonical commands where their semantics differ.

The implementation SHALL not overload one command ambiguously.

---

# 133. Command Response

A mutating API response SHALL communicate the canonical state, not merely provider HTTP success.

Example:

```text
Refund:
PROCESSING
```

may be the correct response even after the provider accepted the request.

---

# 134. Provider HTTP 200

A provider HTTP success response does not necessarily prove final refund completion.

Adapter semantics SHALL determine the canonical state.

---

# 135. Trade Integration

Preferred flow:

```text
Trade
  │
  │ commercially authorised refund
  ▼
Baobab Payments
  │
  ▼
Canonical Refund
  │
  ▼
HyperSwitch
  │
  ▼
Original Provider
  │
  ▼
Financial Outcome
  │
  ▼
Canonical Refund Event
  │
  ▼
Trade projection
```

---

# 136. Subscription Integration

```text
Subscriptions
      │
      │ billing adjustment determines refund due
      ▼
Baobab Payments
      │
      ▼
Refund
      │
      ▼
Provider
      │
      ▼
refund.succeeded
      │
      ▼
Subscriptions updates billing projection
```

Payments does not cancel the subscription.

---

# 137. ERP Integration

```text
Payments
   │
   │ refund.succeeded
   ▼
ERP
   │
   ▼
Accounting treatment
```

Payments does not post arbitrary GL entries.

---

# 138. Full Refund Flow

```text
Captured Payment
ZAR 1,000
      │
      ▼
Commercial Refund Decision
ZAR 1,000
      │
      ▼
Refund R1
REQUESTED
      │
      ▼
Validate refundable balance
      │
      ▼
Resolve original provider context
      │
      ▼
HyperSwitch / Provider
      │
      ▼
PROCESSING
      │
      ▼
Provider confirms
      │
      ▼
SUCCEEDED
      │
      ▼
refund.succeeded
      │
   ┌──┼─────────┐
   ▼  ▼         ▼
Trade ERP  Subscriptions
```

---

# 139. Partial Refund Flow

```text
Payment captured
1,000
   │
   ├── Refund R1 200 ──► succeeded
   │
   ├── Refund R2 300 ──► succeeded
   │
   ▼
Refunded total = 500
Remaining refundable = 500
```

---

# 140. Void Flow

```text
Payment
AUTHORIZED
   │
   ▼
Business no longer wants capture
   │
   ▼
Void Request
   │
   ▼
Payments validates state
   │
   ▼
Original Provider Context
   │
   ▼
Provider void/reversal operation
   │
   ▼
Confirmed
   │
   ▼
VOIDED / CANCELLED canonical outcome
```

No Refund is created if captured funds never required returning.

---

# 141. Capture/Cancellation Race

```text
             AUTHORIZED
                 │
        ┌────────┴────────┐
        ▼                 ▼
Capture requested      Void requested
        │                 │
        └────────┬────────┘
                 ▼
           Provider state
                 │
         ┌───────┴───────┐
         ▼               ▼
    Capture wins      Void wins
         │               │
         ▼               ▼
     CAPTURED          VOIDED
         │
         ▼
If business cancellation
still requires return:
create Refund
```

---

# 142. Refund Timeout Flow

```text
Refund R1
   │
   ▼
Provider request
   │
   X
timeout
   │
   ▼
DO NOT CREATE R2
   │
   ▼
Query / webhook / reconciliation
   │
 ┌─┴───────────┐
 ▼             ▼
Succeeded     Failed
 │             │
 ▼             ▼
R1 success   R1 failure
```

---

# 143. Provider Reversal Flow

```text
Payment CAPTURED
      │
      ▼
Provider later reports reversal
      │
      ▼
PAY-0011 verification
      │
      ▼
Resolve canonical Payment
      │
      ▼
Determine financial meaning
      │
      ▼
Canonical reversal fact
      │
      ├──► Trade
      ├──► Subscriptions
      └──► ERP
```

---

# 144. Refund Reconciliation

```text
Canonical Refund
      │
      ▼
Provider Refund
      │
      ▼
Settlement / Provider Evidence
      │
      ▼
Compare
      │
 ┌────┴─────────┐
 ▼              ▼
MATCH         MISMATCH
 │              │
 ▼              ▼
close       investigate /
            reconcile
```

---

# 145. Production Readiness Gate

```text
REFUND / VOID / REVERSAL READINESS

Canonical Refund aggregate                PASS
Refund state machine                      PASS
Void semantics                            PASS
Cancellation semantics                    PASS
Provider reversal translation             PASS
Partial refund                            PASS
Multiple refund                           PASS
Refundable-balance enforcement            PASS
Concurrent refund safety                  PASS
Idempotency                               PASS
Unknown-outcome recovery                  PASS
Original-provider resolution              PASS
Historical merchant/profile context       PASS
Cross-tenant isolation                    PASS
Cross-Legal-Entity isolation              PASS
Currency validation                       PASS
Refund authorization                      PASS
Approval policy where required            PASS
Provider capability validation            PASS
Provider certification                    PASS
Webhook handling                          PASS
Canonical events                          PASS
Trade projection                          PASS
Subscription projection                   PASS
ERP projection                            PASS
Settlement reconciliation                 PASS
Dispute interaction                       PASS
Observability                             PASS
Audit                                     PASS
Sensitive-data minimisation               PASS
Sandbox tests                             PASS
Production integration tests              PASS
------------------------------------------------
Refund / Void / Reversal Plane            READY
```

---

# 146. Invariants

The following invariants SHALL hold:

1. Payment cancellation is not Refund.
2. Void is not Refund.
3. Provider reversal is not automatically Refund.
4. Refund is not Chargeback.
5. Refund is not ERP credit note.
6. Order cancellation is not proof of Refund.
7. Subscription cancellation is not proof of Refund.
8. Refund requested is not Refund succeeded.
9. Provider HTTP acceptance is not necessarily Refund completion.
10. Refund has its own canonical identity.
11. A Payment may have multiple Refunds.
12. Partial Refunds are supported where provider capability permits.
13. Successful Refund total cannot exceed refundable financial amount.
14. Concurrent Refund requests cannot create over-refund.
15. Refund currency is explicit.
16. Refunds do not silently perform FX.
17. Provider refund identifiers remain external references.
18. Refunds normally execute against the original payment/provider context.
19. New-payment routing does not permit arbitrary Refund rerouting.
20. Group membership does not permit cross-Legal-Entity Refund execution.
21. Cross-tenant Refund execution is prohibited.
22. Caller-supplied provider identifiers cannot redirect a Refund.
23. A Refund does not require raw payment credentials.
24. Refund to arbitrary new beneficiary is not ordinary Refund semantics.
25. Payout and Refund remain distinct.
26. Store credit and Refund remain distinct.
27. Refund and Chargeback remain distinct.
28. Failed Refund does not make original Payment unsuccessful.
29. Successful Refund does not erase original capture history.
30. Refund lifecycle remains auditable.
31. External reversals do not erase prior canonical facts.
32. Provider vocabulary does not define canonical semantics.
33. Payment state is evaluated before deciding void versus Refund.
34. Unknown payment outcome is resolved before unsafe compensation.
35. Timeout does not automatically mean Refund failure.
36. Timeout does not justify duplicate Refund.
37. Every mutating Refund operation is idempotent.
38. A new partial Refund has a distinct logical identity.
39. Refund webhook processing follows PAY-0011.
40. Canonical refund events use outbox semantics.
41. Consumers process Refund events idempotently.
42. Refund execution is a controlled financial mutation.
43. Knowing a Payment ID does not grant Refund authority.
44. Approval is distinct from execution.
45. Commercial domain owns why/amount subject to financial validation.
46. Payments owns execution and operational outcome.
47. ERP owns accounting consequence.
48. Trade owns commerce consequence.
49. Subscriptions owns subscription/billing consequence.
50. Control Plane owns canonical business context.
51. IAM governs identity/authorization.
52. Provider deactivation for new payments does not automatically eliminate historical Refund capability.
53. Historical payment context is preserved.
54. Current routing changes do not rewrite historical provider identity.
55. Out-of-band provider Refunds require reconciliation.
56. Manual provider-console action does not bypass canonical state.
57. Refund principal and provider fees remain conceptually distinct.
58. Settlement and Refund completion remain distinct.
59. Reconciliation can detect and repair projection divergence without rewriting history.
60. Raw payment credentials never enter Refund contracts/events/logs.

---

# 147. Consequences

## Positive

This decision provides:

- provider-independent refund semantics;
- safe partial refunds;
- protection against over-refunding;
- clear void-versus-refund behavior;
- safe handling of provider reversals;
- historical integrity;
- tenant and Legal Entity isolation;
- provider migration safety;
- asynchronous refund recovery;
- accounting separation;
- business-domain separation;
- strong idempotency;
- operational reconciliation.

## Negative

The architecture requires:

- a dedicated Refund aggregate;
- refundable-balance accounting;
- concurrency controls;
- provider-specific semantic translation;
- historical provider context;
- refund recovery;
- refund reconciliation;
- richer event contracts;
- business-domain coordination;
- additional operational tooling.

These costs are accepted because compensation operations move real money and cannot safely be represented as simple status changes.

---

# 148. Alternatives Considered

## Set Payment.status = REFUNDED

Rejected.

It cannot adequately represent:

```text
partial refunds
multiple refunds
failed refunds
pending refunds
reversed refunds
```

---

## Treat void and Refund as one operation

Rejected.

They have different financial semantics.

---

## Let Trade call provider Refund APIs directly

Rejected.

This bypasses payment authority, routing context, idempotency and audit.

---

## Let ERP initiate PSP Refunds

Rejected.

Accounting authority is not payment execution authority.

---

## Always issue Refund when order is cancelled

Rejected.

The payment may never have been captured.

---

## Retry a Refund with a new request after timeout

Rejected.

The first Refund may have succeeded.

---

## Route Refund through any currently active provider

Rejected.

The new provider usually does not own the original financial transaction.

---

## Treat provider reversal as Refund

Rejected.

Provider reversal has multiple possible financial meanings.

---

## Delete failed/reversed Refunds

Rejected.

Financial history must remain auditable.

---

# 149. Relationship to PAY-0008

PAY-0008 defines canonical payment lifecycle.

PAY-0013 adds compensating/return operations around that lifecycle.

Conceptually:

```text
PAY-0008
Payment State
     │
     ▼
Determine permitted compensation
     │
     ├── pre-capture ─► cancellation / void
     │
     └── post-capture ─► Refund
```

---

# 150. Relationship to PAY-0009

PAY-0009 routing determines provider eligibility for new payment execution.

PAY-0013 constrains Refund routing further:

```text
REFUND ROUTE
=
historical payment route
+
provider refund capability
+
current operational availability
+
legal/merchant compatibility
```

not simply the highest-ranked currently eligible collection provider.

---

# 151. Relationship to PAY-0010

PAY-0010 is critical because:

```text
duplicate payment
```

and:

```text
duplicate refund
```

are both financial integrity failures.

Refund idempotency therefore receives the same strict treatment as payment execution.

---

# 152. Relationship to PAY-0011

Provider refund/reversal callbacks are untrusted observations until PAY-0011 establishes canonical facts.

```text
Provider callback
      │
      ▼
Verify
      │
      ▼
Deduplicate
      │
      ▼
Resolve Refund / Payment
      │
      ▼
Validate transition
      │
      ▼
Canonical Refund/Reversal Fact
```

---

# 153. Relationship to PAY-0012

Refunds operate on canonical/provider references.

They SHALL NOT require recovery of raw credentials.

```text
Refund
   │
   ▼
Original Payment
   │
   ▼
Secure provider reference
   │
   ▼
Provider refund operation
```

---

# 154. Relationship to PAY-0014

PAY-0014 SHALL govern:

```text
dispute
chargeback
evidence
representment
dispute outcome
```

PAY-0013 governs merchant/business-originated return of funds and provider reversal observations.

The two domains may interact but SHALL not be collapsed.

---

# 155. Relationship to PAY-0015

PAY-0015 SHALL reconcile:

```text
Payment
Refund
Reversal
Provider settlement
Fees
Net settlement
ERP accounting
```

A Refund is not operationally complete merely because its API lifecycle says succeeded if later settlement evidence reveals divergence.

---

# 156. Complete Financial Return Model

```text
                    ORIGINAL PAYMENT
                          │
                          ▼
                 ┌─────────────────┐
                 │ PAYMENT STATE   │
                 └────────┬────────┘
                          │
           ┌──────────────┴──────────────┐
           │                             │
      NOT CAPTURED                    CAPTURED
           │                             │
           ▼                             ▼
   CANCEL / VOID                      REFUND
           │                             │
           ▼                             ▼
 Original Provider                Original Provider
           │                             │
           ▼                             ▼
 Confirmed release               Funds-return outcome
           │                             │
           ▼                             ▼
 Canonical Payment Fact          Canonical Refund Fact
           │                             │
           └──────────────┬──────────────┘
                          ▼
                    EVENT PLANE
                          │
                ┌─────────┼─────────┐
                ▼         ▼         ▼
              Trade     ERP   Subscriptions
```

Alongside this:

```text
PROVIDER-INITIATED REVERSAL
           │
           ▼
Verify / classify financial effect
           │
           ▼
Canonical reversal fact
           │
           ▼
Downstream business/accounting consequence
```

---

# 157. Final Decision

Baobab SHALL permanently distinguish:

```text
COMMERCIAL CANCELLATION
        ≠
PAYMENT CANCELLATION

PAYMENT CANCELLATION
        ≠
VOID

VOID
        ≠
REFUND

REFUND
        ≠
CHARGEBACK

REFUND
        ≠
PAYOUT

REFUND
        ≠
CREDIT NOTE

PROVIDER REVERSAL
        ≠ automatically
REFUND
```

The authoritative decision flow is:

```text
BUSINESS DOMAIN
determines whether money should be returned/released
        │
        ▼
PAYMENTS
examines canonical financial state
        │
        ├── not captured
        │       │
        │       ▼
        │   CANCEL / VOID
        │
        └── captured
                │
                ▼
             REFUND
                │
                ▼
       HISTORICAL PROVIDER CONTEXT
                │
                ▼
         HYPERSWITCH / PROVIDER
                │
                ▼
        CANONICAL FINANCIAL FACT
                │
                ▼
           EVENT PLANE
                │
      ┌─────────┼──────────┐
      ▼         ▼          ▼
    Trade      ERP   Subscriptions
```

**The business domain decides why money should be returned.  
Baobab Payments determines which financial operation is valid.  
The original payment state determines whether that operation is cancellation, void, or Refund.  
The provider executes the financial operation.  
Payments establishes what actually happened.  
ERP records the accounting consequence.**

**No status change, business cancellation, provider vocabulary, timeout, or retry may substitute for that authority chain.**