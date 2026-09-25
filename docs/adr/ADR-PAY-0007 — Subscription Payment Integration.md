# ADR-PAY-0007 — Subscription Payment Integration

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Subscription Billing Integration / Payment Execution |
| **Scope** | `baobab-subscriptions` ↔ `baobab-payments` |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0006; ADR-SUB-0001; ADR-SUB-0003; applicable Subscription, Control Plane, IAM and Shared ADRs; ADR-SHARED-011 |
| **Related** | ADR-SUB-0013; ADR-PAY-0008; ADR-PAY-0009; ADR-PAY-0010; ADR-PAY-0011; ADR-PAY-0012; ADR-PAY-0013; ADR-PAY-0015; ADR-PAY-0017; ADR-PAY-0018; ADR-PAY-0022 |

---

# 1. Context

Baobab separates subscription lifecycle and billing policy from payment execution.

`baobab-subscriptions`, built around the platform's subscription/billing architecture, determines subscription-domain facts such as:

```text
subscription
plan
billing policy
billing period
billing obligation
amount due
currency
credits
delinquency
subscription status
entitlement implications
```

`baobab-payments` owns external monetary execution.

This separation is fundamental because:

```text
subscription obligation
        ≠
payment execution
```

A subscription may exist without requiring external payment.

A billing obligation may exist while payment remains pending.

A payment may fail without immediately terminating a subscription.

A payment may succeed while subscription-domain processing remains incomplete.

An INTERNAL subscription may deliberately have:

```text
monetary charge = 0
```

and SHALL NOT invoke payment execution merely to simulate a successful payment.

The architecture must therefore establish an explicit handoff between subscription billing and payment execution without allowing either engine to absorb the other's authority.

---

# 2. Decision

`baobab-subscriptions` SHALL hand monetary payment obligations to `baobab-payments` through canonical Baobab payment contracts.

The conceptual path is:

```text
Subscription
     │
     ▼
Billing Policy
     │
     ▼
Billing Obligation
     │
     ├── payment required? ── NO ──► remain inside Subscriptions
     │
     └── YES
           │
           ▼
      PaymentsPort
           │
           ▼
   Baobab Payment API
           │
           ▼
      PaymentIntent
           │
           ▼
      Payment Execution
           │
           ▼
       HyperSwitch
           │
           ▼
     Eligible Provider
```

`baobab-subscriptions` SHALL NOT normally integrate directly with HyperSwitch or a PSP.

---

# 3. Governing Principle

> **Subscriptions determines whether money is owed and when collection should occur. Payments determines how that monetary obligation is executed and what happened during payment execution.**

Therefore:

```text
SUBSCRIPTIONS
owns
WHY / WHEN / HOW MUCH IS OWED

PAYMENTS
owns
HOW PAYMENT IS EXECUTED
AND WHAT HAPPENED

ERP
owns
ACCOUNTING CONSEQUENCES
```

---

# 4. Authority Matrix

| Concern | Authority |
|---|---|
| Subscription | Subscriptions |
| Plan | Subscriptions |
| Billing policy | Subscriptions |
| Billing period | Subscriptions |
| Billing obligation | Subscriptions |
| Amount due | Subscriptions |
| Billing currency | Subscriptions |
| Credits / subscription adjustments | Subscriptions |
| Delinquency policy | Subscriptions |
| Subscription lifecycle | Subscriptions |
| Entitlement consequence | Subscriptions / applicable capability authority |
| Whether payment should be attempted | Subscriptions |
| Canonical PaymentIntent | Payments |
| Payment execution | Payments |
| Payment state | Payments |
| Provider eligibility | Payments |
| Provider routing | Payments |
| HyperSwitch orchestration | Payments |
| Provider references | Payments |
| Refund execution | Payments |
| Settlement observation | Payments |
| Tenant / Organisation / Legal Entity / Market | Control Plane |
| Enterprise identity | IAM |
| Accounting / GL / revenue recognition | ERP |

---

# 5. Fundamental Question Separation

The architecture SHALL distinguish three different questions.

```text
QUESTION 1

Should payment be attempted?
        │
        ▼
Subscriptions / Billing Policy
```

```text
QUESTION 2

Was payment executed?
        │
        ▼
Baobab Payments
```

```text
QUESTION 3

What accounting consequence follows?
        │
        ▼
ERP
```

No engine SHALL answer all three merely for implementation convenience.

---

# 6. Subscription Is Not Payment

A Subscription SHALL remain independent of Payment.

Therefore:

```text
Subscription
      ≠
PaymentIntent
```

and:

```text
Subscription
      ≠
Payment
```

and:

```text
Billing Obligation
      ≠
Payment
```

The objects are related through explicit references.

---

# 7. Billing Obligation

A Billing Obligation represents Subscriptions' determination that an amount is owed under subscription policy.

Conceptually:

```text
BillingObligation
│
├── subscription reference
├── billing period
├── amount
├── currency
├── due date
├── billing policy
├── collection requirement
└── status
```

The exact contract remains owned by the Subscription domain.

Payments SHALL NOT redefine it.

---

# 8. Payment Obligation Handoff

When a billing obligation requires monetary collection, Subscriptions SHALL create a payment handoff through its PaymentsPort.

Conceptually:

```text
Billing Obligation
       │
       ▼
Payment Required
       │
       ▼
PaymentsPort
       │
       ▼
Create / Resolve PaymentIntent
```

The handoff SHALL preserve a stable source reference.

---

# 9. Source Reference

Subscription-originated PaymentIntents SHALL identify their origin.

Conceptually:

```text
source_engine = baobab-subscriptions
source_reference = <billing obligation reference>
```

Payments SHALL not require a complete copy of the Subscription aggregate.

---

# 10. No Subscription Duplication

`baobab-payments` SHALL NOT maintain a shadow Subscription domain.

It MAY retain:

```text
source_engine
source_reference
amount
currency
canonical context
correlation
payment metadata permitted by contract
```

It SHALL NOT become authoritative for:

```text
plan
billing interval
renewal date
subscription status
entitlement
usage
credits
billing policy
```

---

# 11. Amount Authority

Subscriptions is authoritative for the amount to be collected.

Payments SHALL NOT independently recalculate:

```text
subscription price
usage charge
credit
discount
proration
billing-period amount
```

Payments validates the resulting monetary execution request.

---

# 12. Currency Authority

Subscriptions SHALL provide the authoritative transaction currency of the billing obligation.

Payments SHALL validate the currency against PAY-0005.

Payments SHALL NOT silently convert the obligation because another provider route prefers a different currency.

Cross-currency behaviour belongs to PAY-0017.

---

# 13. INTERNAL Subscriptions

An INTERNAL subscription or equivalent zero-charge subscription classification SHALL NOT invoke payment execution merely because a subscription billing cycle occurred.

The mandatory decision is:

```text
Billing Cycle
      │
      ▼
Does this obligation require monetary payment?
      │
      ├── NO
      │    │
      │    ▼
      │  Subscription processing only
      │
      └── YES
           │
           ▼
        PaymentsPort
```

This is a hard invariant.

---

# 14. Zero Monetary Charge

Where:

```text
amount due = 0
```

because of legitimate subscription policy such as:

```text
INTERNAL classification
credit
promotion
waiver
fully offset adjustment
```

Subscriptions SHALL resolve the billing consequence within its own domain.

It SHALL NOT create a zero-value payment merely to obtain a `payment.succeeded` event.

---

# 15. Why Zero-Value Payment Is Prohibited

Creating artificial zero-value payments would:

- pollute payment history;
- confuse settlement and reconciliation;
- create misleading provider metrics;
- unnecessarily couple subscriptions to Payments;
- potentially invoke providers without financial purpose;
- obscure the distinction between billing and collection.

Therefore:

```text
zero monetary obligation
        ≠
successful external payment
```

---

# 16. PaymentsPort

`baobab-subscriptions` SHALL interact with Payments through a dedicated PaymentsPort or equivalent application boundary.

Conceptually:

```text
Subscription Domain
      │
      ▼
PaymentsPort
      │
      ▼
Canonical Payment Contract
      │
      ▼
baobab-payments
```

The port SHALL prevent HyperSwitch concepts from leaking into subscription logic.

---

# 17. PaymentsPort Responsibilities

The PaymentsPort MAY support operations such as:

```text
create payment intent
request payment execution
retrieve payment status
request cancellation where valid
request refund where subscription policy requires it
```

It SHALL NOT expose:

```text
PSP credentials
HyperSwitch organisation
HyperSwitch merchant
business profile
connector account
routing algorithm
provider failover
```

as subscription-domain concerns.

---

# 18. Provider Abstraction

From Subscriptions' perspective:

```text
Payment Executor = Baobab Payments
```

not:

```text
Payment Executor = PSP X
```

Payments determines the downstream eligible provider.

---

# 19. Canonical Context

The subscription payment handoff SHALL preserve sufficient canonical context for Payments to resolve:

```text
tenant
organisation where applicable
legal entity
market
source engine
source reference
currency
```

Payments then resolves:

```text
payment engine instance
merchant
business profile
payment-method eligibility
provider eligibility
routing
```

---

# 20. Legal Entity

Every monetary subscription obligation SHALL be attributable to the Legal Entity under whose authority the billing relationship exists.

A subscription belonging to one Legal Entity SHALL NOT execute through another entity's Merchant merely because both belong to the same corporate group.

---

# 21. Market

Subscription billing context SHALL resolve to the applicable canonical Market where Market is relevant.

A provider or HyperSwitch Business Profile SHALL NOT independently define the subscription's Market.

---

# 22. Payment Method

Subscriptions MAY maintain or reference the customer's preferred payment arrangement.

However, payment-method eligibility remains a Payments decision within the canonical context.

Conceptually:

```text
Subscriptions:
preferred method = CARD
        │
        ▼
Payments:
Is CARD currently eligible
for this context?
```

---

# 23. Stored Payment Method

Recurring collection frequently requires a reusable Payment Method Reference.

Such references SHALL be handled according to PAY-0012.

Subscriptions SHALL NOT store raw:

```text
card number
CVV
provider credential
```

for recurring collection.

---

# 24. Payment Method Reference Versus Subscription

A reusable token/reference SHALL remain separate from the Subscription itself.

Conceptually:

```text
Subscription
     │
     ▼
Payment Arrangement Reference
     │
     ▼
Payments / secure token domain
```

A subscription record SHOULD reference rather than duplicate sensitive payment material.

---

# 25. Token Portability

Subscriptions SHALL NOT assume a stored payment token is portable across:

```text
provider
merchant
legal entity
environment
```

Payments determines whether the reference can be used for a particular execution route.

---

# 26. Mandates

Where a payment method requires a mandate, consent or recurring-payment authorisation, the mandate SHALL be treated as an explicit governed payment artefact/reference.

A subscription's existence does not itself constitute payment-network authorisation.

Therefore:

```text
Active Subscription
       ≠
Valid Payment Mandate
```

---

# 27. Customer Consent

Subscription commercial consent and payment-method consent MAY be related but SHALL remain semantically distinct.

For example:

```text
customer agrees to subscription
```

does not automatically prove:

```text
customer authorised recurring debit
```

where the payment rail requires separate authorisation.

---

# 28. PaymentIntent per Obligation

A monetary billing obligation SHOULD normally resolve to a distinct PaymentIntent or clearly governed collection intent.

Conceptually:

```text
Subscription
   │
   ├── Billing Obligation Jan
   │       └── PaymentIntent Jan
   │
   ├── Billing Obligation Feb
   │       └── PaymentIntent Feb
   │
   └── Billing Obligation Mar
           └── PaymentIntent Mar
```

This preserves billing-period traceability.

---

# 29. PaymentIntent Is Not Subscription

A PaymentIntent SHALL not remain indefinitely reusable merely because the Subscription continues.

Each monetary obligation must remain traceable to the appropriate billing period/obligation.

---

# 30. Billing Period Integrity

A payment executed for:

```text
Billing Period A
```

SHALL NOT silently satisfy:

```text
Billing Period B
```

unless Subscriptions explicitly applies that payment according to its domain policy.

Payments reports the execution fact.

Subscriptions determines which obligation it satisfies.

---

# 31. Collection State

Subscriptions MAY maintain a collection projection such as:

```text
NOT_REQUIRED
DUE
COLLECTION_PENDING
PAID
FAILED
DELINQUENT
```

according to its ADRs.

This projection SHALL NOT replace Payments' canonical execution state.

---

# 32. Payment State

Payments SHALL maintain its canonical lifecycle independently.

For example:

```text
REQUIRES_ACTION
PROCESSING
AUTHORIZED
CAPTURED
FAILED
CANCELLED
```

according to PAY-0008.

The two state machines SHALL be mapped rather than merged.

---

# 33. State Mapping

Conceptually:

```text
Canonical Payment State
        │
        ▼
Subscription Payment Projection
        │
        ▼
Billing / Collection Decision
```

Subscriptions SHALL consume the payment fact and determine its own domain transition.

---

# 34. Payment Success Does Not Directly Activate Subscription

The following coupling is prohibited:

```text
provider says success
        │
        ▼
activate subscription directly
```

The correct flow is:

```text
Provider
   │
   ▼
HyperSwitch
   │
   ▼
Payments
   │
   ▼
Canonical Payment Fact
   │
   ▼
Subscriptions
   │
   ▼
Subscription policy evaluates consequence
```

---

# 35. Payment Failure Does Not Automatically Cancel Subscription

Likewise:

```text
payment.failed
```

SHALL NOT automatically mean:

```text
subscription.cancelled
```

Subscriptions owns:

```text
grace period
retry policy
delinquency
suspension
cancellation
manual intervention
entitlement consequences
```

---

# 36. Collection Failure

When payment execution fails, Payments SHALL report the canonical payment result.

Subscriptions then determines whether to:

```text
retry
wait
mark past due
enter grace period
request another method
suspend
cancel
escalate
```

according to subscription policy.

---

# 37. Delinquency

Delinquency is a subscription/billing concept.

Payments MAY report repeated failed attempts.

It SHALL NOT decide:

```text
customer is delinquent
```

as the canonical subscription state.

---

# 38. Retry Policy Boundary

There are two different retry concerns:

```text
PAYMENT EXECUTION RETRY
```

and:

```text
BILLING COLLECTION RETRY
```

They SHALL remain distinct.

---

# 39. Payment Execution Retry

Payments may retry or fail over within one authorised execution operation according to PAY-0009.

For example:

```text
Provider A
   │
   ├── transient failure
   │
   ▼
Provider B
```

where allowed.

This remains one payment-execution concern.

---

# 40. Billing Collection Retry

Subscriptions may decide:

```text
attempt collection again tomorrow
```

after a failed billing attempt.

That is a new collection-policy decision.

Payments SHALL not independently schedule subscription dunning merely because a Payment failed.

---

# 41. Retry Distinction

Therefore:

```text
Provider Retry
      ≠
Subscription Collection Retry
```

and:

```text
Routing Failover
      ≠
Dunning
```

---

# 42. Idempotency

Subscription → Payments mutation calls SHALL be idempotent.

A repeated delivery of:

```text
collect billing obligation X
```

SHALL NOT create multiple unintended charges.

PAY-0010 defines the detailed mechanism.

---

# 43. Billing Obligation as Idempotency Anchor

The stable billing obligation reference SHOULD participate in idempotency semantics.

Conceptually:

```text
tenant
+
legal entity
+
billing obligation
+
payment operation
```

provides the logical execution scope.

Exact key construction belongs to PAY-0010.

---

# 44. Duplicate Scheduler Execution

If the subscription scheduler processes the same billing period twice because of:

```text
worker retry
crash recovery
message redelivery
scheduler overlap
```

Payments SHALL not create duplicate money movement for the same logical payment command.

---

# 45. Correlation

The integration SHALL preserve a traceable chain:

```text
Subscription
    │
    ▼
Billing Period
    │
    ▼
Billing Obligation
    │
    ▼
PaymentIntent
    │
    ▼
Payment
    │
    ▼
Payment Attempt
    │
    ▼
HyperSwitch
    │
    ▼
Provider
```

---

# 46. Correlation Identifiers

Relevant identifiers SHOULD include:

```text
tenant_id
legal_entity_id
subscription_id
billing_obligation_id
payment_intent_id
payment_id
correlation_id
```

with external provider references retained inside Payments.

---

# 47. Asynchronous Execution

Subscription collection SHALL assume that payment execution may be asynchronous.

Possible flows include:

```text
3DS
bank debit
bank transfer
mobile money
delayed provider processing
webhook-confirmed completion
```

Therefore:

```text
collection requested
        ≠
collection completed
```

---

# 48. Pending Collection

Subscriptions SHALL be capable of representing:

```text
payment requested
but outcome not final
```

without fabricating success or failure.

---

# 49. Customer Action

Some recurring-payment attempts may require customer action.

Payments SHALL expose canonical:

```text
REQUIRES_ACTION
```

semantics where applicable.

Subscriptions determines how to notify or engage the customer.

---

# 50. Customer Action Does Not Change Authority

Even where customer interaction is required:

```text
Subscriptions
     │
     ▼
Customer notification
     │
     ▼
Payment action
```

the provider result still converges through Payments.

The browser or customer redirect is not canonical proof of payment success.

---

# 51. Webhooks

Provider webhooks SHALL terminate inside the payment boundary.

Conceptually:

```text
Provider
   │
   ▼
HyperSwitch
   │
   ▼
baobab-payments
   │
   ▼
Canonical Payment Event
   │
   ▼
baobab-subscriptions
```

Subscriptions SHALL NOT consume raw PSP webhook payloads as canonical payment state.

---

# 52. Event Semantics

Relevant canonical events may include:

```text
payment.requires_action
payment.authorized
payment.captured
payment.failed
payment.cancelled
payment.refunded
```

according to the versioned Payments event contract.

PAY-0011 defines exact delivery semantics.

---

# 53. Event Idempotency

Subscriptions SHALL consume payment events idempotently.

Duplicate delivery of:

```text
payment.captured
```

SHALL NOT:

```text
credit the billing obligation twice
extend the subscription twice
issue duplicate entitlement
create duplicate accounting consequence
```

---

# 54. Event Ordering

Subscriptions SHALL not assume perfect event ordering.

It SHALL converge using:

```text
canonical identifiers
event identity
payment state
version where applicable
```

rather than relying solely on arrival order.

---

# 55. Payment Recovery

If a synchronous collection request times out, Subscriptions SHALL NOT immediately assume payment failure and create another charge.

Conceptually:

```text
Collection Request
       │
       ▼
Timeout
       │
       ▼
UNKNOWN / PROCESSING
       │
       ▼
Recover Payment State
       │
       ├── succeeded
       ├── failed
       └── still pending
```

This is essential to duplicate-payment prevention.

---

# 56. Provider Failover

Provider failover remains a Payments concern.

Subscriptions SHALL not contain:

```text
if ProviderA fails:
    call ProviderB
```

logic.

The subscription engine asks Payments to execute an authorised obligation.

Payments selects the eligible route.

---

# 57. Failover Constraints

Failover SHALL NOT silently change:

```text
tenant
legal entity
market
currency
payment method semantics
billing obligation
```

to obtain a successful charge.

---

# 58. Dunning

Dunning policy belongs to the subscription/billing domain.

Representative policy may include:

```text
retry schedule
grace period
customer notification
method replacement request
service restriction
suspension
cancellation
```

Payments provides execution facts required by that policy.

Payments does not own the policy.

---

# 59. Payment Retry Versus Dunning Flow

```text
Billing Obligation
       │
       ▼
Collection Attempt
       │
       ▼
Payments
       │
       ├── internal provider retry/failover
       │
       ▼
Final canonical outcome
       │
       ▼
Subscriptions
       │
       ├── paid ─────────► settle collection state
       │
       └── failed
              │
              ▼
          Dunning Policy
              │
              ▼
      future collection decision
```

---

# 60. Credits

Subscription credits SHALL remain Subscriptions-domain financial adjustments until they produce an external monetary execution requirement.

For example:

```text
Charge 100
Credit 100
──────────
Amount Due 0
```

results in:

```text
No payment execution
```

not a 100-unit charge followed by a 100-unit refund merely to reproduce billing arithmetic.

---

# 61. Proration

Proration belongs to subscription billing policy.

Payments receives the resulting authoritative amount.

It SHALL not calculate subscription proration.

---

# 62. Usage-Based Billing

Where subscriptions support usage-based billing:

```text
Usage
   │
   ▼
Subscription Billing
   │
   ▼
Amount Due
   │
   ▼
Payment Obligation
```

Payments receives only the authorised monetary result and necessary source context.

It SHALL not calculate usage.

---

# 63. Subscription Upgrade

An upgrade MAY create:

```text
immediate charge
future charge
credit
proration
no charge
```

depending on subscription policy.

Payments SHALL only be invoked when the resulting policy requires monetary execution.

---

# 64. Subscription Downgrade

Likewise, a downgrade may produce:

```text
credit
future adjustment
refund request
no monetary action
```

Subscriptions determines which business operation is required.

Payments executes a refund only when explicitly requested through the appropriate canonical operation.

---

# 65. Cancellation

Subscription cancellation does not automatically mean:

```text
cancel payment
```

or:

```text
refund all historical payments
```

The billing policy determines financial consequences.

Payments executes only explicitly authorised financial operations.

---

# 66. Refund

Where subscription policy determines that money must be returned:

```text
Subscription Decision
       │
       ▼
Refund Request
       │
       ▼
baobab-payments
       │
       ▼
Canonical Refund
       │
       ▼
HyperSwitch / Provider
```

PAY-0013 governs the refund lifecycle.

---

# 67. Refund Is Not Subscription Credit

The architecture SHALL distinguish:

```text
SUBSCRIPTION CREDIT
```

from:

```text
PAYMENT REFUND
```

A credit changes billing obligations.

A refund moves money back through the payment system.

They may be related but are not interchangeable.

---

# 68. Refund Failure

If Subscriptions decides a refund is owed but external refund execution fails:

```text
Refund Obligation
      remains
```

even though:

```text
Refund Execution
      failed
```

Subscriptions and/or the appropriate financial domain SHALL preserve the outstanding business obligation.

---

# 69. Chargeback

A chargeback SHALL NOT be represented as:

```text
subscription refund
```

Chargebacks originate from the payment/provider dispute domain.

PAY-0014 defines the detailed architecture.

Subscriptions MAY consume dispute facts to determine appropriate account consequences.

---

# 70. Settlement

A successful captured payment does not prove settlement.

Therefore:

```text
payment.captured
       ≠
provider settlement
```

and:

```text
provider settlement
       ≠
ERP reconciliation
```

PAY-0015 governs settlement and reconciliation.

---

# 71. Subscription Collection State Versus Settlement

Subscriptions normally requires reliable payment execution facts for collection lifecycle.

It SHOULD NOT become the settlement ledger.

Settlement belongs to the Payments/ERP financial boundary.

---

# 72. Accounting

Subscriptions SHALL NOT infer authoritative accounting entries directly from PSP responses.

Conceptually:

```text
Subscription Billing Fact
       │
       ├──────────────┐
       │              │
       ▼              ▼
Payments           ERP
execution          accounting
       │              ▲
       └──── facts ────┘
```

The exact financial projection architecture is governed by the applicable ERP and payment ADRs.

---

# 73. Revenue Recognition

Revenue recognition SHALL NOT be owned by Payments.

Likewise, successful collection does not by itself define revenue recognition.

ERP remains authoritative for accounting consequences.

---

# 74. Invoices

Where subscription billing produces an invoice or invoice-equivalent financial document, Payments SHALL reference the relevant source where required but SHALL not become the invoice authority.

---

# 75. Collection Before Due Date

Subscriptions decides whether collection may occur:

```text
before due date
on due date
after due date
```

Payments does not schedule subscription collection independently.

---

# 76. Prepaid Subscription

A prepaid subscription may require successful payment before activation.

The authority flow remains:

```text
Subscriptions
determines
payment prerequisite

        │
        ▼

Payments
executes payment

        │
        ▼

Subscriptions
evaluates payment fact

        │
        ▼

Subscriptions
activates if policy permits
```

Payments itself SHALL NOT activate the subscription.

---

# 77. Postpaid Subscription

A postpaid subscription may remain active while billing obligations accumulate.

Payments SHALL not assume that:

```text
subscription active
```

means:

```text
no outstanding payment
```

or vice versa.

---

# 78. Free Trial

A free trial SHALL not require a payment solely because a billing cycle exists.

A payment method MAY be collected in advance where subscription policy requires it, but:

```text
payment method on file
       ≠
payment executed
```

---

# 79. Payment Method Setup

Where a future recurring charge requires establishing a reusable payment method or mandate before money is owed, Payments MAY support a distinct setup flow.

That setup SHALL NOT be misrepresented as a monetary Payment.

The detailed tokenisation/mandate design belongs to PAY-0012.

---

# 80. Trial Conversion

When a trial converts to a paid subscription:

```text
Trial Ends
    │
    ▼
Subscription Policy
    │
    ▼
Billing Obligation
    │
    ▼
Payment Required?
```

Only after the answer is YES does payment execution begin.

---

# 81. Grace Period

Grace period belongs to Subscriptions.

Payments may continue to report:

```text
failed
pending
requires action
```

but SHALL not independently terminate entitlements.

---

# 82. Entitlements

Payments SHALL NOT directly grant or revoke subscription entitlements.

The path is:

```text
Payment Fact
     │
     ▼
Subscriptions
     │
     ▼
Subscription State
     │
     ▼
Entitlement Decision
```

---

# 83. Capability Access

A successful payment does not directly establish platform capability access.

Control Plane/Subscription capability policy remains authoritative.

---

# 84. Multi-Tenant Isolation

Subscription payment execution SHALL preserve canonical tenant isolation.

A payment method or payment belonging to:

```text
Tenant A
```

SHALL NOT satisfy a billing obligation belonging to:

```text
Tenant B
```

without an explicitly designed and authorised financial arrangement.

---

# 85. Legal-Entity Isolation

Likewise:

```text
ZuriBeans billing obligation
```

SHALL NOT be collected through:

```text
Thamani Merchant
```

merely because both belong to the same corporate group.

PAY-0004 remains authoritative.

---

# 86. Market Isolation

A subscription billing obligation in one Market SHALL not silently route through configuration intended exclusively for another Market.

PAY-0005 and PAY-0022 govern eligibility.

---

# 87. External Customers

The architecture SHALL work identically for external Baobab tenants.

No subscription/payment logic SHALL contain special branches such as:

```text
if tenant == NABHOLD
```

for ordinary collection.

---

# 88. Cross-Border Subscriptions

Where a subscription relationship crosses:

```text
market
currency
legal-entity jurisdiction
provider jurisdiction
settlement currency
```

Payments SHALL apply PAY-0017.

Subscriptions SHALL not independently perform hidden FX conversion.

---

# 89. API Authentication

Subscriptions SHALL authenticate to Payments as an authorised workload.

Applicable scopes may include:

```text
payment:execute
payment:refund
payment:read
```

according to Shared/IAM contracts.

Production SHALL not rely on unmanaged static bearer secrets.

---

# 90. Authorisation

Payments SHALL validate:

```text
workload identity
scope
tenant
legal entity
market
source ownership
requested operation
```

before executing subscription-originated commands.

---

# 91. Cross-Subscription Attack

Knowledge of another subscription's:

```text
payment_id
```

or:

```text
payment_method_reference
```

SHALL NOT grant authority to mutate or collect against it.

Canonical ownership validation remains mandatory.

---

# 92. Sensitive Data

Subscriptions SHALL not become a vault for:

```text
PAN
CVV
provider credentials
raw bank credentials
unprotected reusable payment secrets
```

PAY-0012 governs sensitive payment data.

---

# 93. Direct HyperSwitch Integration

`baobab-subscriptions` SHALL NOT normally call HyperSwitch directly.

The canonical path is:

```text
Subscriptions
      │
      ▼
PaymentsPort
      │
      ▼
Baobab Payments
      │
      ▼
HyperSwitch
```

This preserves provider independence and canonical payment authority.

---

# 94. Direct PSP Integration

Likewise, Subscriptions SHALL NOT implement:

```text
ProviderAClient
ProviderBClient
ProviderCClient
```

for normal Baobab payment execution.

Provider integrations belong behind `baobab-payments`.

---

# 95. No Shared Database

Subscriptions SHALL NOT read or write:

```text
baobab-payments database
HyperSwitch database
provider database
```

directly.

Integration occurs through:

```text
versioned APIs
canonical events
```

---

# 96. No Distributed Transaction

Baobab SHALL NOT require a distributed ACID transaction spanning Subscription and Payment databases.

Instead:

```text
durable local transaction
+
idempotent command
+
event delivery
+
reconciliation
```

shall provide convergence.

---

# 97. Failure Scenario — Payment Succeeds, Subscription Update Fails

The architecture SHALL tolerate:

```text
Payments:
CAPTURED

Subscriptions:
temporary update failure
```

without charging again.

Canonical events/recovery SHALL eventually reconcile the Subscription collection state.

---

# 98. Failure Scenario — Request Sent, Response Lost

If:

```text
Subscriptions
      │
      ▼
Payments executes
      │
      X response lost
```

Subscriptions SHALL recover using idempotency and payment lookup.

It SHALL not immediately create another collection attempt.

---

# 99. Failure Scenario — Subscription Commits, Payment Request Fails

If a billing obligation is durably created but payment invocation fails before execution:

```text
Billing Obligation = DUE
Payment = not executed
```

Subscriptions may retry the handoff idempotently.

The obligation SHALL not disappear merely because the payment service was temporarily unavailable.

---

# 100. Reconciliation

Baobab SHOULD support reconciliation between Subscriptions and Payments capable of identifying:

```text
monetary billing obligation
without PaymentIntent

PaymentIntent
without valid subscription source

Subscriptions says paid
Payments has no supporting fact

Payments says captured
Subscriptions still says collection pending

duplicate PaymentIntents
for one billing obligation
```

---

# 101. Operational Recovery

Operators SHOULD be able to trace from:

```text
subscription_id
```

to:

```text
billing obligation
payment intent
payment
payment attempts
provider reference
```

without manually joining private databases.

---

# 102. Error Model

Subscriptions SHOULD consume canonical payment errors such as:

```text
PAYMENT_METHOD_NOT_ELIGIBLE
PAYMENT_ACTION_REQUIRED
PAYMENT_DECLINED
PAYMENT_PROVIDER_UNAVAILABLE
PAYMENT_PROCESSING
PAYMENT_OPERATION_NOT_ALLOWED
PAYMENT_CONTEXT_INVALID
```

rather than provider-specific strings.

---

# 103. Decline

A provider decline SHALL be reported as a payment execution fact.

Subscriptions determines whether that decline triggers:

```text
another collection attempt
customer notification
method replacement
grace period
delinquency
```

---

# 104. Provider Unavailability

Provider infrastructure failure and customer payment decline SHALL remain distinguishable.

They may lead to different subscription collection policy.

Payments SHALL normalise enough semantics to support that distinction.

---

# 105. Observability

The cross-engine trace SHOULD permit reconstruction of:

```text
Tenant
   │
Subscription
   │
Billing Period
   │
Billing Obligation
   │
PaymentIntent
   │
Payment
   │
Payment Attempt
   │
HyperSwitch
   │
Provider
```

---

# 106. Metrics

Useful integration metrics MAY include:

```text
billing obligations requiring payment
payment handoff rate
collection success rate
collection failure rate
requires-action rate
payment latency
dunning-entry rate
idempotency replay count
Subscriptions/Payments state mismatch
zero-charge obligations
provider failures
```

Metrics SHALL preserve tenant isolation.

---

# 107. Production Readiness

Representative readiness:

```text
SUBSCRIPTIONS / PAYMENTS READINESS

Canonical contracts                  PASS
PaymentsPort                         PASS
Workload authentication              PASS
Tenant context                       PASS
Legal Entity context                 PASS
Market context                       PASS
Currency validation                  PASS
Payment-method references            PASS
Mandate handling where required      PASS
Merchant/Profile mapping             PASS
HyperSwitch adapter                  PASS
Provider certification               PASS
Provider activation                  PASS
Idempotency                          PASS
Event convergence                    PASS
Dunning boundary                     PASS
Refund integration                   PASS
Settlement projection                PASS
Reconciliation                       PASS
Observability                        PASS
Security hardening                   PASS
-----------------------------------------
Subscription Payment Integration     READY
```

A sandbox-only payment provider SHALL NOT satisfy production payment readiness.

---

# 108. Implementation Layers

The target architecture is:

```text
baobab-subscriptions
│
├── Subscription Domain
├── Billing Policy
├── Billing Obligation
├── Collection / Delinquency Policy
│
└── PaymentsPort
          │
          ▼
────────────────────────────────────
       canonical API boundary
────────────────────────────────────
          │
          ▼
baobab-payments
│
├── Payment API
├── PaymentIntent
├── Payment Domain
├── Context / Eligibility
├── Idempotency
├── PaymentProvider Port
│      │
│      ├── SandboxProvider
│      └── HyperSwitchProvider
│
└── Canonical Events
          │
          ▼
HyperSwitch
          │
          ▼
PSPs / Payment Rails
```

---

# 109. Contract Testing

Automated contract tests SHOULD verify:

```text
Subscriptions PaymentsPort
        ↕
Payments API
```

including:

```text
payment required
payment not required
zero-charge obligation
successful collection
decline
requires action
timeout
duplicate request
provider unavailability
refund
asynchronous completion
cross-tenant rejection
cross-legal-entity rejection
```

---

# 110. Sandbox Testing

The sandbox provider MAY be used to test subscription collection workflows.

Every resulting payment/event SHALL carry:

```text
simulated = true
```

Sandbox success SHALL not be interpreted as evidence that:

```text
HyperSwitch integration works
provider credentials work
real settlement works
production recurring mandates work
```

---

# 111. HyperSwitch Integration Testing

Production-like integration testing SHOULD exercise:

```text
Subscriptions
      │
      ▼
PaymentsPort
      │
      ▼
Baobab Payments
      │
      ▼
HyperSwitch Adapter
      │
      ▼
HyperSwitch
      │
      ▼
Provider Sandbox
```

where supported.

Testing only against `SandboxProvider` proves the Baobab payment abstraction, not the HyperSwitch integration.

---

# 112. Invariants

The following invariants SHALL hold:

1. Subscription is not Payment.
2. Billing Obligation is not PaymentIntent.
3. PaymentIntent is not Subscription.
4. Subscriptions owns whether payment should be attempted.
5. Payments owns whether payment was executed.
6. ERP owns accounting consequence.
7. Payments does not calculate subscription pricing.
8. Payments does not calculate usage.
9. Payments does not calculate proration.
10. Payments does not determine billing periods.
11. Payments does not determine subscription lifecycle.
12. Payments does not directly grant entitlements.
13. Payment failure does not automatically cancel a subscription.
14. Payment success does not directly activate a subscription.
15. INTERNAL zero-charge subscriptions do not invoke payment execution.
16. Zero monetary obligation is not represented as a successful external payment.
17. Subscription credit is not Payment Refund.
18. Dunning is not provider failover.
19. Billing retry is not payment-routing retry.
20. Active Subscription is not proof of a valid payment mandate.
21. Stored payment references do not imply provider portability.
22. Subscriptions does not route PSPs.
23. Subscriptions does not manage PSP credentials.
24. Subscriptions does not call HyperSwitch directly for normal execution.
25. Provider webhooks terminate within the Payments boundary.
26. Payment events are consumed idempotently.
27. Timeouts do not automatically mean failure.
28. Billing-obligation retries SHALL NOT create duplicate charges.
29. Cross-tenant collection is prohibited.
30. Cross-legal-entity collection is prohibited unless explicitly governed.
31. Capture is not Settlement.
32. Settlement is not ERP reconciliation.
33. Subscription state and Payment state remain separate state machines.
34. The engines do not share databases.
35. Sandbox success is not production readiness.

---

# 113. Consequences

## Positive

This decision:

- preserves Subscription domain authority;
- centralises payment execution;
- prevents PSP coupling in billing logic;
- supports recurring collection;
- supports asynchronous methods;
- supports dunning without confusing it with routing;
- supports INTERNAL subscriptions cleanly;
- prevents artificial zero-value payments;
- centralises token and mandate security;
- supports multi-market subscription billing;
- preserves legal-entity isolation;
- makes Payments reusable across Commerce and Subscriptions;
- enables provider migration without rewriting billing logic.

## Negative

It requires:

- PaymentsPort integration;
- distributed-state convergence;
- payment/source correlation;
- idempotent collection handoff;
- payment event consumption;
- dunning/payment-state mapping;
- recovery and reconciliation;
- secure reusable payment-method handling.

These costs are accepted because merging billing policy and payment execution would create substantially greater financial, security and operational coupling.

---

# 114. Alternatives Considered

## Let the Subscription engine call PSPs directly

Rejected.

It duplicates payment orchestration and couples billing to provider infrastructure.

---

## Let Kill Bill or the billing engine own all payment execution

Rejected as the Baobab platform boundary.

Subscription/billing policy remains distinct from platform-wide payment orchestration.

---

## Let Payments determine subscription collection schedules

Rejected.

Collection scheduling and dunning are subscription/billing policy.

---

## Create a Payment for every billing cycle including zero-value cycles

Rejected.

It pollutes the payment domain and violates the distinction between billing and money movement.

---

## Treat payment failure as automatic subscription cancellation

Rejected.

Grace periods, retries, delinquency and cancellation are subscription-policy decisions.

---

## Treat payment success as automatic entitlement grant

Rejected.

Entitlement authority remains outside Payments.

---

## Store raw recurring-payment credentials in Subscriptions

Rejected.

Sensitive payment material belongs behind the Payments security boundary.

---

# 115. Relationship to PAY-0006

PAY-0006 and PAY-0007 deliberately share the same Payments execution boundary while preserving different source domains.

```text
             baobab-trade
                  │
                  │ commerce obligation
                  ▼
            ┌──────────────┐
            │              │
            │   PAYMENTS   │
            │              │
            └──────────────┘
                  ▲
                  │ subscription
                  │ billing obligation
                  │
        baobab-subscriptions
```

Trade asks:

```text
How do we collect payment
for this commerce obligation?
```

Subscriptions asks:

```text
How do we collect payment
for this billing obligation?
```

Payments answers both through the same canonical execution domain.

---

# 116. Unified Source Model

Conceptually:

```text
                    PAYMENT SOURCES

        ┌─────────────────────────────┐
        │                             │
        ▼                             ▼
  baobab-trade               baobab-subscriptions
        │                             │
Commerce Obligation           Billing Obligation
        │                             │
        └──────────────┬──────────────┘
                       ▼
                PaymentIntent
                       │
                       ▼
                    Payment
                       │
                       ▼
                 Payment Attempt
                       │
                       ▼
                  HyperSwitch
                       │
                       ▼
                    Provider
```

Payments does not need to become either Commerce or Subscriptions to execute both.

---

# 117. Complete Authority Flow

```text
SUBSCRIPTION POLICY
determines
WHETHER MONEY IS OWED

        │
        ▼

BILLING OBLIGATION
defines
HOW MUCH AND IN WHICH CURRENCY

        │
        ▼

SUBSCRIPTIONS
determines
WHETHER COLLECTION SHOULD OCCUR NOW

        │
        ▼

BAOBAB PAYMENTS
determines
HOW THE PAYMENT CAN BE EXECUTED

        │
        ▼

HYPERSWITCH
orchestrates
THE ELIGIBLE ROUTE

        │
        ▼

PROVIDER
performs
THE EXTERNAL FINANCIAL OPERATION

        │
        ▼

BAOBAB PAYMENTS
records
THE CANONICAL PAYMENT FACT

        │
        ▼

SUBSCRIPTIONS
determines
THE BILLING / COLLECTION / SUBSCRIPTION CONSEQUENCE

        │
        ▼

ERP
determines
THE ACCOUNTING CONSEQUENCE
```

---

# 118. Final Decision

The canonical subscription-payment architecture SHALL be:

```text
Subscription
      │
      ▼
Billing Policy
      │
      ▼
Billing Obligation
      │
      ▼
Payment Required?
      │
      ├──────── NO ────────► Subscription Domain
      │
      └──────── YES
                 │
                 ▼
            PaymentsPort
                 │
                 ▼
            PaymentIntent
                 │
                 ▼
              Payment
                 │
                 ▼
          Payment Execution
                 │
                 ▼
             HyperSwitch
                 │
                 ▼
        Certified Provider
                 │
                 ▼
       Canonical Payment Fact
                 │
                 ▼
           Subscriptions
                 │
                 ▼
       Collection / Dunning /
       Lifecycle Consequence
```

The following distinctions are permanent architectural boundaries:

```text
Subscription
      ≠
Payment

Billing Obligation
      ≠
PaymentIntent

Billing Retry
      ≠
Provider Retry

Dunning
      ≠
Routing Failover

Subscription Credit
      ≠
Payment Refund

Payment Capture
      ≠
Settlement

Payment Success
      ≠
Subscription Activation

Payment Failure
      ≠
Subscription Cancellation
```

**Subscriptions decides why, when and how much money is owed.  
Baobab Payments decides how that authorised monetary obligation is executed and records what happened.  
HyperSwitch orchestrates only eligible payment routes.  
Subscriptions consumes the resulting payment fact and decides the subscription consequence.  
ERP remains authoritative for the accounting consequence.**