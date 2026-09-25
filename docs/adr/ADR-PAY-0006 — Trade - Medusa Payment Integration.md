# ADR-PAY-0006 — Trade / Medusa Payment Integration

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Commerce Integration / Payment Execution |
| **Scope** | `baobab-payments` ↔ `baobab-trade` |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0005; applicable `baobab-trade`, Control Plane and Shared ADRs; ADR-SHARED-011 |
| **Related** | ADR-PAY-0007; ADR-PAY-0008; ADR-PAY-0009; ADR-PAY-0010; ADR-PAY-0011; ADR-PAY-0012; ADR-PAY-0013; ADR-PAY-0015; ADR-PAY-0018; ADR-PAY-0022 |

---

# 1. Context

Baobab uses MedusaJS within `baobab-trade` as its headless commerce engine.

Trade owns commerce concerns such as:

```text
catalogue
cart
pricing
promotions
commerce customer
payment collection
order
fulfilment
inventory interaction
commerce workflow
```

`baobab-payments`, by contrast, exists to provide reusable payment execution and orchestration across Baobab engines.

It is not a Medusa-specific payment microservice.

It must also support payment obligations originating from:

```text
baobab-subscriptions
future marketplace capabilities
future service engines
other authorised Baobab workloads
```

The integration must consequently preserve a clear boundary between:

```text
COMMERCE PAYMENT SEMANTICS
```

and:

```text
EXTERNAL PAYMENT EXECUTION
```

Medusa may know that an order requires payment.

It must not become the platform authority for:

```text
provider certification
payment provider routing
payment credentials
HyperSwitch organisation
HyperSwitch merchant/profile selection
cross-engine payment execution
```

Conversely, `baobab-payments` must not become authoritative for:

```text
cart
order
order total
discount
promotion
inventory
fulfilment
commerce lifecycle
```

This ADR defines that boundary.

---

# 2. Decision

`baobab-trade` SHALL integrate with `baobab-payments` through a Baobab-owned Medusa Payment Provider adapter.

Conceptually:

```text
Digital Estate
      │
      ▼
baobab-trade / Medusa
      │
      ▼
Baobab Medusa Payment Provider
      │
      ▼
Baobab Payment API
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
Eligible PSP / Payment Rail
```

Medusa SHALL NOT normally integrate directly with HyperSwitch or individual PSPs for Baobab-governed payment execution.

---

# 3. Governing Principle

> **Trade decides what the customer owes in the commerce domain. Payments decides how an authorised payment obligation is externally executed.**

Therefore:

```text
TRADE
owns
commerce obligation

PAYMENTS
owns
payment execution

ERP
owns
accounting consequence
```

---

# 4. Authority Boundary

| Concern | Authority |
|---|---|
| Product | Trade |
| Variant | Trade |
| Catalogue | Trade |
| Cart | Trade |
| Commerce customer | Trade |
| Commerce pricing | Trade |
| Promotion / discount | Trade |
| Commerce tax calculation | Trade / applicable tax architecture |
| Payment Collection | Trade |
| Payment Session | Trade |
| Order | Trade |
| Order total | Trade |
| Fulfilment | Trade |
| Inventory commerce semantics | Trade |
| Canonical PaymentIntent | Payments |
| External payment execution | Payments |
| Provider eligibility | Payments |
| Provider routing | Payments |
| HyperSwitch orchestration | Payments |
| Provider credential use | Payments |
| Canonical payment execution state | Payments |
| Refund execution | Payments |
| Settlement observation | Payments |
| Tenant / Legal Entity / Market | Control Plane |
| Accounting | ERP |
| Identity / workload authentication | IAM |

---

# 5. Payment Collection

Medusa's Payment Collection remains a commerce-domain object.

It represents commerce's management of payment associated with a cart/order or related commerce workflow.

It SHALL NOT become Baobab's platform-wide canonical PaymentIntent.

Therefore:

```text
Medusa Payment Collection
        ≠
Baobab PaymentIntent
```

The two objects SHALL be correlated through explicit references.

---

# 6. Payment Session

A Medusa Payment Session represents Medusa's provider-facing commerce payment session.

Where the Baobab Payment Provider is selected, that session SHALL represent Medusa's relationship with the Baobab Payments capability.

It SHALL NOT expose HyperSwitch as the commerce provider abstraction.

Conceptually:

```text
Medusa Payment Collection
       │
       ▼
Medusa Payment Session
       │
       ▼
Baobab Payment Provider
       │
       ▼
PaymentIntent
```

---

# 7. PaymentIntent Creation

When Medusa requires external payment execution, the Baobab Payment Provider SHALL create or resolve a canonical Baobab PaymentIntent.

Conceptually:

```text
Order / Cart Context
       │
       ▼
Payment Collection
       │
       ▼
Payment Session
       │
       ▼
Create PaymentIntent
       │
       ▼
baobab-payments
```

The request SHALL include sufficient authoritative information to identify:

```text
source engine
source reference
amount
currency
canonical context
correlation
idempotency
requested payment behaviour
```

---

# 8. Source Reference

Trade-originated PaymentIntents SHALL retain an explicit source reference.

Conceptually:

```text
source_engine = baobab-trade
source_reference = <commerce reference>
```

The exact source reference MAY correspond to the appropriate stable commerce aggregate, such as the Payment Collection or Order relationship, according to the final Shared contract.

Payments SHALL NOT require copied Order contents.

---

# 9. No Order Duplication

`baobab-payments` SHALL NOT create a shadow commerce Order.

It MAY preserve:

```text
source_engine
source_reference
amount
currency
customer/payer reference where required
metadata permitted by contract
```

It SHALL NOT copy the entire:

```text
cart
order
line-item collection
catalogue
promotion state
fulfilment state
```

into its canonical payment model.

---

# 10. Amount Authority

Trade is authoritative for the commerce amount that must be paid.

Payments SHALL treat the authorised amount supplied through the trusted integration as the requested payment obligation.

Payments SHALL NOT independently recalculate:

```text
product prices
discounts
promotions
commerce taxes
shipping charges
order total
```

---

# 11. Amount Validation

Payments MAY validate that:

```text
amount > valid minimum
currency valid
precision valid
context eligible
requested amount compatible with payment operation
```

but such validation SHALL NOT make Payments the commerce-pricing authority.

---

# 12. Currency Authority

Trade SHALL provide the transaction currency of the commerce obligation.

Payments SHALL validate it against PAY-0005 payment context.

Therefore:

```text
Trade:
Order total = 2,500 ZAR
```

becomes:

```text
Payments:
Can 2,500 ZAR be executed
for this Legal Entity × Market × Method?
```

Payments SHALL NOT silently replace the commerce currency.

---

# 13. Market Authority

Trade MAY carry market context as part of commerce operation.

However, the canonical Market identity used for payment execution SHALL align with Control Plane context.

A Medusa Region SHALL NOT independently establish canonical Market identity.

Therefore:

```text
Medusa Region
     ≠
Baobab Market
```

even where explicit mapping exists.

---

# 14. Payment Provider Adapter

Baobab SHALL implement and own a Medusa Payment Provider integration whose responsibility is to translate between:

```text
Medusa Payment Provider Contract
```

and:

```text
Baobab Payment API
```

The adapter SHALL remain thin.

It SHALL not become a second payment orchestration engine.

---

# 15. Adapter Responsibilities

The Baobab Medusa Payment Provider MAY perform:

```text
Medusa request translation
canonical context propagation
PaymentIntent creation
payment execution commands
payment-status retrieval
refund commands
idempotency propagation
correlation propagation
error translation
```

It SHALL NOT perform:

```text
provider routing
PSP selection
provider certification
credential management
merchant inference
HyperSwitch orchestration
cross-provider failover
settlement accounting
```

---

# 16. Provider Selection

Medusa SHALL select:

```text
Baobab Payment Provider
```

as the commerce payment provider.

It SHALL NOT select:

```text
Stripe
Paystack
Flutterwave
Adyen
or another PSP
```

as Baobab's canonical payment execution path merely because such a Medusa provider exists.

The downstream PSP is selected within Payments.

---

# 17. Provider Abstraction

From Trade's perspective:

```text
Provider = Baobab Payments
```

From Payments' perspective:

```text
Execution Route =
eligible provider selected by payment orchestration
```

This is deliberate.

It prevents Trade from coupling itself to PSP topology.

---

# 18. HyperSwitch Boundary

Trade SHALL NOT require knowledge of:

```text
HyperSwitch Organisation ID
HyperSwitch Merchant ID
Business Profile ID
Merchant Connector Account ID
HyperSwitch routing configuration
```

These are Payments concerns.

The integration boundary is:

```text
Trade
  │
  ▼
Baobab Payment Contract
```

not:

```text
Trade
  │
  ▼
HyperSwitch API
```

---

# 19. Canonical Context

Trade SHALL propagate sufficient trusted context to allow Payments to resolve:

```text
tenant
organisation where applicable
legal entity
market
digital estate where applicable
source engine
source reference
```

Payments SHALL resolve operational:

```text
engine instance
merchant
business profile
eligible provider
```

through its governed mappings.

---

# 20. Caller-Supplied Provider IDs

Trade SHALL NOT determine payment authority by sending:

```text
merchant_id
business_profile_id
connector_id
provider_id
```

as caller-controlled routing instructions.

If such values appear for correlation or internal optimisation, Payments SHALL validate them against authoritative context rather than trust them.

---

# 21. Customer Payment-Method Selection

Trade MAY convey the customer's selected canonical payment method.

For example:

```text
CARD
BANK_TRANSFER
MOBILE_MONEY
```

Payments SHALL validate that method against the canonical execution context.

Trade's customer selection is not permission to use an ineligible provider.

---

# 22. Eligible Payment Methods

Before checkout presentation, Trade SHOULD obtain an eligibility projection from the appropriate Payments/Baobab context rather than hard-code payment methods.

Conceptually:

```text
Cart Context
      │
      ▼
Legal Entity + Market + Currency
      │
      ▼
Payment Eligibility
      │
      ▼
Eligible Methods
      │
      ▼
Checkout UI
```

---

# 23. Checkout UI

The digital estate MAY present:

```text
Card
Bank Transfer
Mobile Money
```

where eligible.

It SHOULD NOT need to know that the eventual execution route is:

```text
HyperSwitch
  │
  ▼
Provider X
```

unless disclosure is required for a legitimate business or regulatory reason.

---

# 24. Payment Method Reference

Where checkout produces or selects a payment-method reference, Trade SHALL pass only the permitted canonical/tokenised reference required by Payments.

Trade SHALL not persist raw payment credentials merely for convenience.

PAY-0012 governs sensitive payment information.

---

# 25. PCI Boundary

The integration SHOULD minimise the exposure of Trade and digital estates to sensitive cardholder/payment data.

Where provider-hosted, tokenised or secure payment collection can prevent raw sensitive payment data from traversing Trade, that architecture SHALL be preferred.

Exact PCI and tokenisation requirements are governed by PAY-0012.

---

# 26. Payment Lifecycle Mapping

Medusa commerce payment states and Baobab payment states SHALL be mapped explicitly.

They SHALL NOT be assumed identical.

Conceptually:

```text
Medusa Commerce State
        │
        ▼
Baobab Payment Provider Adapter
        │
        ▼
Canonical Payment State
```

PAY-0008 SHALL define the authoritative Baobab payment lifecycle.

---

# 27. Commerce State Does Not Define Provider State

The following equivalence is prohibited:

```text
Medusa payment state
        =
provider payment state
```

Trade receives a projection appropriate to commerce.

Payments retains the richer operational execution state.

---

# 28. Payment State Does Not Define Order State

Likewise:

```text
Payment CAPTURED
```

does not itself authoritatively mutate the Order outside Trade.

Payments reports the fact.

Trade determines the appropriate commerce transition.

Conceptually:

```text
Payments
   │
   │ payment.captured
   ▼
Trade
   │
   ▼
Commerce workflow decides
what happens to Order
```

---

# 29. Synchronous Response

A synchronous Payments API response MAY provide an immediate result such as:

```text
requires action
authorised
processing
failed
```

where available.

Trade SHALL NOT assume that every payment reaches final state synchronously.

---

# 30. Asynchronous Completion

Payment execution may complete asynchronously through:

```text
provider webhook
HyperSwitch processing
polling
delayed payment rail
3DS/customer action
bank transfer
mobile-money approval
```

Trade SHALL therefore support asynchronous payment-state convergence.

---

# 31. Event Integration

Canonical payment events SHALL be used for asynchronous state propagation.

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
baobab-trade
   │
   ▼
Commerce State Projection
```

Raw provider webhook payloads SHALL NOT be consumed by Trade as canonical payment events.

---

# 32. Webhooks

Trade SHALL NOT expose provider-specific webhook handlers as the normal Baobab architecture.

Provider webhooks terminate within the payment boundary:

```text
Provider
    │
    ▼
HyperSwitch / Payments
    │
    ▼
validated canonical event
    │
    ▼
Trade
```

PAY-0011 defines webhook semantics.

---

# 33. Event Idempotency

Trade SHALL consume payment events idempotently.

Duplicate:

```text
payment.captured
```

events SHALL NOT result in duplicate:

```text
order processing
fulfilment
invoice action
```

or other irreversible commerce operations.

---

# 34. Event Ordering

Trade SHALL NOT assume perfect event ordering.

For example:

```text
payment.authorized
```

and:

```text
payment.captured
```

may be delivered under at-least-once distributed-system semantics.

PAY-0011 defines the detailed event model.

---

# 35. Correlation

The integration SHALL preserve a correlation chain such as:

```text
Cart
  │
  ▼
Payment Collection
  │
  ▼
Payment Session
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

Relevant stable references SHALL be available for operational investigation.

---

# 36. Idempotency

Trade → Payments mutation calls SHALL be idempotent.

This includes operations such as:

```text
create payment intent
authorise
capture
cancel
refund
```

where applicable.

Network retries SHALL NOT create duplicate money movement.

PAY-0010 defines exact idempotency semantics.

---

# 37. PaymentIntent Reuse

Repeated Medusa calls representing the same logical payment operation SHOULD resolve to the same canonical PaymentIntent where the contract and lifecycle permit.

The adapter SHALL NOT create a new PaymentIntent merely because an HTTP request was retried.

---

# 38. Order Mutation

If the commerce amount changes before payment execution due to:

```text
cart change
shipping change
promotion change
tax recalculation
```

Trade SHALL explicitly reconcile the existing payment intent with the new obligation.

Payments SHALL not infer that an old PaymentIntent automatically represents the new amount.

PAY-0008 SHALL define permitted lifecycle transitions.

---

# 39. Authorisation

Where payment methods support separate authorisation and capture:

```text
Order
   │
   ▼
Payment Authorisation
   │
   ▼
Commerce condition
   │
   ▼
Capture
```

Trade MAY determine when capture is commercially appropriate.

Payments executes the capture operation.

---

# 40. Capture Authority

The boundary is:

```text
Trade:
"Capture is now commercially required."

Payments:
"Execute capture safely through
the correct provider context."
```

Trade SHALL NOT directly invoke a PSP capture endpoint.

---

# 41. Capture Amount

Where partial capture is supported, Trade SHALL supply the commercially authorised capture amount.

Payments SHALL validate it against the canonical Payment state and provider capability.

Payments SHALL not independently decide which order lines should be captured.

---

# 42. Cancellation / Void

Where commerce cancellation requires payment void/cancellation:

```text
Trade
   │
   │ cancel/void requested
   ▼
Payments
   │
   ▼
Provider
```

Payments determines the technically valid operation based on canonical payment state and provider capability.

Trade SHALL not assume that:

```text
cancel order
```

always means:

```text
refund payment
```

---

# 43. Refund Initiation

A commerce refund may originate from Trade because of:

```text
return
order cancellation
commercial adjustment
customer service action
```

Trade determines the commerce reason and amount permitted by its domain.

Payments owns execution of the financial refund.

---

# 44. Refund Flow

Conceptually:

```text
Commerce Refund Decision
       │
       ▼
Trade Refund Request
       │
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
Provider
```

PAY-0013 defines detailed refund semantics.

---

# 45. Refund Does Not Rewrite Payment

A refund SHALL remain a distinct Payments aggregate/operation.

Trade SHALL not require Payments to overwrite the original payment as if it never occurred.

Historical sequence matters:

```text
Payment Captured
      │
      ▼
Refund Requested
      │
      ▼
Refund Completed
```

---

# 46. Partial Refunds

Trade MAY request partial refunds where commerce policy allows.

Payments SHALL validate:

```text
refund amount
captured amount
previous refunds
provider capability
currency
payment state
```

before execution.

---

# 47. Refund Failure

A commerce decision to refund and a successfully executed financial refund are different facts.

Therefore:

```text
Trade requests refund
       │
       ▼
Payments attempts refund
       │
       ├── success
       └── failure
```

Trade SHALL preserve appropriate intermediate commerce state until financial outcome is known.

---

# 48. Disputes

Provider disputes and chargebacks originate within the payment domain.

Payments SHALL emit appropriate canonical facts.

Trade MAY consume those facts to:

```text
flag an order
restrict fulfilment where appropriate
inform customer service
provide evidence
```

but Trade SHALL NOT become the canonical dispute engine.

PAY-0014 governs disputes.

---

# 49. Settlement

Trade SHALL NOT determine an order is financially settled merely because:

```text
payment.captured
```

occurred.

Capture and settlement are distinct.

Settlement belongs to the Payments/ERP reconciliation boundary defined by PAY-0015.

---

# 50. Accounting

Trade SHALL NOT create authoritative ledger consequences from provider responses.

The flow is:

```text
Trade
   │
   ▼
Payment Obligation
   │
   ▼
Payments
   │
   ▼
Payment Fact
   │
   ▼
ERP
   │
   ▼
Accounting Consequence
```

---

# 51. Payment Failure

A failed payment SHALL be reported to Trade through canonical Payments semantics.

Trade determines the commerce consequence.

Examples may include:

```text
allow retry
retain cart
hold order
cancel order
request another method
```

Payments SHALL not decide the commerce workflow.

---

# 52. Retry

Where customer or commerce workflow requests retry, Payments SHALL determine whether the existing PaymentIntent can be retried and how.

Trade SHALL NOT choose the replacement PSP.

---

# 53. Provider Failover

Provider failover is entirely inside the Payments orchestration boundary.

Conceptually:

```text
Trade
   │
   ▼
PaymentIntent
   │
   ▼
Payments
   │
   ├── Provider A fails
   │
   └── Provider B eligible
```

Trade sees canonical payment outcome, not routing implementation.

PAY-0009 governs failover.

---

# 54. Failover Cannot Change Commercial Terms

Provider failover SHALL NOT change:

```text
order
amount
currency
legal entity
market
customer-selected payment method
```

without a new explicit business decision where required.

---

# 55. Payment Action Required

Some payment methods require customer interaction.

Payments SHALL be able to return a canonical action requirement.

Conceptually:

```text
Payment
   │
   ▼
REQUIRES_ACTION
   │
   ▼
Action Descriptor
   │
   ▼
Trade / Digital Estate
   │
   ▼
Customer
```

The action descriptor SHALL expose only the information required to continue the payment safely.

---

# 56. Provider UI Leakage

Where provider-hosted interaction is required, Trade MAY display or redirect to provider-controlled UI through canonical Payments instructions.

This SHALL NOT make Trade dependent on the provider's internal domain model.

---

# 57. Action Completion

After customer action:

```text
Customer
   │
   ▼
Provider
   │
   ▼
Payments
   │
   ▼
Canonical state
   │
   ▼
Trade
```

Trade SHALL not trust browser redirects alone as proof of successful payment.

---

# 58. Browser Success Is Not Payment Success

The following equivalence is prohibited:

```text
success_url loaded
       =
payment captured
```

Trade SHALL confirm canonical Payments state.

---

# 59. Browser Failure Is Not Necessarily Final Failure

Likewise:

```text
failure_url loaded
```

may not be sufficient evidence of final provider state in asynchronous scenarios.

Canonical payment state remains authoritative.

---

# 60. Frontend Boundary

The frontend SHALL interact primarily with:

```text
Trade / Medusa
```

for commerce checkout.

It MAY interact with provider-controlled payment surfaces where technically required, but SHALL NOT independently orchestrate Baobab payment routing.

---

# 61. Direct Frontend-to-Payments Calls

Direct digital-estate calls to Payments SHALL be permitted only where an explicitly designed payment flow requires them and appropriate context/authentication exists.

They SHALL NOT bypass Trade authority over the commerce obligation.

---

# 62. B2B ZuriBeans

The architecture SHALL support ZuriBeans B2B workflows where payment may occur:

```text
immediately
after quotation
against invoice
through bank transfer
through another approved method
```

Trade may therefore have commerce flows that do not resemble consumer instant checkout.

The integration SHALL not assume every Order produces immediate card capture.

---

# 63. B2C Thamani

The same Payments interface SHALL support Thamani B2C checkout without coupling the payment engine to B2C-specific semantics.

Conceptually:

```text
ZuriBeans B2B
       │
       ▼
baobab-trade
       │
       ┐
       │
       ▼
baobab-payments
       ▲
       │
       ┘
       │
baobab-trade
       ▲
       │
Thamani B2C
```

The estates remain independent Legal Entities and payment contexts.

---

# 64. No Cross-Estate Payment Leakage

ZuriBeans and Thamani SHALL NOT share payment execution context merely because both use Medusa.

A payment originating from:

```text
ZuriBeans
```

SHALL resolve through:

```text
ZuriBeans legal entity
ZuriBeans market context
ZuriBeans merchant/profile mappings
ZuriBeans provider activation
```

and not Thamani's equivalents.

---

# 65. B2B Account Payment

Future B2B mechanisms such as:

```text
invoice payment
approved account terms
bank transfer
deposit
balance payment
```

MAY require delayed or externally reconciled payment flows.

The Medusa provider integration SHALL not assume all payment methods provide immediate synchronous confirmation.

---

# 66. Offline Payment Claims

An offline or bank-transfer instruction SHALL NOT be treated as successful payment merely because instructions were issued.

Conceptually:

```text
Bank Transfer Instructions Issued
          │
          ≠
Payment Confirmed
```

Confirmation requires appropriate payment/reconciliation evidence.

---

# 67. Order Creation Timing

Trade retains authority over whether an Order may be created:

```text
before payment
after authorisation
after capture
```

according to the applicable commerce workflow.

Payments SHALL not impose a universal order-creation strategy.

---

# 68. Fulfilment Timing

Likewise, Trade determines whether fulfilment requires:

```text
payment authorised
payment captured
manual approval
credit approval
other commerce condition
```

Payments only reports canonical financial execution facts.

---

# 69. No Distributed Transaction

Trade and Payments SHALL NOT depend upon a distributed ACID transaction spanning their databases.

For example:

```text
BEGIN

write Medusa order
write Payments payment

COMMIT BOTH
```

is prohibited as an architectural requirement.

The engines SHALL use:

```text
idempotent commands
durable local transactions
events
reconciliation
state convergence
```

---

# 70. Failure Between Engines

The architecture SHALL tolerate:

```text
Trade request succeeds
Payments response lost
```

without duplicate payment.

It SHALL also tolerate:

```text
Payment succeeds
Trade update temporarily fails
```

without losing the payment fact.

This requires PAY-0010 and PAY-0011 semantics.

---

# 71. Recovery

Trade SHALL be able to recover payment state using canonical identifiers and source references.

Recovery SHALL not depend solely upon the original synchronous HTTP response.

---

# 72. Reconciliation Between Trade and Payments

Baobab SHOULD support operational reconciliation capable of identifying:

```text
Trade payment session
without PaymentIntent

PaymentIntent
without expected Trade source

Trade says pending
Payments says captured

Trade says paid
Payments has no supporting canonical payment fact
```

These discrepancies SHALL be observable.

---

# 73. State Convergence

The desired distributed-system model is:

```text
Trade State
    │
    │ commands/events
    ▼
Payments State
    │
    │ canonical events
    ▼
Trade Projection

        eventually converges
```

not shared-database mutation.

---

# 74. Contract Versioning

Trade ↔ Payments integration SHALL use versioned canonical contracts.

Conceptually:

```text
contracts/payments/v1
```

Provider-specific APIs SHALL remain behind the Payments boundary.

Breaking payment-contract changes require controlled version evolution.

---

# 75. API Authentication

Trade SHALL authenticate to Payments as an authorised workload.

Production integration SHALL use the platform-approved workload identity mechanism.

Relevant payment scopes may include:

```text
payment:execute
payment:refund
payment:read
```

according to Shared/IAM contracts.

Static shared bearer secrets SHALL not be the production architecture.

---

# 76. Authorisation

Authentication alone SHALL not grant arbitrary payment authority.

Payments SHALL also evaluate:

```text
workload scope
tenant context
legal entity
market
requested operation
payment ownership
```

according to PAY-0018.

---

# 77. Trade Cannot Refund Another Entity's Payment

A compromised or misconfigured Trade context SHALL not be able to refund a Payment belonging to another:

```text
tenant
legal entity
```

merely by knowing its `payment_id`.

Ownership/context validation is mandatory.

---

# 78. Idempotency Namespace

Trade-generated idempotency information SHALL be namespaced sufficiently to avoid collision across:

```text
tenant
legal entity
source operation
environment
```

Exact semantics belong to PAY-0010.

---

# 79. Correlation Identifiers

The integration SHOULD preserve:

```text
correlation_id
payment_intent_id
payment_id
source_engine
source_reference
```

alongside appropriate Medusa references.

This SHALL allow one transaction to be traced without relying on log-message text searches.

---

# 80. Error Model

Payments SHALL return canonical error classes rather than raw PSP errors as the stable Trade contract.

Representative categories MAY include:

```text
PAYMENT_CONTEXT_INVALID
PAYMENT_METHOD_NOT_ELIGIBLE
PAYMENT_ACTION_REQUIRED
PAYMENT_DECLINED
PAYMENT_PROCESSING
PAYMENT_PROVIDER_UNAVAILABLE
PAYMENT_OPERATION_NOT_ALLOWED
PAYMENT_ALREADY_PROCESSED
```

Exact error contracts SHALL be versioned.

---

# 81. Provider Error Preservation

Provider-specific errors MAY be retained internally for:

```text
operations
support
routing
analytics
incident investigation
```

but Trade SHOULD consume canonical error semantics.

Sensitive provider details SHALL not leak unnecessarily.

---

# 82. Retryable Errors

Payments SHALL indicate whether an error is:

```text
retryable
non-retryable
requires customer action
requires different method
requires operator intervention
```

where the canonical contract supports doing so safely.

Trade SHALL not infer retryability from raw provider error strings.

---

# 83. Timeouts

A timeout SHALL NOT automatically mean:

```text
payment failed
```

The execution may have succeeded remotely.

Therefore:

```text
timeout
   │
   ▼
UNKNOWN / PROCESSING
   │
   ▼
recover state
```

may be required before retry.

This is critical to duplicate-payment prevention.

---

# 84. Circuit Breaking

Payments MAY apply:

```text
timeouts
circuit breakers
provider health
retry policy
failover
```

within its execution boundary.

Trade SHALL not duplicate provider-level resilience logic.

---

# 85. Observability

Cross-engine traces SHOULD permit reconstruction of:

```text
Digital Estate
   │
Cart
   │
Payment Collection
   │
Payment Session
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

without exposing sensitive payment data.

---

# 86. Metrics

Useful integration metrics MAY include:

```text
payment intent creation rate
authorisation success
capture success
decline rate
requires-action rate
payment latency
provider latency
Trade/Payments state mismatch
idempotency replay count
refund success
event convergence delay
```

Metrics SHOULD be segmentable by authorised contextual dimensions such as:

```text
tenant
legal entity
market
currency
payment method
```

without violating isolation.

---

# 87. Logging

Trade and Payments logs SHOULD share:

```text
correlation_id
```

and appropriate canonical identifiers.

Logs SHALL NOT include:

```text
PAN
CVV
raw payment credential
provider secret
unredacted sensitive token
```

---

# 88. Deployment Independence

`baobab-trade` and `baobab-payments` SHALL remain independently deployable.

Neither SHALL require the other's database schema.

Contract compatibility SHALL be maintained through:

```text
versioned API
versioned events
contract testing
```

---

# 89. HyperSwitch Unavailability

If HyperSwitch is unavailable:

```text
Trade
   │
   ▼
Payments
   │
   ▼
Provider execution unavailable
```

Trade SHALL receive an appropriate canonical failure/processing response.

Trade SHALL NOT bypass Payments and call a PSP directly as an automatic fallback.

---

# 90. Payments Unavailability

If `baobab-payments` is unavailable, Trade MAY continue commerce operations that do not require immediate payment execution where its business workflow permits.

It SHALL NOT fabricate payment success.

---

# 91. Sandbox Provider

Development/test environments MAY use the Baobab sandbox provider.

The integration remains:

```text
Medusa
   │
   ▼
Baobab Payment Provider
   │
   ▼
baobab-payments
   │
   ▼
SandboxProvider
```

Every resulting transaction/event SHALL remain:

```text
simulated = true
```

---

# 92. HyperSwitch Adapter Readiness

The existence of the Medusa → Payments integration SHALL NOT be described as production HyperSwitch integration unless the Payments HyperSwitch adapter actually performs HyperSwitch execution.

A chain ending at:

```text
SandboxProvider
```

is sandbox payment integration.

It is not production PSP integration.

---

# 93. Production Readiness

Representative Trade/Payments production readiness SHALL include:

```text
TRADE / PAYMENTS READINESS

Canonical payment contracts          PASS
Medusa provider adapter              PASS
Workload authentication              PASS
Tenant context propagation           PASS
Legal Entity context                 PASS
Market context                       PASS
Currency validation                  PASS
Method eligibility                   PASS
Merchant/Profile mapping             PASS
HyperSwitch adapter                  PASS
Provider certification               PASS
Provider activation                  PASS
Idempotency                          PASS
Webhook/event convergence            PASS
Refund integration                   PASS
Settlement projection                PASS
Operational reconciliation           PASS
Observability                        PASS
Security hardening                   PASS
-----------------------------------------
Production Payment Integration       READY
```

A sandbox-only provider SHALL NOT satisfy the production HyperSwitch/provider gates.

---

# 94. Implementation Layers

The recommended separation is:

```text
baobab-trade
│
├── Medusa Commerce Domain
│
├── Payment Collection
│
└── Baobab Payment Provider
          │
          ▼
────────────────────────────────────
       canonical API boundary
────────────────────────────────────
          │
          ▼
baobab-payments
│
├── API / Application Layer
├── Canonical Payment Domain
├── PaymentProvider Port
│      │
│      ├── SandboxProvider
│      └── HyperSwitchProvider
│
├── Context / Eligibility
├── Idempotency
├── Event Translation
└── Persistence
          │
          ▼
HyperSwitch
          │
          ▼
PSPs / payment rails
```

---

# 95. Contract Testing

Baobab SHOULD maintain automated contract tests verifying:

```text
Medusa adapter
        ↕
Payments API
```

compatibility.

Tests SHOULD include:

```text
PaymentIntent creation
authorisation
capture
action required
failure
timeout
retry
duplicate request
refund
asynchronous completion
context rejection
cross-tenant rejection
```

---

# 96. Integration Testing

Production-like integration testing SHOULD verify the complete path:

```text
Medusa
  │
  ▼
Baobab Payment Provider
  │
  ▼
Payments API
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

Testing only:

```text
Medusa
  │
  ▼
SandboxProvider
```

does not prove HyperSwitch integration.

---

# 97. Invariants

The following invariants SHALL hold:

1. Trade owns commerce semantics.
2. Payments owns external payment execution.
3. Medusa Payment Collection is not Baobab PaymentIntent.
4. Medusa Payment Session is not canonical Payment.
5. Medusa Region is not canonical Market.
6. Trade does not select PSP routing.
7. Trade does not manage provider credentials.
8. Trade does not call HyperSwitch directly for normal Baobab payment execution.
9. HyperSwitch identifiers do not become Trade authority.
10. Payments does not recalculate commerce pricing.
11. Payments does not own Order.
12. Payments does not own Cart.
13. Payments does not own fulfilment.
14. Payments does not own inventory.
15. Payment success does not itself define Order lifecycle.
16. Order cancellation does not automatically mean refund.
17. Refund request does not mean refund completed.
18. Browser redirect does not prove payment success.
19. Timeout does not necessarily mean payment failure.
20. Retries SHALL be idempotent.
21. Trade consumes canonical payment events, not raw PSP webhooks.
22. Provider failover remains inside Payments.
23. Failover cannot change commercial terms silently.
24. ZuriBeans and Thamani retain independent payment contexts.
25. Shared Medusa infrastructure does not imply shared Merchant identity.
26. Payment capture is not settlement.
27. Settlement is not accounting reconciliation.
28. Trade and Payments do not share databases.
29. Distributed ACID transactions are not required across engines.
30. Sandbox payment success is not production payment readiness.

---

# 98. Consequences

## Positive

This decision:

- keeps Medusa focused on commerce;
- makes Payments reusable across engines;
- prevents PSP coupling in Trade;
- centralises provider orchestration;
- centralises payment security;
- supports B2B and B2C commerce;
- supports asynchronous payment methods;
- supports multi-market payment execution;
- supports provider failover;
- preserves independent legal entities;
- simplifies future PSP migration;
- reduces PCI exposure;
- enables consistent reconciliation and observability.

## Negative

It introduces:

- another service boundary;
- network communication;
- distributed-state convergence;
- adapter maintenance;
- idempotency requirements;
- event handling;
- contract-version management;
- operational reconciliation.

These costs are accepted because direct PSP integration inside every business engine would fragment payment authority and materially weaken Baobab's architecture.

---

# 99. Alternatives Considered

## Use Medusa PSP providers directly

Rejected as the Baobab platform architecture.

It would make Trade responsible for provider topology and prevent clean reuse by Subscriptions and future engines.

---

## Implement HyperSwitch directly as a Medusa provider

Rejected as the canonical boundary.

This would couple Trade to HyperSwitch and bypass the Baobab Payments domain.

---

## Make Payments own Order payment state entirely

Rejected.

Commerce payment state remains a Trade concern; Payments provides authoritative execution facts.

---

## Share the Payments database with Trade

Rejected.

It violates engine independence and authority boundaries.

---

## Let the frontend call PSPs directly

Rejected as the orchestration architecture.

Secure provider-controlled client components may participate in tokenisation/customer interaction, but payment authority remains server-governed.

---

## Use synchronous calls only

Rejected.

Many payment methods and provider workflows are inherently asynchronous.

---

# 100. Final Decision

The canonical commerce payment path SHALL be:

```text
CUSTOMER
   │
   ▼
DIGITAL ESTATE
   │
   ▼
BAOBAB TRADE / MEDUSA
   │
   │
   │ owns:
   │ cart
   │ order
   │ pricing
   │ commerce payment collection
   │
   ▼
BAOBAB MEDUSA PAYMENT PROVIDER
   │
   ▼
BAOBAB PAYMENTS API
   │
   │
   │ owns:
   │ PaymentIntent
   │ Payment
   │ execution
   │ eligibility
   │ routing
   │
   ▼
HYPERSWITCH
   │
   ▼
CERTIFIED / ACTIVATED PROVIDER
```

Results return as:

```text
PROVIDER
   │
   ▼
HYPERSWITCH
   │
   ▼
BAOBAB PAYMENTS
   │
   ▼
CANONICAL PAYMENT STATE / EVENTS
   │
   ▼
BAOBAB TRADE
   │
   ▼
COMMERCE STATE
```

The authority boundary is therefore:

```text
TRADE
determines
WHAT THE CUSTOMER OWES

        │
        ▼

PAYMENTS
determines
HOW THAT AUTHORISED OBLIGATION
CAN BE EXECUTED

        │
        ▼

HYPERSWITCH
ORCHESTRATES
THE ELIGIBLE PAYMENT ROUTE

        │
        ▼

PROVIDER
PERFORMS
THE EXTERNAL PAYMENT OPERATION

        │
        ▼

PAYMENTS
REPORTS
THE CANONICAL EXECUTION FACT

        │
        ▼

TRADE
DECIDES
THE COMMERCE CONSEQUENCE
```

**Medusa owns commerce. Baobab Payments owns payment execution. HyperSwitch orchestrates eligible providers. No layer silently assumes the authority of another.**