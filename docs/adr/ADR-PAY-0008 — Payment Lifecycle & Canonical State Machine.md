# ADR-PAY-0008 — Payment Lifecycle & Canonical State Machine

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Payment Domain / State Machine |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0007; ADR-SHARED-011; applicable Control Plane, Trade, Subscription and Shared contracts |
| **Related** | ADR-PAY-0009; ADR-PAY-0010; ADR-PAY-0011; ADR-PAY-0012; ADR-PAY-0013; ADR-PAY-0014; ADR-PAY-0015; ADR-PAY-0017; ADR-PAY-0018; ADR-PAY-0022 |

---

# 1. Context

ADR-PAY-0002 established the canonical payment-domain distinction:

```text
Payment Obligation
      ≠
PaymentIntent
      ≠
Payment
      ≠
PaymentAttempt
      ≠
Refund
      ≠
Dispute
      ≠
Settlement
```

PAY-0006 and PAY-0007 established two principal payment sources:

```text
baobab-trade
      │
      ├── Commerce Obligation
      │
      ▼
PaymentIntent

baobab-subscriptions
      │
      ├── Billing Obligation
      │
      ▼
PaymentIntent
```

Payment execution then crosses several asynchronous systems:

```text
Originating Engine
       │
       ▼
baobab-payments
       │
       ▼
HyperSwitch
       │
       ▼
Provider / Acquirer / Payment Rail
       │
       ▼
External Financial Network
```

These systems do not necessarily use the same states.

Provider terminology may include concepts such as:

```text
created
pending
requires_action
authorized
captured
charged
processing
succeeded
failed
cancelled
voided
```

HyperSwitch may expose its own operational representation.

Baobab business engines need stable, provider-independent semantics.

Therefore Baobab requires a canonical payment lifecycle owned by `baobab-payments`.

---

# 2. Problem

Without a canonical lifecycle, upstream engines would need to understand:

```text
Provider A states
Provider B states
Provider C states
HyperSwitch states
```

leading to logic such as:

```text
if provider == A and status == "charged":
    ...

if provider == B and status == "settled":
    ...

if provider == C and status == "success":
    ...
```

This would leak payment-provider semantics across the platform.

It would also encourage dangerous equivalences such as:

```text
provider success
      =
commerce order paid
```

or:

```text
payment captured
      =
subscription settled
```

or:

```text
payment captured
      =
ERP reconciled
```

All are architecturally incorrect.

---

# 3. Decision

`baobab-payments` SHALL own a canonical provider-independent payment state model.

The model SHALL distinguish at minimum:

```text
PaymentIntent
Payment
PaymentAttempt
```

with separate lifecycles.

Refund, Dispute, Settlement and Payout SHALL retain independent lifecycles defined by their respective ADRs.

Provider and HyperSwitch states SHALL be translated into canonical Baobab payment facts.

They SHALL NOT become the canonical lifecycle themselves.

---

# 4. Governing Principle

> **Baobab records business-significant payment facts, not provider-specific status vocabulary.**

Therefore:

```text
Provider State
      │
      ▼
Adapter / Normalisation
      │
      ▼
Canonical Baobab Transition
      │
      ▼
Canonical Payment Event
```

---

# 5. Aggregate Relationships

The core execution model is:

```text
Payment Obligation
        │
        ▼
   PaymentIntent
        │
        ▼
      Payment
        │
        ├───────────────┐
        ▼               ▼
PaymentAttempt 1   PaymentAttempt 2
        │               │
        ▼               ▼
   Provider A       Provider B
```

An obligation originates outside Payments.

`PaymentIntent` represents the canonical intention to execute that monetary obligation.

`Payment` represents the canonical payment execution.

`PaymentAttempt` represents a concrete execution attempt through a particular route.

---

# 6. PaymentIntent

A `PaymentIntent` represents:

> An authorised canonical instruction that a monetary payment should be prepared or executed for a defined amount, currency and Baobab context.

It is not proof that payment occurred.

---

# 7. PaymentIntent Identity

A PaymentIntent SHALL have its own Baobab-generated canonical identifier.

For example:

```text
payment_intent_id
```

It SHALL NOT use:

```text
order_id
subscription_id
billing_obligation_id
HyperSwitch payment_id
provider transaction_id
```

as its canonical identity.

Those remain references.

---

# 8. PaymentIntent Core Context

A PaymentIntent SHOULD preserve at least:

```text
payment_intent_id

tenant_id
organisation_id where applicable
legal_entity_id
market_id
digital_estate_id where applicable

source_engine
source_reference

amount
currency

requested payment method/context
capture mode where applicable

correlation_id
idempotency context

status

simulated

created_at
updated_at
```

Additional fields SHALL remain contract-governed.

---

# 9. PaymentIntent Context Stability

Once external execution has begun, core financial context SHALL not be silently mutated.

In particular:

```text
tenant
legal entity
market
amount
currency
```

SHALL be treated as execution-defining context.

A materially changed obligation SHOULD normally require a new PaymentIntent or an explicitly permitted pre-execution amendment.

---

# 10. PaymentIntent State Machine

The canonical PaymentIntent lifecycle SHALL support semantics equivalent to:

```text
CREATED
   │
   ▼
REQUIRES_PAYMENT_METHOD
   │
   ▼
READY
   │
   ▼
PROCESSING
   │
   ├──────────────► REQUIRES_ACTION
   │                     │
   │                     └────► PROCESSING
   │
   ├──────────────► SUCCEEDED
   │
   ├──────────────► FAILED
   │
   └──────────────► CANCELLED
```

Not every PaymentIntent SHALL traverse every state.

---

# 11. PaymentIntent States

## CREATED

The PaymentIntent exists canonically but has not yet satisfied all prerequisites for execution.

---

## REQUIRES_PAYMENT_METHOD

Payment execution requires an eligible payment method or Payment Method Reference.

---

## READY

The PaymentIntent contains sufficient validated context to permit execution.

`READY` does not mean payment has started.

---

## PROCESSING

Payment execution has begun or an external outcome is not yet final.

---

## REQUIRES_ACTION

Further payer/customer action is required.

Examples may include:

```text
authentication
redirect
bank approval
mobile-money approval
additional authorisation
```

---

## SUCCEEDED

The PaymentIntent's required payment objective has been successfully achieved according to its capture semantics.

`SUCCEEDED` SHALL NOT mean settlement or ERP reconciliation.

---

## FAILED

The PaymentIntent can no longer satisfy its intended objective under the current lifecycle without an explicit new recovery/retry transition.

---

## CANCELLED

The PaymentIntent has been intentionally terminated before successful completion.

---

# 12. PaymentIntent Success Semantics

For an automatic-capture PaymentIntent:

```text
SUCCEEDED
```

normally means the required amount has been captured successfully.

For a manual-capture model, the intent MAY remain at an appropriate authorised state/projection until capture completes.

The exact API representation SHALL avoid treating:

```text
AUTHORIZED
```

as equivalent to:

```text
CAPTURED
```

---

# 13. Payment

A `Payment` represents the canonical execution of money collection associated with a PaymentIntent.

It records what happened at the payment-execution layer.

It is not:

```text
an Order
a Subscription
an Invoice
a Settlement
an Accounting Entry
```

---

# 14. Payment Identity

Every Payment SHALL have a Baobab-generated canonical:

```text
payment_id
```

Provider identifiers SHALL remain external references.

---

# 15. Payment Relationship

Conceptually:

```text
PaymentIntent
     │
     ▼
Payment
     │
     ├── PaymentAttempt A
     └── PaymentAttempt B
```

A Payment MAY require more than one attempt while remaining one canonical payment execution.

---

# 16. Payment State Machine

The canonical Payment lifecycle SHALL support states equivalent to:

```text
CREATED
   │
   ▼
PROCESSING
   │
   ├────────────► REQUIRES_ACTION
   │                   │
   │                   └────► PROCESSING
   │
   ├────────────► AUTHORIZED
   │                   │
   │                   ▼
   │             CAPTURE_PENDING
   │                   │
   │                   ▼
   │                CAPTURED
   │
   ├────────────► CAPTURED
   │
   ├────────────► FAILED
   │
   └────────────► CANCELLED
```

Provider-specific intermediate states MAY be internally represented where operationally necessary, but SHALL not destabilise the canonical contract.

---

# 17. CREATED

`CREATED` means a canonical Payment exists but external execution has not yet reached an active provider-processing state.

---

# 18. PROCESSING

`PROCESSING` means execution is underway or the external financial outcome is not yet conclusively known.

This state is important for:

```text
timeouts
asynchronous methods
provider processing
webhook-delayed completion
network uncertainty
```

---

# 19. REQUIRES_ACTION

`REQUIRES_ACTION` means payer/customer participation is required before execution can progress.

The state SHALL be accompanied by a safe canonical action descriptor where appropriate.

---

# 20. AUTHORIZED

`AUTHORIZED` means the payment provider has accepted an authorisation or equivalent reservation of funds according to the payment rail.

It SHALL NOT mean:

```text
funds captured
funds settled
funds reconciled
```

---

# 21. CAPTURE_PENDING

`CAPTURE_PENDING` means capture has been requested but final capture outcome is not yet established.

This distinction is required because:

```text
capture requested
      ≠
capture completed
```

---

# 22. CAPTURED

`CAPTURED` means the payment system has received authoritative evidence that the applicable capture/charge operation succeeded.

It SHALL NOT mean:

```text
SETTLED
```

and SHALL NOT mean:

```text
ERP_RECONCILED
```

---

# 23. FAILED

`FAILED` means the current canonical Payment execution has reached a failure condition according to the lifecycle.

Failure SHALL preserve enough information to determine whether:

```text
a new attempt is permitted
a new payment method is required
a new Payment is required
operator intervention is required
```

---

# 24. CANCELLED

`CANCELLED` means the Payment was intentionally terminated before successful capture.

Cancellation SHALL not be used to represent a refund of already captured funds.

---

# 25. PaymentAttempt

A `PaymentAttempt` represents:

> One concrete attempt to execute a Payment through a specific eligible payment route.

It captures operational execution history without allowing provider retries/failover to overwrite previous facts.

---

# 26. Why PaymentAttempt Is Required

Without PaymentAttempt:

```text
Payment
  provider = A
```

might later be overwritten as:

```text
Payment
  provider = B
```

after failover.

That would destroy the fact that Provider A was attempted.

Instead:

```text
Payment
   │
   ├── Attempt 1
   │      └── Provider A → failed
   │
   └── Attempt 2
          └── Provider B → succeeded
```

preserves history.

---

# 27. PaymentAttempt Identity

Each attempt SHALL have its own:

```text
payment_attempt_id
```

and SHALL reference:

```text
payment_id
provider
merchant/profile context used
provider external reference where available
attempt number or sequence
timestamps
canonical attempt outcome
```

---

# 28. PaymentAttempt State Machine

A canonical attempt lifecycle SHOULD support:

```text
CREATED
   │
   ▼
SUBMITTED
   │
   ▼
PROCESSING
   │
   ├────────► REQUIRES_ACTION
   │               │
   │               └────► PROCESSING
   │
   ├────────► AUTHORIZED
   │
   ├────────► CAPTURED
   │
   ├────────► FAILED
   │
   ├────────► CANCELLED
   │
   └────────► UNKNOWN
```

The precise contract MAY use a smaller vocabulary where equivalent semantics remain available.

---

# 29. UNKNOWN

`UNKNOWN` is a legitimate operational state.

It means:

> An execution attempt was initiated but Baobab cannot yet determine its authoritative external outcome.

This commonly occurs after:

```text
timeout
connection loss
ambiguous provider response
webhook delay
provider outage
```

---

# 30. Unknown Is Not Failed

The following inference is prohibited:

```text
HTTP timeout
     │
     ▼
FAILED
```

because the provider may have processed the payment successfully.

The correct model may be:

```text
HTTP timeout
     │
     ▼
UNKNOWN / PROCESSING
     │
     ▼
Recover External State
```

---

# 31. Unknown State Safety

While an attempt remains `UNKNOWN`, the system SHALL avoid unsafe duplicate execution.

Recovery SHOULD first determine whether the original attempt succeeded.

PAY-0010 defines duplicate safety.

---

# 32. Requested Versus Observed State

The canonical model SHALL distinguish:

```text
REQUEST
```

from:

```text
OBSERVED OUTCOME
```

For example:

```text
capture_requested
      ≠
captured
```

and:

```text
cancel_requested
      ≠
cancelled
```

and:

```text
refund_requested
      ≠
refunded
```

---

# 33. Commands

Representative payment commands include:

```text
CreatePaymentIntent
ProvidePaymentMethod
ExecutePayment
AuthorizePayment
CapturePayment
CancelPayment
```

Commands express requested actions.

They SHALL NOT be stored or emitted as if they prove the outcome occurred.

---

# 34. Facts

Representative canonical facts include:

```text
payment.created
payment.processing
payment.requires_action
payment.authorized
payment.captured
payment.failed
payment.cancelled
```

Facts represent observed canonical outcomes.

---

# 35. Provider State Normalisation

Provider states SHALL pass through an adapter translation layer.

Conceptually:

```text
Provider State
      │
      ▼
HyperSwitch Adapter
      │
      ▼
Canonical Interpretation
      │
      ▼
Payment State Transition
```

No originating engine SHALL be required to interpret provider status vocabulary.

---

# 36. HyperSwitch State Normalisation

HyperSwitch state is also external implementation state.

Therefore:

```text
HyperSwitch Payment State
        ≠
Baobab Payment State
```

even where individual values appear identical.

The adapter SHALL translate semantics rather than merely copy strings.

---

# 37. Unknown Provider State

If a provider or HyperSwitch introduces a state that cannot safely map to a canonical state, Payments SHALL fail conservatively.

It SHALL NOT guess that an unknown state means success.

Operationally:

```text
UNKNOWN EXTERNAL STATE
        │
        ▼
quarantine / processing / investigation
```

as appropriate.

---

# 38. State Transition Authority

Only `baobab-payments` SHALL authoritatively transition canonical Payment state.

Trade may request:

```text
capture
cancel
refund
```

Subscriptions may request:

```text
collection
refund
```

HyperSwitch/provider may report external facts.

But canonical state transition belongs to Payments.

---

# 39. Originating Engine Cannot Set Payment State

An API request such as:

```text
status = CAPTURED
```

from Trade or Subscriptions SHALL NOT authoritatively transition a Payment to CAPTURED.

The caller requests an operation.

Payments observes and records the outcome.

---

# 40. Provider Cannot Set Canonical State Directly

Likewise, raw provider input SHALL not directly mutate canonical state.

It must pass:

```text
authentication / signature verification
context validation
mapping
idempotency
state-transition validation
normalisation
```

before canonical mutation.

PAY-0011 governs webhook processing.

---

# 41. Transition Validation

Every state mutation SHALL be validated against the current canonical state.

For example:

```text
CREATED → CAPTURED
```

MAY be valid for an immediate-capture provider flow if the intermediate states are observationally collapsed.

But:

```text
FAILED → AUTHORIZED
```

SHALL NOT occur merely because a stale webhook arrives.

Transition rules SHALL account for asynchronous observations without accepting impossible regressions.

---

# 42. Monotonic Financial Facts

Certain financial facts SHALL be monotonic.

Once authoritative evidence establishes:

```text
CAPTURED amount X
```

a later stale:

```text
PROCESSING
```

event SHALL NOT erase that capture.

Likewise, later configuration changes SHALL not rewrite historical execution.

---

# 43. State Versus Financial Facts

A single status field SHALL NOT be the only financial history.

Payments SHOULD preserve important facts such as:

```text
authorized amount
captured amount
refunded amount
timestamps
attempts
external references
```

independently enough to maintain auditability.

---

# 44. Partial Capture

Where partial capture is supported, the model SHALL preserve amounts.

Example:

```text
Authorized: 1,000 ZAR
Captured:     600 ZAR
Remaining:    400 ZAR
```

A single boolean:

```text
captured = true
```

would be insufficient.

---

# 45. Multiple Captures

If a provider/payment method supports multiple partial captures, Payments SHALL preserve each execution fact and cumulative totals.

The lifecycle SHALL enforce:

```text
total captured
    ≤
authorised amount
```

unless a specific payment rail explicitly permits another governed semantic.

---

# 46. Capture Mode

PaymentIntent MAY specify a capture mode such as:

```text
AUTOMATIC
MANUAL
```

or equivalent canonical semantics.

Provider-specific capture terminology SHALL remain behind the adapter.

---

# 47. Automatic Capture

Conceptually:

```text
READY
  │
  ▼
PROCESSING
  │
  ▼
AUTHORIZED
  │
  ▼
CAPTURED
```

may occur as one external operation.

Baobab MAY observe only the final `CAPTURED` outcome where the provider does not expose a meaningful intermediate authorisation.

---

# 48. Manual Capture

Conceptually:

```text
Payment
   │
   ▼
AUTHORIZED
   │
   │ later business command
   ▼
Capture Requested
   │
   ▼
CAPTURE_PENDING
   │
   ▼
CAPTURED
```

The authorisation may expire if capture is not performed within provider constraints.

---

# 49. Authorisation Expiry

Payments SHALL be able to represent an authorisation that can no longer be captured.

The exact canonical representation MAY be:

```text
FAILED
CANCELLED
EXPIRED
```

or a dedicated authorisation-expiry fact, depending on the final contract.

The critical semantic is that expired authorisation SHALL NOT be treated as captured funds.

---

# 50. Expiry

Where expiry is a material domain condition, the implementation MAY introduce an explicit:

```text
EXPIRED
```

state.

This SHALL be used only where expiration has distinct business semantics and SHALL not be a synonym for generic failure.

---

# 51. Customer Action

A Payment in `REQUIRES_ACTION` SHALL preserve:

```text
action type
safe continuation information
expiration where relevant
```

without exposing unnecessary provider internals.

---

# 52. Action Descriptor

A canonical action descriptor MAY represent:

```text
REDIRECT
SDK_ACTION
AUTHENTICATION
QR_CODE
MOBILE_APPROVAL
BANK_INSTRUCTION
```

or other supported interaction.

Provider-specific payloads SHALL be encapsulated where necessary.

---

# 53. Action Expiry

If required customer action expires, Payments SHALL transition according to the resulting authoritative payment condition.

It SHALL not remain indefinitely actionable.

---

# 54. Bank Transfer / Delayed Methods

Some methods do not fit immediate card semantics.

Example:

```text
Payment Created
      │
      ▼
Instructions Issued
      │
      ▼
PROCESSING
      │
      ▼
Funds Observed
      │
      ▼
CAPTURED / equivalent successful collection
```

The canonical lifecycle SHALL support delayed methods without pretending instructions equal payment.

---

# 55. Mobile Money

Likewise:

```text
Request Sent
     │
     ▼
REQUIRES_ACTION / PROCESSING
     │
     ▼
Customer Approves
     │
     ▼
Provider Confirms
     │
     ▼
CAPTURED
```

where applicable.

---

# 56. Cancellation

Cancellation is valid only before the payment has entered a state where cancellation semantics no longer apply.

For captured money:

```text
CANCEL
```

SHALL NOT be used as a synonym for:

```text
REFUND
```

---

# 57. Void

Provider terminology may use:

```text
void
```

for reversal of an authorisation before settlement/capture.

Baobab SHALL map that to canonical cancellation/void semantics according to PAY-0013.

It SHALL not confuse it with post-capture Refund.

---

# 58. Refund Boundary

Refund has an independent lifecycle.

Therefore the Payment SHALL not transition:

```text
CAPTURED
    │
    ▼
REFUNDED
```

as though refund simply replaced payment history.

Instead:

```text
Payment
  status = CAPTURED
       │
       ▼
Refund
  status = SUCCEEDED
```

with appropriate aggregate financial projections.

---

# 59. Fully Refunded Projection

Payments MAY expose a derived projection such as:

```text
fully_refunded = true
```

or:

```text
refundable_amount = 0
```

without rewriting the historical fact that the Payment was captured.

---

# 60. Dispute Boundary

A disputed or charged-back payment SHALL not lose its original payment state.

Conceptually:

```text
Payment
  CAPTURED
      │
      ▼
Dispute
  OPEN
```

The dispute lifecycle is separate.

PAY-0014 governs it.

---

# 61. Settlement Boundary

Likewise:

```text
Payment
  CAPTURED
      │
      ▼
Settlement
  OBSERVED
```

rather than:

```text
Payment status = SETTLED
```

being the sole representation of settlement.

PAY-0015 governs settlement.

---

# 62. ERP Boundary

Payments SHALL NOT introduce canonical states such as:

```text
POSTED_TO_GL
REVENUE_RECOGNISED
ACCOUNTING_CLOSED
```

into the Payment state machine.

Those belong to ERP/accounting.

---

# 63. PaymentIntent Versus Payment Success

A PaymentIntent may derive its success from Payment execution.

Conceptually:

```text
Payment
CAPTURED
    │
    ▼
PaymentIntent
SUCCEEDED
```

according to the intent's capture semantics.

The PaymentIntent remains a separate aggregate.

---

# 64. Multiple Payment Attempts

A Payment may require multiple attempts:

```text
Payment
   │
   ├── Attempt 1
   │      Provider A
   │      FAILED
   │
   ├── Attempt 2
   │      Provider B
   │      UNKNOWN
   │
   └── Attempt 3
          Provider B
          CAPTURED
```

PAY-0009 and PAY-0010 SHALL govern whether a subsequent attempt is safe.

---

# 65. Attempt Number Is Not Authority

Attempt sequence is operational history.

It SHALL NOT independently establish which provider is eligible.

Eligibility still comes from PAY-0005 and PAY-0022.

---

# 66. Attempt Route Snapshot

Each PaymentAttempt SHOULD preserve the execution route actually used:

```text
engine_instance_id
merchant reference
business_profile reference
provider / connector reference
payment method context
environment
```

This supports:

```text
audit
incident response
reconciliation
provider analytics
```

---

# 67. Route Changes

If failover changes provider:

```text
Attempt 1 → Provider A
Attempt 2 → Provider B
```

both remain historical facts.

The Payment record SHALL not pretend Provider B was always used.

---

# 68. Retry Versus New Attempt

A network retry of the same idempotent provider operation SHALL not necessarily create a new semantic PaymentAttempt.

Conversely, a deliberate new execution through a different route generally SHOULD.

PAY-0010 SHALL define exact duplicate boundaries.

---

# 69. Recovery Before Retry

When the previous attempt outcome is uncertain:

```text
Attempt 1 = UNKNOWN
```

the system SHOULD recover that attempt before creating another externally executable attempt.

This reduces duplicate charges.

---

# 70. State Transition Persistence

Canonical state changes SHALL be persisted atomically with sufficient metadata to prevent partial local transitions.

Where event publication follows, an outbox or equivalent reliable mechanism SHOULD ensure:

```text
state committed
     │
     ▼
event eventually published
```

without requiring a distributed transaction.

---

# 71. Optimistic Concurrency

Canonical payment aggregates SHOULD use concurrency control sufficient to prevent conflicting transitions.

Conceptually:

```text
payment_id
revision = 12
```

with mutation expecting:

```text
revision = 12
```

before producing:

```text
revision = 13
```

or equivalent database-level concurrency guarantees.

---

# 72. Stale Mutation

If two workers attempt:

```text
CAPTURE
```

and:

```text
CANCEL
```

against the same current state concurrently, only a valid serialised transition SHALL succeed.

The losing operation SHALL re-evaluate current state.

---

# 73. Event Version

Canonical payment events SHOULD contain sufficient version/revision information to allow consumers to detect stale or duplicate observations.

---

# 74. Event Model

Representative events include:

```text
payment.intent.created.v1
payment.intent.ready.v1
payment.intent.requires_action.v1
payment.intent.succeeded.v1
payment.intent.failed.v1
payment.intent.cancelled.v1

payment.created.v1
payment.processing.v1
payment.requires_action.v1
payment.authorized.v1
payment.capture_requested.v1
payment.captured.v1
payment.failed.v1
payment.cancelled.v1

payment.attempt.created.v1
payment.attempt.processing.v1
payment.attempt.failed.v1
payment.attempt.unknown.v1
payment.attempt.succeeded.v1
```

Exact event names SHALL be governed by canonical contracts.

---

# 75. Event Facts Are Immutable

Published canonical financial events SHALL not be rewritten.

Corrections SHALL be expressed through subsequent facts.

---

# 76. At-Least-Once Delivery

Consumers SHALL assume payment events may be delivered:

```text
more than once
out of order
after delay
```

PAY-0011 defines delivery semantics.

---

# 77. Consumer Projection

Trade may project:

```text
payment authorised
payment captured
payment failed
```

into commerce state.

Subscriptions may project the same facts into:

```text
collection pending
paid
collection failed
```

Neither projection changes the canonical Payments state.

---

# 78. Trade Example

```text
Trade Order
     │
     ▼
Payment Collection
     │
     ▼
PaymentIntent
     │
     ▼
Payment
     │
     ▼
AUTHORIZED
     │
     │ Trade requests capture
     ▼
CAPTURE_PENDING
     │
     ▼
CAPTURED
     │
     ▼
payment.captured
     │
     ▼
Trade decides commerce consequence
```

---

# 79. Subscription Example

```text
Billing Obligation
      │
      ▼
PaymentIntent
      │
      ▼
Payment
      │
      ▼
PROCESSING
      │
      ▼
CAPTURED
      │
      ▼
payment.captured
      │
      ▼
Subscriptions marks
collection consequence
```

---

# 80. Requires-Action Example

```text
Payment
   │
   ▼
PROCESSING
   │
   ▼
REQUIRES_ACTION
   │
   ▼
Customer Completes Action
   │
   ▼
Provider Processes
   │
   ▼
CAPTURED
```

The customer-facing application SHALL not mark the payment successful merely because the action UI completed.

---

# 81. Timeout Example

```text
Payment Attempt
      │
      ▼
Provider Request
      │
      X
Network Timeout
      │
      ▼
UNKNOWN
      │
      ▼
Provider State Recovery
      │
      ├── CAPTURED
      ├── FAILED
      └── still PROCESSING
```

No duplicate attempt is automatically created.

---

# 82. Failover Example

```text
Payment
  │
  ├── Attempt 1
  │      Provider A
  │      FAILED: technical
  │
  └── Attempt 2
         Provider B
         CAPTURED
```

The canonical Payment becomes `CAPTURED`.

Both attempts remain recorded.

---

# 83. Decline Example

A definitive customer/payment-method decline may produce:

```text
PaymentAttempt
    FAILED

Payment
    FAILED

PaymentIntent
    REQUIRES_PAYMENT_METHOD
```

where another method may legitimately satisfy the same intent.

The precise transition SHALL depend on canonical error classification.

This illustrates why the three lifecycles SHALL not be collapsed into one status.

---

# 84. Recoverable Versus Terminal

States SHALL be classified according to whether additional transitions remain possible.

Representative model:

| State | Terminal? | Notes |
|---|---:|---|
| CREATED | No | Not yet executed |
| READY | No | Execution permitted |
| PROCESSING | No | Outcome pending |
| REQUIRES_ACTION | No | Customer action required |
| AUTHORIZED | No | Capture/cancel may remain |
| CAPTURE_PENDING | No | Capture outcome pending |
| CAPTURED | Terminal for original collection execution | Refund/dispute/settlement are separate lifecycles |
| FAILED | Context-dependent terminality | Intent may permit new method/new Payment |
| CANCELLED | Yes for that Payment | New PaymentIntent/Payment may be created if business requires |
| UNKNOWN | No | Requires recovery |

---

# 85. Terminal Does Not Mean No Future Financial Activity

`CAPTURED` may be terminal for the original Payment collection lifecycle while later independent operations occur:

```text
Refund
Dispute
Settlement
Chargeback
```

This is why those operations SHALL not be encoded as Payment-state replacements.

---

# 86. Failure Classification

Payments SHOULD distinguish failure categories such as:

```text
CUSTOMER_DECLINE
PAYMENT_METHOD_FAILURE
PROVIDER_TECHNICAL_FAILURE
CONFIGURATION_FAILURE
CONTEXT_FAILURE
AUTHENTICATION_FAILURE
TIMEOUT_OR_UNKNOWN
FRAUD_OR_RISK_REJECTION
```

where safe and supported.

Provider-specific codes remain internal metadata.

---

# 87. Retryability

Canonical failure information SHOULD indicate whether an operation is:

```text
retryable
not retryable
requires new payment method
requires customer action
requires operator intervention
```

without requiring callers to parse provider error strings.

---

# 88. Failure Does Not Rewrite Attempt History

If a later retry succeeds:

```text
Attempt 1 FAILED
Attempt 2 CAPTURED
```

Attempt 1 remains FAILED.

Only the aggregate Payment progresses according to the successful execution.

---

# 89. Idempotent State Transitions

Receiving the same authoritative provider fact twice SHALL not cause two semantic transitions.

For example:

```text
CAPTURED
   +
duplicate CAPTURED webhook
   =
still CAPTURED
```

with duplicate processing safely recognised.

---

# 90. Conflicting Provider Facts

If Baobab receives contradictory external facts:

```text
CAPTURED
then
FAILED
```

it SHALL evaluate:

```text
event identity
provider reference
attempt
provider sequence/version where available
event time
current canonical financial facts
```

and SHALL not blindly apply the latest-arriving message.

---

# 91. Capture Is Monotonic

Once a specific amount has been authoritatively captured, a later failure event SHALL not erase that captured amount.

A subsequent financial reversal belongs to:

```text
refund
reversal
dispute
chargeback
```

semantics.

---

# 92. Simulation

Sandbox execution SHALL use the same canonical lifecycle wherever practical.

However every simulated aggregate/event SHALL carry:

```text
simulated = true
```

A simulated:

```text
CAPTURED
```

is not real money movement.

---

# 93. Simulation Integrity

A Payment SHALL NOT transition from:

```text
simulated = true
```

to:

```text
simulated = false
```

within the same execution lifecycle.

Production execution requires a production payment context.

---

# 94. Historical Integrity

Canonical payment history SHALL retain:

```text
original context
state transitions
attempts
provider references
timestamps
amount facts
correlation
```

according to retention requirements.

Operational cleanup SHALL not destroy financial traceability.

---

# 95. State Audit

Operators SHALL be able to answer:

```text
What state is the Payment in?

How did it reach that state?

Which command initiated the transition?

Which PaymentAttempt executed it?

Which provider was used?

Which external event confirmed it?

Which amount was authorised?

Which amount was captured?

Was it simulated?

Which source obligation initiated it?
```

---

# 96. No Manual Database State Editing

Operators SHALL NOT normally repair Payment state through direct SQL mutation.

Recovery SHALL use controlled administrative commands with:

```text
authorisation
audit
transition validation
reason
correlation
```

PAY-0018 governs controlled mutation.

---

# 97. Reconciliation as State Verification

Reconciliation MAY reveal that canonical state differs from authoritative external financial evidence.

Such discrepancies SHALL trigger a controlled reconciliation/correction workflow.

They SHALL not justify silent database rewriting.

---

# 98. Payment State Query

The Payments API SHALL provide an authoritative way to query current canonical state.

Consumers SHALL not need to query HyperSwitch or the PSP directly.

---

# 99. Projection Versus Source of Truth

Caches, read models and upstream projections MAY expose payment state.

They SHALL be treated as projections.

The authoritative operational payment state remains `baobab-payments`.

---

# 100. Availability

Temporary inability to determine external state SHALL prefer:

```text
PROCESSING
UNKNOWN
```

over incorrectly declaring financial failure.

Correctness takes precedence over prematurely producing a terminal answer.

---

# 101. Observability

Every state transition SHOULD emit telemetry containing appropriate:

```text
correlation_id
payment_intent_id
payment_id
payment_attempt_id
tenant_id
legal_entity_id
market_id
source_engine
source_reference
from_state
to_state
provider where appropriate
```

without exposing sensitive payment data.

---

# 102. Metrics

Useful lifecycle metrics MAY include:

```text
payment intents created
payment intents succeeded
payment intents failed

payments processing
payments requiring action
authorisation success
capture success
payment failure

attempt count per payment
provider failover count
unknown attempt count
unknown-state duration
capture latency
requires-action duration
state-transition rejection count
```

---

# 103. Stuck-State Detection

Payments SHOULD detect aggregates remaining unexpectedly long in:

```text
PROCESSING
REQUIRES_ACTION
CAPTURE_PENDING
UNKNOWN
```

according to method/provider-specific operational thresholds.

Detection SHALL not itself fabricate a financial outcome.

---

# 104. Recovery Workers

Payments MAY operate recovery workers for:

```text
UNKNOWN attempts
long-running PROCESSING payments
pending capture confirmation
webhook gaps
```

Such workers SHALL use the same canonical transition rules as synchronous execution.

---

# 105. Scheduler Boundary

HyperSwitch scheduler capabilities MAY participate in external payment processing.

Baobab SHALL still record the resulting state through the canonical lifecycle.

HyperSwitch scheduler state SHALL not become Baobab's canonical state machine.

---

# 106. Database Model

A conceptual persistence model is:

```text
payment_intents
│
├── payment_intent_id
├── canonical context
├── amount
├── currency
├── status
├── simulated
└── revision

payments
│
├── payment_id
├── payment_intent_id
├── status
├── authorised_amount
├── captured_amount
├── simulated
└── revision

payment_attempts
│
├── payment_attempt_id
├── payment_id
├── provider route
├── status
├── external references
├── submitted_at
├── resolved_at
└── revision
```

This is conceptual rather than a mandated physical schema.

---

# 107. Provider Metadata

Provider-specific data MAY be retained in namespaced operational metadata where necessary.

It SHALL NOT redefine canonical state.

For example:

```text
canonical_status = CAPTURED

provider_metadata = {
    external_status: "...",
    external_reference: "..."
}
```

---

# 108. State Machine Ownership

State-machine definitions SHALL reside within the Payments domain/application implementation.

They SHALL NOT be duplicated independently in:

```text
Trade
Subscriptions
Digital Estates
ERP
Control Plane
```

Those systems consume canonical facts and maintain their own domain projections.

---

# 109. Contract Versioning

Adding a new internal provider state SHALL not necessarily require a canonical contract change.

Adding or changing canonical state semantics MAY require versioned contract evolution.

Provider churn SHALL be absorbed by the adapter whenever possible.

---

# 110. Backward Compatibility

Canonical lifecycle evolution SHALL preserve consumers' ability to interpret existing historical states/events.

A provider upgrade SHALL not silently alter the meaning of:

```text
AUTHORIZED
CAPTURED
FAILED
```

for existing Baobab consumers.

---

# 111. Production Readiness

Representative lifecycle readiness:

```text
PAYMENT LIFECYCLE READINESS

PaymentIntent persistence             PASS
Payment persistence                   PASS
PaymentAttempt persistence            PASS
Canonical transition validation       PASS
Optimistic concurrency                PASS
Amount invariants                     PASS
Provider state normalisation          PASS
HyperSwitch state translation         PASS
Requires-action handling              PASS
Timeout/unknown handling              PASS
Capture semantics                     PASS
Idempotent transition processing      PASS
Canonical events                      PASS
Outbox/reliable publication           PASS
Recovery workflow                     PASS
State reconciliation                  PASS
Audit trail                           PASS
Observability                         PASS
Sandbox/production separation         PASS
------------------------------------------
Canonical Payment Lifecycle           READY
```

---

# 112. Invariants

The following invariants SHALL hold:

1. Payment Obligation is not PaymentIntent.
2. PaymentIntent is not Payment.
3. Payment is not PaymentAttempt.
4. Refund is not Payment state.
5. Dispute is not Payment state.
6. Settlement is not Payment state.
7. Accounting state is not Payment state.
8. Provider state is not canonical Baobab state.
9. HyperSwitch state is not canonical Baobab state.
10. Canonical Payment state is owned by `baobab-payments`.
11. Originating engines request operations; they do not set canonical Payment state.
12. Raw webhooks do not directly mutate canonical state.
13. `AUTHORIZED` does not mean `CAPTURED`.
14. `CAPTURED` does not mean `SETTLED`.
15. `SETTLED` does not mean ERP reconciled.
16. Capture requested does not mean captured.
17. Refund requested does not mean refunded.
18. Cancellation does not mean refund.
19. Timeout does not mean failure.
20. `UNKNOWN` does not mean failure.
21. Customer redirect does not prove success.
22. Provider failover does not erase earlier attempts.
23. Provider references do not become canonical IDs.
24. A duplicate provider fact does not create a duplicate canonical transition.
25. Stale external facts cannot regress established financial facts.
26. Captured amounts cannot be silently erased.
27. Historical attempt routes remain immutable.
28. Core execution context cannot silently change after execution begins.
29. Simulation cannot become production execution.
30. Simulated capture is not real money movement.
31. Terminal Payment state does not prohibit separate Refund, Dispute or Settlement lifecycles.
32. Canonical financial history SHALL remain auditable.
33. Unknown external state fails conservatively.
34. State recovery uses controlled transitions.
35. Direct database mutation is not the normal state-repair mechanism.

---

# 113. Consequences

## Positive

This decision:

- provides stable provider-independent semantics;
- prevents HyperSwitch state leakage;
- supports asynchronous payment methods;
- supports manual and automatic capture;
- supports provider failover;
- preserves individual attempts;
- makes timeout handling safe;
- strengthens duplicate-payment prevention;
- enables reliable Trade and Subscription projections;
- preserves historical financial facts;
- provides a clean basis for Refund, Dispute and Settlement lifecycles;
- improves observability and reconciliation.

## Negative

It introduces:

- multiple related aggregates;
- explicit state machines;
- transition validation;
- concurrency control;
- recovery workflows;
- event versioning;
- additional persistence;
- provider state-normalisation logic.

These costs are accepted because a single provider-derived status field cannot safely represent a multi-provider, asynchronous, multi-engine payment platform.

---

# 114. Alternatives Considered

## Use HyperSwitch status directly

Rejected.

HyperSwitch is an implementation dependency, not Baobab's canonical payment contract.

---

## Use provider status directly

Rejected.

It would couple every consumer to provider-specific semantics.

---

## One `payment_status` field for everything

Rejected.

It cannot correctly represent Intent, Payment, Attempt, Refund, Dispute and Settlement independently.

---

## Treat timeout as failure

Rejected.

It can create duplicate charges.

---

## Treat refund as Payment status

Rejected.

It destroys the historical fact of the original capture and prevents proper partial/multiple refunds.

---

## Treat settlement as Payment status

Rejected.

Capture and settlement are separate financial facts.

---

## Allow business engines to mutate payment status

Rejected.

It destroys Payments' authority over execution truth.

---

# 115. Relationship to Subsequent ADRs

PAY-0008 establishes the state model upon which the remaining execution architecture depends.

```text
PAY-0008
Canonical Lifecycle
      │
      ├──────────────► PAY-0009
      │                Routing / Failover
      │
      ├──────────────► PAY-0010
      │                Idempotency / Duplicate Safety
      │
      ├──────────────► PAY-0011
      │                Webhooks / Events
      │
      ├──────────────► PAY-0013
      │                Refunds / Voids / Reversals
      │
      ├──────────────► PAY-0014
      │                Disputes / Chargebacks
      │
      └──────────────► PAY-0015
                       Settlement / Reconciliation
```

---

# 116. Canonical Lifecycle Overview

```text
                        EXTERNAL OBLIGATION
                               │
                               ▼
                        PAYMENT INTENT
                               │
                  ┌────────────┼─────────────┐
                  │            │             │
                  ▼            ▼             ▼
                READY   REQUIRES_METHOD   CANCELLED
                  │
                  ▼
               PAYMENT
                  │
                  ▼
              PROCESSING
                  │
        ┌─────────┼───────────┐
        │         │           │
        ▼         ▼           ▼
REQUIRES_ACTION AUTHORIZED   FAILED
        │         │
        │         ▼
        │    CAPTURE_PENDING
        │         │
        └────┐    ▼
             └► CAPTURED
                  │
          ┌───────┼────────────┐
          │       │            │
          ▼       ▼            ▼
       REFUND   DISPUTE    SETTLEMENT
      lifecycle lifecycle   lifecycle
```

Refund, Dispute and Settlement are **not** subsequent Payment statuses.

They are separate financial lifecycles associated with the captured Payment.

---

# 117. Canonical Attempt Model

```text
Payment
   │
   ▼
Attempt 1
   │
   ├── Provider A
   ├── PROCESSING
   └── FAILED
            │
            ▼
       Routing Decision
            │
            ▼
         Attempt 2
            │
            ├── Provider B
            ├── PROCESSING
            └── CAPTURED
```

The Payment therefore records:

```text
Payment = CAPTURED
```

while preserving:

```text
Attempt 1 = FAILED
Attempt 2 = CAPTURED
```

No history is overwritten.

---

# 118. Final Decision

Baobab SHALL maintain a provider-independent canonical payment lifecycle centred on:

```text
PaymentIntent
      │
      ▼
Payment
      │
      ▼
PaymentAttempt
```

The core semantic distinctions are:

```text
INTENT
=
what authorised payment objective exists

PAYMENT
=
what happened while executing that objective

ATTEMPT
=
which concrete external route attempted execution
```

Provider and HyperSwitch states are translated into that model:

```text
PROVIDER
    │
    ▼
HYPERSWITCH
    │
    ▼
ADAPTER
    │
    ▼
CANONICAL INTERPRETATION
    │
    ▼
VALIDATED STATE TRANSITION
    │
    ▼
CANONICAL PAYMENT FACT
```

And the following boundaries remain permanent:

```text
AUTHORIZED
     ≠
CAPTURED

CAPTURED
     ≠
SETTLED

SETTLED
     ≠
ERP RECONCILED

TIMEOUT
     ≠
FAILED

CANCELLED
     ≠
REFUNDED

PAYMENT
     ≠
REFUND

PAYMENT
     ≠
DISPUTE

PAYMENT
     ≠
SETTLEMENT
```

**Baobab Payments owns the canonical truth of payment execution.  
HyperSwitch and providers supply external observations.  
Adapters translate those observations.  
The canonical state machine determines what they mean to Baobab.  
Historical financial facts are never rewritten merely to simplify current state.**