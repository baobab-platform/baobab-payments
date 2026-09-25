# ADR-PAY-0002 — Payment Domain Model & Authority

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Domain Model / Authority |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001; ADR-SHARED-011 |
| **Related** | ADR-PAY-0003 through ADR-PAY-0022; applicable Control Plane, Trade, Subscriptions, ERP, IAM and Shared ADRs |

---

## 1. Context

ADR-PAY-0001 establishes `baobab-payments` as Baobab's independently deployable, headless payment-orchestration engine and adopts HyperSwitch as its foundational orchestration implementation.

That decision establishes the engine boundary but does not, by itself, completely define the canonical Baobab payment domain.

Baobab must support multiple:

- tenants;
- organisations;
- legal entities;
- digital estates;
- markets;
- currencies;
- payment methods;
- commercial engines;
- payment providers;
- processors;
- settlement arrangements;
- and future external customers.

The payment domain must therefore remain independent of any single:

- provider;
- processor;
- HyperSwitch object model;
- commerce engine;
- billing engine;
- ERP;
- digital estate;
- legal entity;
- or market.

The canonical domain also needs to distinguish several concepts that are commonly, but incorrectly, collapsed into the generic term **payment**.

A commercial obligation, a payment instruction, execution of that instruction, movement of settlement funds, a refund, a chargeback, a payout and an accounting entry are related but fundamentally different facts.

Baobab therefore requires an explicit domain model and authority map before defining provider mappings, lifecycle transitions, routing, reconciliation and other downstream payment behaviour.

---

# 2. Decision

Baobab SHALL maintain a provider-independent canonical payment domain owned by `baobab-payments`.

The core domain SHALL distinguish at minimum:

```text
PaymentIntent
Payment
Refund
```

and SHALL preserve explicit conceptual boundaries for:

```text
Payment Obligation
Payment Attempt
Payment Method Reference
Provider Execution
Refund
Reversal
Dispute / Chargeback
Settlement
Payout
Accounting Consequence
```

Not every concept must initially become an independently persisted aggregate.

However, the architecture SHALL NOT collapse concepts with different authorities or lifecycle semantics into a generic `Payment` object.

The governing principle is:

> **Baobab Payments owns payment execution and its canonical operational truth. It does not own the commercial obligation that caused payment, the accounting consequences of payment, or the canonical business context within which payment occurs.**

---

# 3. Domain Boundary

The payment domain begins when another authorised Baobab engine requires money movement or payment-method interaction.

It ends at the authoritative operational facts concerning payment execution.

Conceptually:

```text
Commercial Domain
     │
     │ "money is owed / payment is required"
     ▼
┌───────────────────────────────────────────┐
│           BAOBAB PAYMENTS                 │
│                                           │
│  PaymentIntent                            │
│       │                                   │
│       ▼                                   │
│  Payment Execution                        │
│       │                                   │
│       ├── Authorisation                   │
│       ├── Capture                         │
│       ├── Cancellation / Void             │
│       ├── Failure                         │
│       └── Refund                          │
│                                           │
│  Provider orchestration                   │
│  Provider references                      │
│  Payment operational state                │
└───────────────────────────────────────────┘
     │
     ├──────────────► Settlement evidence
     │
     ├──────────────► Reconciliation evidence
     │
     └──────────────► Canonical payment events
                              │
                              ▼
                         Baobab ERP
                              │
                              ▼
                    Accounting Consequence
```

The Payments engine SHALL NOT infer or originate the commercial obligation.

---

# 4. Authority Model

Baobab SHALL explicitly separate domain authority.

| Domain Fact | Authoritative Engine |
|---|---|
| Tenant | Control Plane |
| Organisation | Control Plane |
| Legal Entity | Control Plane |
| Digital Estate | Control Plane |
| Market | Control Plane |
| Engine / Engine Instance | Control Plane |
| Capability Binding | Control Plane |
| Isolation Profile | Control Plane |
| Customer commerce identity | Trade / applicable business engine |
| Order | Trade |
| Order pricing | Trade |
| Inventory | Trade |
| Subscription | Subscriptions |
| Billing obligation | Subscriptions / originating commercial engine |
| Invoice accounting | ERP |
| Payment Intent | Payments |
| Payment execution | Payments |
| Payment operational state | Payments |
| Provider routing | Payments / HyperSwitch within Baobab constraints |
| Provider reference | Payments |
| Refund execution | Payments |
| Settlement observation | Payments |
| Settlement accounting | ERP |
| Receivables | ERP |
| Revenue recognition | ERP |
| General ledger | ERP |
| Enterprise identity | IAM |
| Payment-provider certification | Payments governance |
| Provider market activation | Payments + governed Control Plane context |

An engine SHALL consume authoritative identifiers rather than silently recreate their underlying entities.

---

# 5. Canonical Aggregate Model

The initial canonical payment model SHALL centre on:

```text
PaymentIntent
     │
     │ 1
     │
     ├───────────────┐
     │               │
     ▼               ▼
 Payment          Payment
     │
     ├───────────────┐
     │               │
     ▼               ▼
  Refund          Refund
```

The exact cardinality MAY evolve according to PAY-0008 and implementation requirements.

The important invariant is:

```text
PaymentIntent ≠ Payment
Payment ≠ Refund
```

These objects represent different domain facts.

---

# 6. Payment Obligation

A **Payment Obligation** answers:

> Why is money owed?

Examples include:

```text
commerce order
subscription invoice
service charge
future marketplace obligation
other commercial obligation
```

The Payment Obligation SHALL NOT be owned by `baobab-payments`.

For example:

```text
Trade
  │
  └── Order requires payment

Subscriptions
  │
  └── Billing obligation requires payment
```

Payments receives a reference to that obligation.

It does not recreate the obligation.

Therefore:

```text
Payment Obligation
        │
        │ source reference
        ▼
PaymentIntent
```

The originating engine remains authoritative for:

```text
why money is owed
how much the commercial obligation is
what goods/services caused it
commercial tax treatment
commercial discounts
billing period
order lifecycle
subscription lifecycle
```

---

# 7. PaymentIntent

A **PaymentIntent** represents Baobab's canonical instruction that payment execution is required or being prepared for a defined amount, currency and business context.

It answers:

> What payment execution is Baobab attempting to accomplish?

Conceptually:

```text
PaymentIntent
│
├── payment_intent_id
├── tenant_id
├── legal_entity_id
├── market_id
├── digital_estate_id        optional where applicable
│
├── source_engine
├── source_reference
│
├── amount
├── currency
│
├── payment_method_context
├── capture_mode
│
├── correlation_id
├── idempotency_reference
│
├── status
├── simulated
│
├── created_at
└── updated_at
```

The exact contract SHALL evolve through the relevant Shared payment schema and later Payments ADRs.

A PaymentIntent SHALL NOT contain copied:

```text
customer master records
order line items
subscription definitions
product catalogue data
tax ledgers
accounting entries
```

unless a later ADR identifies a narrowly justified immutable payment snapshot.

---

# 8. Payment

A **Payment** represents canonical Baobab knowledge about execution of a PaymentIntent.

It answers:

> What happened when Baobab attempted to execute this payment?

Conceptually:

```text
Payment
│
├── payment_id
├── payment_intent_id
│
├── amount
├── currency
│
├── status
│
├── provider_reference
├── processor_reference
│
├── authorised_amount
├── captured_amount
│
├── simulated
│
├── correlation_id
│
├── created_at
└── updated_at
```

The Payment SHALL remain provider-independent.

Provider-specific fields SHALL NOT become first-class canonical fields merely because a particular PSP exposes them.

Where provider-specific metadata must be retained, it SHALL remain:

```text
opaque
namespaced
or mapped through provider adapters
```

rather than redefining the canonical domain.

---

# 9. Payment Attempt

A **Payment Attempt** represents one concrete attempt to execute some or all of a PaymentIntent through an eligible provider route.

Conceptually:

```text
PaymentIntent
      │
      ▼
 Payment Attempt #1
      │
      └── Provider A
             │
             └── timeout
      │
      ▼
 Payment Attempt #2
      │
      └── Provider B
             │
             └── authorised
```

Payment attempts are operational execution facts.

They are particularly important for:

- retries;
- routing;
- failover;
- timeout handling;
- duplicate prevention;
- provider telemetry;
- incident reconstruction.

The exact persistence and aggregate relationship SHALL be finalised by PAY-0008, PAY-0009 and PAY-0010.

A retry SHALL NOT silently overwrite the previous attempt.

---

# 10. Payment Method

A **Payment Method** describes how the payer intends to satisfy the payment.

Canonical categories MAY include:

```text
CARD
BANK_TRANSFER
ACCOUNT_TO_ACCOUNT
MOBILE_MONEY
WALLET
DIRECT_DEBIT
BUY_NOW_PAY_LATER
OTHER
```

The canonical vocabulary SHALL belong to Baobab contracts.

Provider-specific payment-method codes SHALL be translated at the Payments boundary.

---

# 11. Payment Method Reference

Baobab SHALL distinguish a payment method from sensitive payment credentials.

A Payment Method Reference MAY represent:

```text
token reference
vault reference
provider payment-method reference
network token reference
bank mandate reference
mobile-money reference
```

It SHALL NOT imply that Baobab owns or stores underlying sensitive payment credentials.

The exact PCI, tokenisation and vaulting boundary is governed by PAY-0012.

---

# 12. Provider Execution

Provider execution is an implementation-level interaction between Payments and an external payment provider.

Conceptually:

```text
Canonical PaymentIntent
          │
          ▼
Baobab Payment Provider Port
          │
          ▼
HyperSwitch Adapter
          │
          ▼
HyperSwitch
          │
          ▼
Provider Connector
          │
          ▼
PSP / Acquirer / Payment Rail
```

Provider objects SHALL NOT escape upward as Baobab canonical domain objects.

---

# 13. Provider References

Provider identifiers SHALL be treated as external references.

Examples include:

```text
HyperSwitch payment ID
processor payment ID
acquirer reference
merchant transaction ID
refund reference
network reference
```

They SHALL NOT replace canonical Baobab identifiers.

Therefore:

```text
payment_id
```

remains a Baobab identifier even where:

```text
provider_payment_id
```

also exists.

---

# 14. Refund

A **Refund** represents an instruction and resulting execution to return previously captured value.

It answers:

> What amount from a previous payment is being returned?

Conceptually:

```text
Payment
   │
   ├──────────► Refund A
   │
   └──────────► Refund B
```

A refund SHALL have its own identity and lifecycle.

A Payment SHALL NOT simply transition from:

```text
CAPTURED
```

to:

```text
REFUNDED
```

without preserving the refund operation as a distinct domain fact.

This is necessary because:

```text
one payment
```

may have:

```text
multiple partial refunds
```

and because refund execution itself may:

```text
be pending
fail
require retry
complete asynchronously
```

PAY-0013 SHALL define refund, void and reversal semantics in detail.

---

# 15. Void / Cancellation

A **Void** or payment cancellation prevents or reverses an execution before final capture according to provider and payment-method semantics.

It SHALL NOT be treated as synonymous with a Refund.

Conceptually:

```text
AUTHORISED
     │
     ├────────► CAPTURE
     │
     └────────► VOID
```

versus:

```text
CAPTURED
     │
     └────────► REFUND
```

Exact semantics are governed by PAY-0013.

---

# 16. Reversal

A **Reversal** is not automatically equivalent to either:

```text
refund
```

or:

```text
void
```

Some providers or payment rails may use reversal terminology for correcting or undoing prior processing.

Baobab SHALL translate provider-specific reversal semantics into the correct canonical operation.

Where a distinct canonical reversal concept is necessary, PAY-0013 SHALL define it.

Provider terminology SHALL NOT dictate the Baobab domain model.

---

# 17. Dispute

A **Dispute** represents a challenge to a payment after or during payment processing according to the applicable payment rail.

It is not:

```text
a refund
```

and it is not:

```text
a failed payment
```

Conceptually:

```text
Payment
   │
   ▼
Captured
   │
   ▼
Dispute
   │
   ├── opened
   ├── evidence required
   ├── under review
   └── resolved
```

PAY-0014 SHALL define dispute semantics.

---

# 18. Chargeback

A **Chargeback** represents a financial consequence associated with a dispute or payment-rail process in which funds are reclaimed through the provider/acquirer/network mechanism.

A chargeback SHALL NOT be modelled as an ordinary merchant-initiated Refund.

The distinction is important for:

```text
authority
financial reconciliation
fees
evidence
timelines
ERP treatment
risk analysis
```

PAY-0014 SHALL define the detailed model.

---

# 19. Settlement

A **Settlement** represents provider-side financial movement or reporting concerning amounts due between relevant payment participants after payment processing.

Settlement answers:

> What funds did the payment provider actually settle, or report as settled, and under what references?

Settlement SHALL remain distinct from Payment.

A captured Payment does not prove settlement.

Therefore:

```text
PAYMENT CAPTURED
        │
        ▼
provider processing
        │
        ▼
SETTLEMENT OBSERVED
        │
        ▼
ERP RECONCILIATION
```

The following are explicitly prohibited equivalences:

```text
CAPTURED = SETTLED
```

and:

```text
provider says settled = accounting reconciled
```

PAY-0015 SHALL define settlement and reconciliation.

---

# 20. Accounting Consequence

A successful payment produces information relevant to accounting.

It does not make `baobab-payments` the accounting authority.

Payments MAY emit facts such as:

```text
payment captured
refund completed
settlement observed
processor fee reported
chargeback observed
```

ERP determines their accounting consequences.

Therefore:

```text
Payment Event
     │
     ▼
Baobab ERP
     │
     ▼
Accounting Policy
     │
     ▼
Ledger Entry
```

Payments SHALL NOT create authoritative:

```text
general-ledger postings
revenue-recognition records
accounts-receivable balances
financial statements
```

---

# 21. Payout

A **Payout** represents movement of money from an authorised funding context toward a beneficiary.

It is fundamentally different from customer payment collection.

Conceptually:

```text
COLLECTION

Customer
   │
   ▼
Merchant / Legal Entity
```

versus:

```text
PAYOUT

Legal Entity / Funding Source
   │
   ▼
Beneficiary
```

Payout SHALL therefore have independent:

```text
authority
beneficiary
eligibility
approval
risk
lifecycle
routing
reconciliation
```

semantics.

PAY-0016 SHALL define the canonical payout model.

---

# 22. Treasury Transfer

A treasury or internal fund transfer SHALL NOT automatically be modelled as either:

```text
Payment
```

or:

```text
Payout
```

The future architecture MAY require a dedicated treasury or financial-transfer domain.

PAY-0002 intentionally does not assign that responsibility to `baobab-payments`.

A separate ADR SHALL be required before Payments assumes treasury authority.

---

# 23. Money

All monetary values SHALL be represented explicitly by:

```text
amount
currency
```

The architecture SHALL avoid implicit currency.

Conceptually:

```text
Money
│
├── amount
└── currency
```

Implementation SHALL use representations appropriate for exact monetary arithmetic.

Binary floating-point SHALL NOT be used for authoritative monetary values.

---

# 24. Currency Authority

Payments consumes canonical currency context but does not own global currency definitions.

Payments SHALL validate whether a currency is eligible for the:

```text
market
provider
payment method
legal entity
transaction type
```

according to payment policy and provider activation.

Currency eligibility is governed further by PAY-0005, PAY-0017 and PAY-0022.

---

# 25. Context

Every payment operation SHALL execute within resolved Baobab context.

At minimum, context SHALL be capable of establishing:

```text
tenant
legal entity
market
source engine
source reference
correlation
```

and, where applicable:

```text
organisation
digital estate
engine instance
isolation profile
```

The Payments engine SHALL consume this context.

It SHALL NOT independently derive corporate hierarchy from HyperSwitch objects.

---

# 26. Source Reference

Every PaymentIntent SHALL be traceable to the authoritative source that requested payment.

Conceptually:

```text
source_engine
source_reference
```

Examples:

```text
baobab-trade / order-123

baobab-subscriptions / invoice-456

future-engine / obligation-789
```

Payments SHALL NOT require every originating engine to share the same commercial aggregate model.

The source reference creates traceability without coupling Payments to source-engine internals.

---

# 27. Correlation

Payment operations SHALL preserve correlation across engine boundaries.

A trace SHOULD be reconstructable conceptually as:

```text
Commercial Obligation
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
       │
       ▼
Settlement
       │
       ▼
ERP Reconciliation
```

Correlation architecture is defined further by PAY-0010 and PAY-0019.

---

# 28. Idempotency

Domain identity and idempotency SHALL remain distinct.

For example:

```text
payment_intent_id
```

answers:

> Which canonical payment intent is this?

while:

```text
idempotency_key
```

answers:

> Have I already processed this logically identical command?

An idempotency key SHALL NOT become the canonical aggregate identifier.

PAY-0010 SHALL define exact semantics.

---

# 29. Canonical Identity

Baobab-generated identifiers SHALL be authoritative for Baobab payment aggregates.

Examples:

```text
payment_intent_id
payment_id
refund_id
dispute_id
settlement_id
payout_id
```

where those aggregates exist.

External identifiers SHALL be represented separately.

Therefore:

```text
CanonicalEntity
       │
       └── ExternalReference
```

is preferred over:

```text
provider identifier = canonical identifier
```

---

# 30. HyperSwitch Boundary

HyperSwitch SHALL remain an implementation behind the Baobab payment domain.

Therefore:

```text
Baobab PaymentIntent
        ≠
HyperSwitch PaymentIntent
```

even where an adapter maps one to the other.

Similarly:

```text
Baobab Payment
        ≠
HyperSwitch Payment object

Baobab Refund
        ≠
HyperSwitch Refund object
```

The mapping may be close.

The authority remains separate.

---

# 31. Domain Translation

The provider adapter layer SHALL perform translation:

```text
Baobab Domain
      │
      ▼
PaymentProvider Port
      │
      ▼
HyperSwitch Adapter
      │
      ▼
HyperSwitch Domain
      │
      ▼
Provider Domain
```

and return normalised results:

```text
Provider Result
      │
      ▼
HyperSwitch
      │
      ▼
Adapter
      │
      ▼
Baobab Canonical Result
```

Provider-specific behaviour SHALL be contained as close to this boundary as practical.

---

# 32. Payment Lifecycle

PAY-0002 establishes the distinction between aggregates but does not define every lifecycle transition.

PAY-0008 SHALL define the canonical state machine.

At a high level, Payments must be able to represent execution outcomes such as:

```text
CREATED
   │
   ▼
REQUIRES_ACTION
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

with applicable alternate paths such as:

```text
FAILED
CANCELLED
VOIDED
```

Refund, dispute, settlement and payout lifecycles SHALL remain separate.

---

# 33. Simulated Payments

A simulated payment is still a useful payment-domain record but SHALL never be indistinguishable from real money movement.

Every applicable aggregate and event SHALL preserve:

```text
simulated = true | false
```

A sandbox result SHALL never be promoted into production financial evidence.

Therefore:

```text
simulated payment
        ≠
real payment
```

and:

```text
simulated capture
        ≠
real settlement
```

---

# 34. Events

Canonical payment events SHALL describe Baobab domain facts rather than raw provider callbacks.

Representative events include:

```text
payment.created
payment.requires_action
payment.authorized
payment.capture_requested
payment.captured
payment.failed
payment.cancelled

refund.requested
refund.completed
refund.failed

dispute.opened
dispute.updated

settlement.observed
```

Exact names, versions and schemas SHALL be governed by Shared contracts and later ADRs.

Provider webhook payloads SHALL NOT themselves become canonical platform events.

---

# 35. Commands Versus Events

Baobab SHALL distinguish:

```text
COMMAND
```

from:

```text
EVENT
```

A command requests something to happen:

```text
CreatePaymentIntent
AuthorizePayment
CapturePayment
CancelPayment
CreateRefund
```

An event records that something happened:

```text
PaymentCreated
PaymentAuthorized
PaymentCaptured
PaymentFailed
RefundCompleted
```

A request to capture money SHALL never be represented as proof that money was captured.

---

# 36. Requested State Versus Observed State

Payments SHALL distinguish requested actions from observed outcomes.

For example:

```text
Capture Requested
       │
       ▼
Provider Processing
       │
       ├────────► Capture Failed
       │
       └────────► Payment Captured
```

Similarly:

```text
Refund Requested
       │
       ▼
Provider Processing
       │
       ├────────► Refund Failed
       │
       └────────► Refund Completed
```

This distinction is mandatory for asynchronous providers.

---

# 37. Eventual Consistency

Baobab SHALL assume that external payment systems may behave asynchronously.

Therefore:

```text
API response
```

does not necessarily represent final financial state.

Authoritative payment state may evolve through:

```text
synchronous provider response
webhook
polling
reconciliation
settlement report
manual governed correction
```

Later information SHALL be incorporated according to PAY-0008, PAY-0010, PAY-0011 and PAY-0015 without destroying historical evidence.

---

# 38. Historical Integrity

Financial-domain history SHALL be append-conscious and auditable.

The architecture SHALL avoid rewriting historical facts merely to represent the latest state.

For example:

```text
Payment authorised
Payment capture requested
Payment captured
Refund requested
Refund completed
```

represents a meaningful history.

Reducing this to:

```text
status = REFUNDED
```

is insufficient as the sole financial history.

Current-state projections MAY exist for efficient reads.

They SHALL remain reconstructable or explainable from authoritative state transitions and evidence.

---

# 39. No Shared Database Authority

Other engines SHALL NOT access the `baobab-payments` database directly.

Likewise, Payments SHALL NOT query:

```text
Trade tables
Subscription tables
ERP tables
Control Plane tables
IAM tables
```

to establish authority.

Cross-engine communication SHALL occur through:

```text
versioned APIs
canonical contracts
events
governed projections
```

according to Baobab architecture.

---

# 40. Persistence Boundary

`baobab-payments` SHALL own its own operational persistence.

This includes canonical records necessary for payment execution and recovery.

HyperSwitch persistence SHALL remain separate.

Conceptually:

```text
Baobab Payments DB
        │
        │ mapping / external reference
        ▼
HyperSwitch
        │
        ▼
HyperSwitch DB
```

No Baobab engine SHALL depend directly upon HyperSwitch database schemas.

---

# 41. Customer Boundary

Payments SHALL NOT become the canonical customer system.

Payments MAY consume:

```text
customer reference
payer reference
billing contact reference
payment-method reference
```

where necessary.

The authoritative customer or trading-party identity remains in the appropriate business domain.

Provider customer identifiers SHALL be external references.

---

# 42. Order Boundary

Payments SHALL NOT own:

```text
cart
order
order line
catalogue
inventory
pricing
discount
promotion
fulfilment
shipment
```

Those remain Trade concerns.

A PaymentIntent may reference an Order.

It SHALL NOT recreate the Order aggregate.

---

# 43. Subscription Boundary

Payments SHALL NOT determine:

```text
subscription plan
billing period
usage charge
credit entitlement
renewal
subscription lifecycle
whether money is owed
```

Subscriptions determines those facts.

Payments executes authorised payment obligations.

Therefore:

```text
Subscriptions
      │
      │ payment required
      ▼
Payments
```

not:

```text
Payments
      │
      │ decide billing
      ▼
Subscriptions
```

---

# 44. INTERNAL Subscription Invariant

Where the authoritative billing policy establishes:

```text
subscriptionType = INTERNAL
```

and:

```text
monetary charge = zero
payment required = false
```

`baobab-payments` SHALL NOT be invoked merely to simulate a zero-value payment.

This preserves the existing Shared contract boundary.

Zero monetary obligation is not equivalent to successful payment execution.

---

# 45. ERP Boundary

Payments reports operational financial facts.

ERP interprets their accounting consequences.

Therefore:

```text
Payments:
"ZAR 1,000 captured"

ERP:
"How is this represented in receivables,
cash clearing, revenue, fees and ledger?"
```

The second question does not belong to Payments.

---

# 46. IAM Boundary

Payments SHALL consume authenticated workload and administrative identity.

Payments SHALL NOT become:

```text
identity provider
user directory
role authority
authentication authority
```

PAY-0018 SHALL define payment-specific authorisation and controlled mutation.

---

# 47. Control Plane Boundary

Control Plane resolves the business context in which payment execution is permitted.

Payments SHALL NOT infer:

```text
tenant
organisation
legal entity
market
digital estate
engine instance
```

from provider configuration.

The direction is:

```text
Control Plane Context
        │
        ▼
Payments
        │
        ▼
Provider Mapping
        │
        ▼
HyperSwitch
```

not the reverse.

---

# 48. Legal Entity Boundary

The legal entity is fundamental to payment execution because it may determine:

```text
merchant contract
provider credentials
settlement account
regulatory obligations
market eligibility
tax consequences
reconciliation
```

A PaymentIntent SHALL therefore be associated with authoritative legal-entity context.

Payments SHALL consume that identity rather than create it.

---

# 49. Market Boundary

Market SHALL also remain canonical Control Plane context.

Payment provider profiles or merchant accounts MAY map to markets.

They SHALL NOT define them.

PAY-0005 defines detailed market/currency/payment-method semantics.

---

# 50. Digital Estate Boundary

A Digital Estate may originate or facilitate payment activity.

It does not own payment execution authority.

For example:

```text
ZuriBeans Digital Estate
       │
       ▼
Trade
       │
       ▼
Payments
```

The frontend SHALL NOT directly become authoritative for:

```text
amount
currency
provider
merchant
routing
capture result
```

Client-supplied payment information SHALL be treated as input requiring server-side authority and validation.

---

# 51. Provider Boundary

Payment providers are external execution dependencies.

They SHALL NOT determine Baobab canonical business context.

Provider metadata MAY be operationally important.

It remains subordinate to canonical Baobab authority.

---

# 52. Cross-Border Boundary

Cross-border payment execution may introduce:

```text
source currency
destination currency
FX
cross-border fees
correspondent institutions
local acquiring
settlement currency
regulatory constraints
```

PAY-0017 SHALL define those semantics.

PAY-0002 establishes only that cross-border concerns SHALL NOT mutate the core authority model.

---

# 53. Domain Invariants

The following invariants SHALL hold.

1. A PaymentIntent is not a commercial obligation.
2. A PaymentIntent is not a Payment.
3. A Payment is not a Settlement.
4. A Payment is not an accounting entry.
5. A Refund is not a Payment state shortcut.
6. A Refund is not a Chargeback.
7. A Void is not a Refund.
8. A Dispute is not a Refund.
9. A Chargeback is not an ordinary merchant refund.
10. A Settlement is not proof of accounting reconciliation.
11. A Payout is not customer payment collection.
12. A provider reference is not a canonical Baobab identifier.
13. A HyperSwitch object is not a Baobab canonical object.
14. A payment method is not sensitive payment credential data.
15. A source reference is not a copied commercial aggregate.
16. A sandbox transaction is not real money movement.
17. A capture request is not proof of capture.
18. A refund request is not proof of refund.
19. Payments does not own tenants.
20. Payments does not own organisations.
21. Payments does not own legal entities.
22. Payments does not own markets.
23. Payments does not own orders.
24. Payments does not own subscriptions.
25. Payments does not own the general ledger.
26. Payments does not own enterprise identity.
27. Payments owns canonical payment execution state.
28. Provider-specific terminology SHALL NOT silently redefine the Baobab payment domain.

---

# 54. Aggregate Authority Summary

```text
                         BAOBAB PLATFORM

 ┌─────────────────┐
 │  Control Plane  │
 │                 │
 │ Tenant          │
 │ Organisation    │
 │ Legal Entity    │
 │ Market          │
 │ Digital Estate  │
 └────────┬────────┘
          │ context
          ▼
 ┌─────────────────┐
 │ Trade / Billing │
 │                 │
 │ Commercial      │
 │ Obligation      │
 └────────┬────────┘
          │ request payment
          ▼
 ┌─────────────────────────────────┐
 │        BAOBAB PAYMENTS          │
 │                                 │
 │ PaymentIntent                   │
 │ Payment                         │
 │ Payment Attempt                 │
 │ Refund                          │
 │ Payment execution state         │
 │ Provider orchestration          │
 └───────────────┬─────────────────┘
                 │
                 ▼
          ┌─────────────┐
          │ HyperSwitch │
          └──────┬──────┘
                 │
                 ▼
          ┌─────────────┐
          │  Provider   │
          └──────┬──────┘
                 │
                 ▼
           financial facts
                 │
                 ▼
          ┌─────────────┐
          │ Baobab ERP  │
          │             │
          │ Accounting  │
          │ Ledger      │
          │ Receivables │
          └─────────────┘
```

---

# 55. Domain Extension Rule

Future payment concepts SHALL be introduced according to their actual authority and lifecycle rather than forced into existing aggregates.

Before adding a new financial concept, architecture review SHALL ask:

```text
What real-world fact does this represent?

Who is authoritative for it?

Does it have an independent lifecycle?

Can it occur more than once?

Can it fail independently?

Does it have independent reconciliation consequences?

Does it cross a legal-entity boundary?

Does it move money or merely describe money?

Does it require independent audit history?
```

Where those answers indicate independent semantics, a separate domain object or aggregate SHOULD be preferred over overloaded Payment fields.

---

# 56. Consequences

## Positive

This decision:

- establishes a stable Baobab payment vocabulary;
- prevents HyperSwitch implementation leakage;
- preserves engine authority boundaries;
- supports multiple commerce and billing engines;
- supports multiple providers;
- supports independent legal entities;
- supports multi-market operations;
- makes retries and failover auditable;
- separates execution from settlement;
- separates payment from accounting;
- separates refunds from chargebacks;
- provides a foundation for cross-border payments;
- supports future external Baobab customers;
- reduces provider lock-in;
- makes future payment ADRs composable.

## Negative

The model is more explicit than simply adopting HyperSwitch objects directly.

It requires:

- mapping layers;
- canonical identifiers;
- additional persistence;
- event translation;
- lifecycle modelling;
- external-reference management;
- reconciliation architecture;
- stronger cross-engine contracts.

These costs are accepted because payment systems require durable authority boundaries and financial traceability.

---

# 57. Alternatives Considered

## Use HyperSwitch's domain model directly

Rejected.

It would make Baobab's canonical payment contracts dependent on an implementation technology and complicate future provider-orchestrator replacement or extension.

---

## Let Medusa own the canonical payment domain

Rejected.

Medusa is the commerce engine and is not the platform-wide payment authority.

Subscriptions and future Baobab engines also require payment execution.

---

## Let ERP own payment execution

Rejected.

ERP remains authoritative for accounting consequences and financial records, not payment-provider orchestration.

---

## Model every financial movement as Payment

Rejected.

Payment, Refund, Settlement, Chargeback, Payout and future treasury transfers have materially different authorities and lifecycles.

---

## Store only provider state

Rejected.

Baobab requires canonical state, idempotency, correlation, multi-provider routing and independence from provider-specific identifiers.

---

## Allow each digital estate to integrate directly with providers

Rejected.

That would fragment payment authority, duplicate credentials and provider logic, weaken auditability and bypass Baobab's multi-tenant payment architecture.

---

# 58. Relationship to Subsequent ADRs

This ADR establishes the vocabulary and authority boundaries upon which the remaining Payments ADRs depend.

```text
PAY-0002  DOMAIN MODEL & AUTHORITY
     │
     ├── PAY-0003 Tenant / Organisation Mapping
     ├── PAY-0004 Merchant / Legal Entity / Profile Mapping
     ├── PAY-0005 Market / Currency / Payment Method
     ├── PAY-0006 Trade Integration
     ├── PAY-0007 Subscription Integration
     ├── PAY-0008 Lifecycle
     ├── PAY-0009 Routing
     ├── PAY-0010 Idempotency
     ├── PAY-0011 Webhooks
     ├── PAY-0012 PCI / Tokenisation
     ├── PAY-0013 Refunds / Voids / Reversals
     ├── PAY-0014 Disputes / Chargebacks
     ├── PAY-0015 Settlement / ERP
     ├── PAY-0016 Payouts
     ├── PAY-0017 FX / Cross-Border
     ├── PAY-0018 Security / IAM
     ├── PAY-0019 Observability
     ├── PAY-0020 Resilience / DR
     ├── PAY-0021 HyperSwitch Lifecycle
     └── PAY-0022 Provider Certification
```

Subsequent ADRs MAY refine the internal structure of these concepts.

They SHALL NOT silently transfer the authority established here.

Any such authority change requires an explicit superseding architectural decision.

---

# 59. Final Decision

Baobab adopts a canonical, provider-independent payment domain in which:

```text
COMMERCIAL ENGINE
owns
WHY MONEY IS OWED

        │
        ▼

BAOBAB PAYMENTS
owns
HOW PAYMENT IS EXECUTED
AND WHAT HAPPENED DURING EXECUTION

        │
        ▼

PAYMENT PROVIDER
performs
EXTERNAL FINANCIAL PROCESSING

        │
        ▼

BAOBAB PAYMENTS
observes
PAYMENT / REFUND / SETTLEMENT FACTS

        │
        ▼

BAOBAB ERP
owns
THE ACCOUNTING CONSEQUENCES
```

The canonical distinctions are therefore:

```text
Obligation
    ≠
PaymentIntent
    ≠
Payment
    ≠
PaymentAttempt
    ≠
Refund
    ≠
Dispute / Chargeback
    ≠
Settlement
    ≠
Payout
    ≠
Accounting Entry
```

HyperSwitch provides payment orchestration.

Payment providers execute external financial operations.

The Control Plane provides authoritative business context.

Trade and Subscriptions provide commercial obligations.

ERP provides accounting authority.

IAM provides identity and authorisation foundations.

**`baobab-payments` alone owns Baobab's canonical operational truth about payment execution — no more, and no less.**