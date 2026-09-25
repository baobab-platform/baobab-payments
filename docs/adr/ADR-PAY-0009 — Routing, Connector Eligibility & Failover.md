# ADR-PAY-0009 — Routing, Connector Eligibility & Failover

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Payment Routing / Resilience |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0008; ADR-PAY-0022; ADR-SHARED-011; applicable Control Plane and Shared contracts |
| **Related** | ADR-PAY-0010; ADR-PAY-0011; ADR-PAY-0012; ADR-PAY-0015; ADR-PAY-0017; ADR-PAY-0018; ADR-PAY-0019; ADR-PAY-0020; ADR-PAY-0021 |

---

# 1. Context

Baobab Payments may eventually orchestrate multiple payment providers, connectors, acquirers and payment rails across:

```text
multiple tenants
multiple legal entities
multiple markets
multiple currencies
multiple payment methods
multiple provider contracts
multiple regions
multiple HyperSwitch engine instances
```

A payment cannot therefore be routed merely by asking:

```text
Which provider is available?
```

The correct question is:

```text
Which routes are AUTHORISED,
ELIGIBLE,
CAPABLE,
HEALTHY,
and SAFE

for THIS payment context
at THIS moment?
```

These distinctions are critical.

A technically reachable provider may not be:

- contracted for the Legal Entity;
- certified by Baobab;
- activated for the Market;
- enabled for the Currency;
- capable of the Payment Method;
- permitted for the requested transaction type;
- authorised for the relevant Merchant/Profile;
- compliant with the jurisdiction;
- healthy enough to receive traffic;
- safe to use following an ambiguous previous attempt.

Routing is therefore a constrained decision problem, not a provider-selection convenience.

---

# 2. Decision

Baobab SHALL implement payment routing as a two-stage process:

```text
ELIGIBILITY
     │
     ▼
RANKING / ROUTING
```

HyperSwitch MAY perform dynamic routing, retry and failover among eligible connectors.

However, Baobab SHALL first establish the permitted execution envelope.

Conceptually:

```text
Canonical Payment Context
          │
          ▼
Baobab Eligibility Resolution
          │
          ▼
Eligible Execution Envelope
          │
          ▼
HyperSwitch Routing
          │
          ▼
Eligible Connector
          │
          ▼
Provider
```

HyperSwitch SHALL NOT be given unrestricted authority to route outside Baobab's canonical business constraints.

---

# 3. Governing Principle

> **Baobab determines where payment is allowed to execute. HyperSwitch determines how to route among the allowed execution choices.**

Therefore:

```text
BAOBAB
decides
BUSINESS ELIGIBILITY

        │
        ▼

HYPERSWITCH
decides
PAYMENT ROUTE

        │
        ▼

PROVIDER
performs
PAYMENT EXECUTION
```

---

# 4. Eligibility Is Not Routing

These concepts SHALL remain separate.

```text
Eligibility
=
May this connector/provider be used?
```

```text
Routing
=
Which eligible connector/provider should be used?
```

A routing algorithm SHALL NOT make an ineligible connector eligible.

---

# 5. Canonical Routing Context

Every routing decision SHALL originate from trusted canonical context.

At minimum, relevant dimensions may include:

```text
tenant
organisation where applicable
legal entity
market
digital estate where applicable
engine instance
environment

currency
amount
payment method
transaction type

capture mode
source engine
commercial model where relevant
```

The exact context contract SHALL remain versioned.

---

# 6. Routing Authority Matrix

| Concern | Authority |
|---|---|
| Tenant | Control Plane |
| Organisation | Control Plane |
| Legal Entity | Control Plane |
| Market | Control Plane |
| Digital Estate | Control Plane |
| Engine Instance | Control Plane |
| Isolation Profile | Control Plane |
| Merchant/Profile mapping | Payments |
| Payment-method eligibility | Payments |
| Provider certification | Payments |
| Provider activation | Payments |
| Execution eligibility | Payments |
| Canonical payment state | Payments |
| Dynamic routing among eligible connectors | HyperSwitch |
| Connector invocation | HyperSwitch |
| Provider execution | PSP / Acquirer / Rail |
| Accounting consequence | ERP |
| Mutation authority | IAM + Payments policy |

---

# 7. Route Model

A payment execution route SHOULD conceptually represent:

```text
ExecutionRoute
│
├── engine_instance
├── external merchant
├── business profile
├── connector
├── provider
├── payment method
├── currency
├── transaction capability
├── environment
└── applicable constraints
```

This is an execution concept, not a new canonical organisational hierarchy.

---

# 8. Eligibility Pipeline

The routing pipeline SHALL conceptually operate as:

```text
Payment Context
      │
      ▼
Context Valid?
      │
      ▼
Merchant Mapping Valid?
      │
      ▼
Business Profile Valid?
      │
      ▼
Provider Certified?
      │
      ▼
Provider Activated?
      │
      ▼
Market Eligible?
      │
      ▼
Currency Eligible?
      │
      ▼
Payment Method Eligible?
      │
      ▼
Transaction Capability Eligible?
      │
      ▼
Operationally Available?
      │
      ▼
SAFE TO EXECUTE?
      │
      ▼
Eligible Route Set
```

Any mandatory failure SHALL remove the route.

---

# 9. Fail Closed

If eligibility cannot be established confidently:

```text
UNKNOWN
```

SHALL NOT mean:

```text
ELIGIBLE
```

The default is:

```text
UNKNOWN ELIGIBILITY
        │
        ▼
DO NOT ROUTE
```

Payment execution is financially sensitive; permissive guessing is unacceptable.

---

# 10. Provider Certification

A connector/provider SHALL NOT participate in production routing merely because:

```text
connector exists
API responds
credentials are configured
sandbox test passed
```

PAY-0022 certification SHALL be satisfied.

Therefore:

```text
Connector Available
       ≠
Provider Certified
```

---

# 11. Provider Activation

Certification alone SHALL NOT make a provider routable.

The applicable Legal Entity / Market / payment context activation SHALL also be valid.

Therefore:

```text
Certified
     ≠
Activated
```

and:

```text
Activated
     ≠
Automatically Selected
```

---

# 12. Legal-Entity Eligibility

A provider route SHALL be eligible only for the Legal Entity for which the required payment arrangement is authorised.

Example:

```text
Provider X
certified globally
      │
      ├── ZuriBeans ZA     ACTIVE
      └── Thamani ZA       NOT ACTIVE
```

The provider SHALL be eligible for the first context and ineligible for the second.

---

# 13. No Group Inheritance

Corporate relationship SHALL NOT create payment eligibility.

Therefore:

```text
ZuriBeans provider activation
        │
        X
        ▼
Thamani provider activation
```

does not inherit automatically.

Each Legal Entity retains independent financial configuration.

---

# 14. Tenant Isolation

The same principle applies to external Baobab tenants.

Provider activation for:

```text
Tenant A
```

SHALL NOT establish eligibility for:

```text
Tenant B
```

unless an explicit architecture and contract authorises the shared arrangement.

---

# 15. Merchant Eligibility

The Legal Entity SHALL resolve to an active Merchant mapping according to PAY-0004.

A provider SHALL not be routed using an unrelated Merchant simply because the connector is technically available.

---

# 16. Business Profile Eligibility

The applicable Business Profile SHALL be determined through explicit governed mapping.

Profile names SHALL NOT be used as routing authority.

For example:

```text
profile_name = "south-africa"
```

does not prove:

```text
market = ZA
```

---

# 17. Market Eligibility

A route SHALL be eligible for the canonical Market associated with the payment context.

The following is prohibited:

```text
Market ZA
    │
    ▼
connector configured only for UG
```

unless the configuration explicitly and lawfully supports the required cross-border semantics.

PAY-0017 governs cross-border cases.

---

# 18. Currency Eligibility

A route SHALL support the requested transaction Currency.

For example:

```text
Payment = ZAR
Provider Route = UGX only
```

means:

```text
INELIGIBLE
```

unless a separately governed FX/cross-border flow applies.

Routing SHALL NOT silently change Currency.

---

# 19. Payment-Method Eligibility

The route SHALL support the canonical Payment Method.

Example:

```text
CARD
MOBILE_MONEY
BANK_TRANSFER
```

A card-only connector SHALL not receive a mobile-money payment merely because it has a high routing score.

---

# 20. Transaction Capability

Eligibility SHALL account for the requested operation.

A provider capable of:

```text
AUTHORIZE
CAPTURE
```

may not necessarily support:

```text
PARTIAL_CAPTURE
REFUND
PARTIAL_REFUND
VOID
RECURRING
MANDATE
```

The route must support the required operation.

---

# 21. Capture-Mode Eligibility

If a payment requires:

```text
MANUAL CAPTURE
```

a route supporting only immediate capture SHALL not be selected.

---

# 22. Recurring Eligibility

Subscription payment execution may require:

```text
token reuse
mandate support
merchant-initiated transaction support
recurring payment capability
```

where applicable.

A connector lacking the required capability SHALL be excluded.

---

# 23. Amount Constraints

Providers or payment methods MAY have:

```text
minimum amount
maximum amount
transaction limits
daily limits
risk thresholds
```

where available and authoritative.

Such constraints MAY participate in eligibility.

---

# 24. Commercial Constraints

Provider agreements MAY impose constraints such as:

```text
legal entity
market
currency
transaction type
volume
payment method
merchant category
```

These MAY participate in eligibility where represented in governed configuration.

They SHALL NOT be inferred from provider availability alone.

---

# 25. Compliance Constraints

Routing SHALL respect applicable compliance constraints.

These may include:

```text
jurisdiction
provider licence scope
merchant onboarding status
payment-method regulation
data residency
sanctions controls
risk controls
```

where applicable to the execution context.

Routing optimisation SHALL never override mandatory compliance restrictions.

---

# 26. Environment Isolation

Production SHALL route only through production-authorised resources.

Therefore:

```text
production Payment
       │
       X
       ▼
sandbox connector
```

is prohibited.

Likewise sandbox/test transactions SHALL remain clearly separated.

---

# 27. Simulation

`simulated = true` payments SHALL not be routed to real-money providers.

Sandbox execution and production execution remain distinct.

---

# 28. Engine Instance

Control Plane SHALL resolve the applicable payment Engine Instance.

Routing SHALL occur within that resolved instance boundary unless an explicit resilience architecture authorises otherwise.

Payments SHALL not discover another engine instance opportunistically merely because it is reachable.

---

# 29. Regional Isolation

Where Baobab operates multiple regional payment deployments:

```text
Africa South
Africa East
EU
```

or equivalent future topology, routing SHALL respect:

```text
data residency
tenant isolation
provider contracts
regulatory scope
engine-instance authority
```

---

# 30. Eligible Route Set

After eligibility resolution, Payments SHALL produce a set conceptually equivalent to:

```text
EligibleRoutes = {
    Route A,
    Route B,
    Route C
}
```

Only members of this set may participate in routing.

---

# 31. Empty Route Set

If:

```text
EligibleRoutes = {}
```

Payments SHALL fail safely.

It SHALL NOT:

```text
pick any configured connector
use another tenant's provider
switch legal entity
change market
change currency
change payment method
```

merely to complete the transaction.

---

# 32. Routing

Within the eligible route set, routing MAY consider factors such as:

```text
provider health
success rate
latency
cost
payment-method performance
provider capacity
geography
historical performance
commercial preference
```

subject to policy and available data.

---

# 33. Routing Is Constrained Optimisation

Conceptually:

```text
ALL CONNECTORS
      │
      ▼
ELIGIBILITY FILTER
      │
      ▼
PERMITTED CONNECTORS
      │
      ▼
ROUTING OPTIMISATION
      │
      ▼
SELECTED CONNECTOR
```

Optimisation occurs only after hard constraints.

---

# 34. Hard Versus Soft Constraints

Routing policy SHALL distinguish:

```text
HARD CONSTRAINTS
```

from:

```text
SOFT PREFERENCES
```

Hard constraints cannot be overridden by routing optimisation.

---

# 35. Representative Hard Constraints

Hard constraints include where applicable:

```text
tenant
legal entity
environment
provider certification
provider activation
market eligibility
currency eligibility
payment-method support
transaction capability
security/compliance prohibition
```

---

# 36. Representative Soft Preferences

Soft preferences MAY include:

```text
cost
latency
success rate
preferred acquirer
load distribution
commercial preference
```

A soft preference SHALL never override a hard constraint.

---

# 37. Routing Policy

Routing policy SHOULD be declarative/configuration-driven where practical.

Leaf applications SHALL not contain logic such as:

```text
if market == "ZA":
    provider = X
elif market == "UG":
    provider = Y
```

Provider topology belongs to Payments.

---

# 38. HyperSwitch Routing

HyperSwitch MAY implement dynamic routing among the eligible connectors made available for the execution context.

Baobab SHALL use HyperSwitch routing capabilities rather than duplicating every routing algorithm unnecessarily.

---

# 39. HyperSwitch Is Not Business Authority

HyperSwitch SHALL NOT determine:

```text
which tenant owns the payment
which Legal Entity is responsible
which Market is canonical
whether a provider is contractually approved
whether cross-entity routing is permitted
```

Those decisions precede routing.

---

# 40. Routing Envelope

Baobab SHOULD provide or enforce a routing envelope conceptually equivalent to:

```text
RoutingEnvelope
│
├── tenant
├── legal entity
├── market
├── currency
├── payment method
├── transaction capability
├── merchant/profile
└── permitted connector set
```

HyperSwitch then routes within that envelope.

---

# 41. Route Snapshot

Every PaymentAttempt SHALL preserve sufficient information about the selected execution route.

Conceptually:

```text
PaymentAttempt
│
├── engine_instance
├── merchant
├── business_profile
├── connector
├── provider
├── payment method
├── currency
└── routing decision reference
```

This snapshot supports historical integrity.

---

# 42. Routing Decision Record

Payments SHOULD preserve a routing-decision record sufficient to answer:

```text
What routes were eligible?

Which route was selected?

Why was it selected?

Which constraints excluded alternatives?

Which policy version was applied?
```

Sensitive commercial details MAY be access-controlled.

---

# 43. Routing Policy Version

A PaymentAttempt SHOULD be traceable to the routing/eligibility policy version used at execution time.

Later configuration changes SHALL not rewrite historical decisions.

---

# 44. Connector Health

Connector health MAY influence routing.

Health signals MAY include:

```text
availability
timeout rate
error rate
latency
recent technical failures
provider incident status
```

---

# 45. Health Does Not Create Eligibility

A healthy connector that is not certified or activated remains ineligible.

Likewise:

```text
high success rate
```

does not override:

```text
wrong Legal Entity
```

---

# 46. Circuit Breakers

Payments/HyperSwitch MAY use circuit breakers to temporarily suppress unhealthy routes.

Conceptually:

```text
Connector
   │
   ├── healthy ─────► routable
   │
   └── unhealthy ───► circuit open
```

provided the connector was otherwise eligible.

---

# 47. Operational Suspension

Payments SHALL support rapid operational suspension of a provider/connector.

A suspended route SHALL be removed from new payment eligibility.

Existing transactions SHALL be handled according to their lifecycle rather than erased.

---

# 48. Certification Suspension

If PAY-0022 certification becomes:

```text
SUSPENDED
EXPIRED
REVOKED
```

the affected route SHALL become ineligible for new production execution.

---

# 49. Activation Suspension

Likewise, suspension of the applicable Market/Legal-Entity activation SHALL remove the route from new execution.

---

# 50. In-Flight Payments

Suspending a route SHALL not automatically rewrite existing PaymentAttempts.

In-flight transactions may require:

```text
status recovery
webhook processing
capture completion
refund
dispute handling
settlement reconciliation
```

through the original provider.

---

# 51. Failover

Failover means:

> Creating or selecting a subsequent eligible execution route after a previous PaymentAttempt did not safely complete the Payment.

Failover is not simply:

```text
try another provider whenever anything goes wrong
```

---

# 52. Failover Safety Principle

> **No failover is permitted when doing so could create an uncontrolled duplicate financial operation.**

This principle takes precedence over availability.

---

# 53. Failure Classification Before Failover

Before failover, the previous attempt SHALL be classified.

Conceptually:

```text
Attempt Outcome
      │
      ├── DEFINITIVE FAILURE
      │
      ├── SAFE TECHNICAL FAILURE
      │
      ├── BUSINESS DECLINE
      │
      ├── REQUIRES ACTION
      │
      ├── PROCESSING
      │
      └── UNKNOWN
```

Different classes require different behaviour.

---

# 54. Definitive Technical Failure

A failover MAY be safe when authoritative evidence establishes that the provider did not perform the financial operation.

Example:

```text
connector unavailable
before request submission
```

or an equivalent definitive failure.

---

# 55. Pre-Submission Failure

If the request never left Baobab/HyperSwitch:

```text
Attempt 1
   │
   X
connector unavailable
before submission
```

a new eligible route may normally be attempted safely.

---

# 56. Post-Submission Ambiguity

If the request was submitted but the response was lost:

```text
Attempt 1
   │
   ▼
Provider
   │
   X
response timeout
```

the outcome is ambiguous.

Immediate failover may create a duplicate charge.

---

# 57. UNKNOWN Blocks Unsafe Failover

PAY-0008 established `UNKNOWN`.

While an attempt remains:

```text
UNKNOWN
```

Payments SHALL NOT automatically fail over to another route unless duplicate safety can be established by a stronger mechanism.

Default behaviour is:

```text
UNKNOWN
   │
   ▼
RECOVER
   │
   ├── failed safely ───► failover may proceed
   ├── succeeded ───────► stop
   └── still unknown ───► do not duplicate
```

---

# 58. Recovery Before Failover

Recovery MAY include:

```text
provider status query
HyperSwitch state query
webhook wait/reconciliation
idempotent replay to same provider
operator intervention
```

according to provider capabilities and PAY-0010.

---

# 59. Business Decline

A definitive customer/payment-method decline SHALL NOT automatically trigger provider failover.

For example:

```text
INSUFFICIENT_FUNDS
```

is not necessarily a provider infrastructure failure.

Trying another acquirer may be:

```text
contractually prohibited
ineffective
risk-sensitive
or undesirable
```

The canonical routing policy SHALL determine whether such retry is allowed.

---

# 60. Hard Decline

A hard decline SHOULD generally stop automatic routing retries for the same payment method unless a documented payment policy permits otherwise.

---

# 61. Soft Decline

A soft decline MAY allow:

```text
customer action
same-provider retry
different eligible route
```

depending on:

```text
payment method
provider semantics
risk rules
network rules
merchant policy
```

The distinction SHALL be canonicalised rather than inferred by upstream engines.

---

# 62. Requires Action

A PaymentAttempt in:

```text
REQUIRES_ACTION
```

SHALL not be failed over merely because another connector exists.

The payer must normally complete or abandon the required action first.

---

# 63. Processing

A PaymentAttempt in:

```text
PROCESSING
```

SHALL not be automatically failed over while the external operation remains legitimately pending.

This is especially important for:

```text
bank transfer
mobile money
bank debit
asynchronous rails
```

---

# 64. Successful Attempt

Once authoritative evidence establishes:

```text
CAPTURED
```

for the required amount, no collection failover SHALL occur for that same monetary obligation.

---

# 65. Authorised Attempt

For manual-capture payments, an:

```text
AUTHORIZED
```

attempt generally SHALL remain bound to the provider holding that authorisation.

Capture SHALL use the applicable original provider route.

Payments SHALL not casually fail over capture to another provider.

---

# 66. Capture Routing

A capture operation normally follows the provider route that created the authorisation.

Conceptually:

```text
AUTHORIZE
Provider A
    │
    ▼
CAPTURE
Provider A
```

not:

```text
AUTHORIZE
Provider A
    │
    ▼
CAPTURE
Provider B
```

unless the payment rail explicitly supports such semantics.

---

# 67. Refund Routing

Refunds generally SHALL route back through the provider relationship that executed the original captured payment.

PAY-0013 defines the exact rules.

A routing optimiser SHALL not independently choose the cheapest provider for a refund.

---

# 68. Dispute Routing

Dispute/chargeback handling remains tied to the original payment/provider relationship.

PAY-0014 governs it.

---

# 69. Settlement Routing

Settlement observations remain tied to the provider/acquirer route that processed the transaction.

PAY-0015 governs settlement.

---

# 70. Retry Versus Failover

These terms SHALL remain distinct.

```text
RETRY
=
repeat an operation,
often against the same route
```

```text
FAILOVER
=
move execution to another eligible route
```

Both require duplicate safety.

---

# 71. Same-Route Retry

A same-route retry MAY be preferable where the provider supports strong idempotency.

Example:

```text
Attempt
  │
  ▼
Provider A
  │
  X timeout
  │
  ▼
replay same idempotent operation
  │
  ▼
Provider A returns original result
```

This may be safer than provider failover.

---

# 72. Provider Idempotency

Provider idempotency MAY contribute to duplicate safety.

However:

```text
provider idempotency
      ≠
Baobab idempotency
```

Baobab remains responsible for canonical duplicate protection under PAY-0010.

---

# 73. Cross-Provider Idempotency

An idempotency key accepted by Provider A SHALL NOT be assumed to protect an operation sent to Provider B.

Therefore cross-provider failover requires explicit duplicate-safety reasoning.

---

# 74. Failover Eligibility Recalculation

Before every failover attempt, eligibility SHALL be recalculated or revalidated.

A route that was eligible five minutes earlier may now be:

```text
suspended
unhealthy
expired
deactivated
```

---

# 75. No Stale Route Assumption

A cached eligible route set SHALL not be treated as permanently valid.

Caching MAY be used, but critical eligibility shall respect appropriate freshness/invalidation guarantees.

---

# 76. Route Exclusion

A failed route MAY be excluded from subsequent routing according to failure classification.

Example:

```text
Attempt 1
Provider A
TECHNICAL FAILURE
      │
      ▼
Provider A temporarily excluded
      │
      ▼
eligible Provider B considered
```

---

# 77. Route Stickiness

Certain payment operations SHOULD remain sticky to their original route.

Examples include:

```text
capture after authorization
refund
void
dispute
settlement reconciliation
```

This SHALL override generic routing optimisation.

---

# 78. Payment-Method Stickiness

Some payment methods create provider-specific:

```text
tokens
mandates
references
customer agreements
```

that may not be portable.

Routing SHALL account for this.

---

# 79. Token Portability

A token created through Provider A SHALL not be assumed usable by Provider B.

Therefore:

```text
Provider A fails
      │
      ▼
Provider B available
```

does not necessarily mean failover is possible with the same Payment Method Reference.

PAY-0012 governs token semantics.

---

# 80. Mandate Portability

Likewise:

```text
Mandate A
```

may be bound to:

```text
provider
merchant
legal entity
payment rail
```

and SHALL not be silently reused elsewhere.

---

# 81. Routing and PCI

Routing decisions SHALL operate on canonical/tokenised payment-method context.

Routing logic SHALL not require unnecessary access to raw sensitive payment credentials.

---

# 82. Routing and Secrets

Provider credentials SHALL be resolved securely at execution time.

They SHALL NOT be embedded in:

```text
routing events
Payment records
Trade configuration
Subscription configuration
digital-estate code
```

---

# 83. Routing Configuration Ownership

Provider routing configuration belongs to the Payments operational domain.

Control Plane remains authoritative for canonical context and Engine Instance resolution.

---

# 84. Control Plane Does Not Pick PSP Per Transaction

Control Plane SHALL determine:

```text
where Payments executes
under what canonical context
with what capability authority
```

It SHOULD NOT become a per-transaction PSP routing engine.

---

# 85. Trade Does Not Pick PSP

Trade may request:

```text
CARD
```

but SHALL not request:

```text
CARD via Provider X
```

as ordinary payment authority.

---

# 86. Subscriptions Does Not Pick PSP

Subscriptions may request collection using an authorised Payment Method Reference.

It SHALL not select the PSP routing topology.

---

# 87. Frontend Does Not Pick PSP

Digital estates SHALL not choose provider routing merely from client-side logic.

Customer selection may choose a canonical payment method, not an unauthorised execution route.

---

# 88. Provider Preference

Where a legitimate business requirement needs provider preference, it SHALL be represented as governed Payments routing policy.

It SHALL not be hidden inside leaf application code.

---

# 89. Routing Policy Precedence

A conceptual precedence is:

```text
LEGAL / SECURITY / COMPLIANCE
            │
            ▼
CANONICAL CONTEXT
            │
            ▼
CERTIFICATION / ACTIVATION
            │
            ▼
CAPABILITY ELIGIBILITY
            │
            ▼
OPERATIONAL HEALTH
            │
            ▼
ROUTING PREFERENCES
            │
            ▼
OPTIMISATION
```

Higher layers cannot be overridden by lower layers.

---

# 90. Cost Routing

Cost MAY influence routing only after all mandatory eligibility checks.

The cheapest route is irrelevant if it is not legally or technically eligible.

---

# 91. Success-Rate Routing

Historical success rate MAY influence routing.

Metrics SHALL be scoped sufficiently to avoid misleading aggregation.

For example:

```text
provider success rate
```

may differ by:

```text
market
payment method
currency
issuer geography
transaction type
```

---

# 92. Latency Routing

Latency MAY be used as a soft preference where appropriate.

It SHALL not override compliance or provider activation.

---

# 93. Load Balancing

Traffic MAY be distributed across multiple eligible providers for:

```text
resilience
capacity
commercial commitments
performance
```

where routing policy permits.

---

# 94. Percentage Routing

Payments MAY support policies such as:

```text
Provider A 70%
Provider B 30%
```

provided both providers are independently eligible for each transaction.

Percentage configuration SHALL not create eligibility.

---

# 95. Priority Routing

Payments MAY support:

```text
Provider A → primary
Provider B → secondary
```

again subject to per-transaction eligibility and failover safety.

---

# 96. Routing Experiments

Controlled routing experiments MAY be permitted where:

```text
all candidate routes are certified
all are activated
all are context-eligible
risk is bounded
audit exists
```

Experimentation SHALL not bypass production governance.

---

# 97. No Silent Cross-Legal-Entity Failover

If no route exists for:

```text
Legal Entity A
```

Payments SHALL NOT route through:

```text
Legal Entity B
```

merely because B has a functioning provider.

---

# 98. No Silent Cross-Market Failover

Likewise:

```text
Market ZA unavailable
```

does not authorise:

```text
route through UG configuration
```

without explicit cross-border policy.

---

# 99. No Silent Currency Substitution

If ZAR routing fails, Payments SHALL not silently execute in USD or UGX.

FX requires explicit PAY-0017 semantics.

---

# 100. No Silent Method Substitution

If CARD is unavailable, Payments SHALL not silently switch the customer to BANK_TRANSFER.

The originating workflow/customer must explicitly choose or approve the alternative where required.

---

# 101. Routing Failure

When no eligible route remains, Payments SHALL return a canonical failure appropriate to the situation.

Representative semantics may include:

```text
NO_ELIGIBLE_PAYMENT_ROUTE
PAYMENT_PROVIDER_UNAVAILABLE
PAYMENT_METHOD_NOT_ELIGIBLE
PAYMENT_CONTEXT_INVALID
PAYMENT_EXECUTION_UNCERTAIN
```

Exact error contracts SHALL be versioned.

---

# 102. Routing Failure Is Not Business Failure

A routing failure does not determine:

```text
order cancellation
subscription cancellation
entitlement revocation
```

The originating business engine decides its consequence.

---

# 103. Routing Events

Payments MAY emit operational events such as:

```text
payment.routing.evaluated
payment.route.selected
payment.route.failed
payment.failover.requested
payment.failover.executed
payment.failover.blocked
```

where operationally useful.

These events SHALL not expose sensitive provider credentials or commercially sensitive detail to unauthorised consumers.

---

# 104. Canonical Payment Events

Business consumers SHOULD normally consume:

```text
payment.processing
payment.requires_action
payment.authorized
payment.captured
payment.failed
```

rather than routing telemetry.

Routing telemetry primarily serves Payments operations and observability.

---

# 105. Audit

For financially significant routing decisions, Baobab SHOULD retain:

```text
payment_id
payment_attempt_id
canonical context
eligible route set or sufficient decision evidence
selected route
routing policy/version
decision timestamp
failover reason where applicable
operator override where applicable
```

---

# 106. Explainability

Operators SHALL be able to determine why a route was:

```text
eligible
ineligible
selected
skipped
failed over
blocked from failover
```

without reverse-engineering opaque application logs.

---

# 107. Operator Override

Manual routing override MAY be supported only through controlled privileged operations.

It SHALL still respect hard eligibility constraints.

An operator SHALL NOT be able to override:

```text
wrong tenant
wrong legal entity
revoked provider
sandbox/production boundary
mandatory compliance prohibition
```

merely through a routing preference.

---

# 108. Emergency Suspension

Operators with appropriate authority SHALL be able to suspend a provider/connector rapidly.

This is different from deleting configuration.

Suspension preserves historical relationships while stopping new execution.

---

# 109. Separation of Duties

High-impact changes such as:

```text
provider activation
routing-policy change
emergency suspension
manual route override
```

SHOULD support separation of duties according to PAY-0018.

---

# 110. Drift Detection

Baobab SHOULD detect divergence between:

```text
Baobab eligibility configuration
```

and:

```text
HyperSwitch connector/routing configuration
```

Examples:

```text
connector enabled in HyperSwitch
but suspended in Baobab

connector removed from HyperSwitch
but active in Baobab

wrong Business Profile attachment

production connector attached
to incorrect Merchant
```

---

# 111. Drift Behaviour

Material drift SHALL:

```text
alert
degrade readiness
and where necessary remove route eligibility
```

It SHALL NOT silently broaden routing authority.

---

# 112. Configuration Reconciliation

Payments SHOULD maintain a reconciliation mechanism that verifies the governed Baobab model against the actual HyperSwitch configuration.

Baobab remains the business-governance source.

---

# 113. Caching

Eligibility data MAY be cached for performance.

Cache keys SHALL be sufficiently scoped.

Representative dimensions include:

```text
tenant
legal entity
market
currency
payment method
transaction type
environment
engine instance
```

---

# 114. Cache Invalidation

Changes to:

```text
provider certification
provider activation
merchant/profile mapping
connector suspension
payment-method eligibility
```

SHOULD invalidate or rapidly expire affected routing caches.

---

# 115. Security

Routing APIs and configuration SHALL be protected from unauthorised mutation.

An attacker able to alter routing could redirect financial transactions.

Routing mutation is therefore a privileged security boundary.

---

# 116. Authentication

Payments SHALL authenticate all administrative routing mutations.

Workload execution requests SHALL also carry trusted workload identity.

---

# 117. Authorisation

Routing mutation SHALL evaluate:

```text
actor
scope
tenant
legal entity
market
operation
```

according to PAY-0018.

---

# 118. No Caller-Controlled Connector

A payment execution request SHALL not gain routing authority merely by including:

```text
connector_id = attacker_choice
```

Caller-supplied connector hints SHALL be ignored or validated against the governed eligible set.

---

# 119. Observability

Routing telemetry SHOULD include appropriate:

```text
correlation_id
payment_id
payment_attempt_id
tenant_id
legal_entity_id
market_id
payment method
selected provider
routing policy version
failover reason
```

without sensitive credentials.

---

# 120. Metrics

Useful metrics MAY include:

```text
eligible-route count
no-route rate
route-selection count
provider selection distribution
provider technical failure rate
failover rate
failover success rate
failover blocked rate
UNKNOWN attempt rate
circuit-breaker state
provider latency
provider success rate
routing decision latency
```

---

# 121. Alerting

Operational alerts SHOULD cover:

```text
no eligible provider for active market
provider failure spike
UNKNOWN attempt spike
excessive failover
routing drift
all routes unhealthy
unexpected connector activation
cross-context routing rejection
```

---

# 122. SLO Relationship

Routing availability contributes to Payments SLOs.

PAY-0019 SHALL define detailed service-level objectives.

High payment availability SHALL not be achieved by weakening eligibility controls.

---

# 123. Disaster Recovery

PAY-0020 SHALL define DR.

After recovery, routing SHALL re-establish:

```text
canonical mappings
certification state
activation state
routing configuration
connector health
```

before broad payment execution resumes.

---

# 124. Cold-Start Safety

A newly recovered or newly provisioned Payments instance SHALL NOT assume every configured HyperSwitch connector is eligible.

Canonical eligibility must be reconstructed or verified first.

---

# 125. Routing Data Consistency

Routing configuration SHALL be versioned or otherwise concurrency-controlled sufficiently to prevent partially applied policy from producing inconsistent execution.

---

# 126. Configuration Change During Payment

A PaymentAttempt SHALL preserve the route chosen at execution time even if routing configuration changes milliseconds later.

New attempts use current eligible configuration.

Historical attempts retain historical route facts.

---

# 127. Provider Removal

Removing a provider from future routing SHALL not destroy information required for:

```text
refunds
disputes
settlement
reconciliation
audit
```

of historical payments.

---

# 128. Connector Version Change

Connector upgrades MAY require:

```text
re-certification
compatibility testing
activation review
```

according to PAY-0021 and PAY-0022.

A new connector version SHALL not automatically inherit all production eligibility if the change is materially significant.

---

# 129. Routing Readiness

Representative readiness gate:

```text
PAYMENT ROUTING READINESS

Canonical context resolution          PASS
Merchant mapping                      PASS
Business Profile mapping              PASS
Provider certification                PASS
Provider activation                   PASS
Market eligibility                    PASS
Currency eligibility                  PASS
Payment-method eligibility            PASS
Transaction capability                PASS
HyperSwitch configuration             PASS
Routing policy                        PASS
Provider health checks                PASS
Circuit breaking                      PASS
Failover classification               PASS
UNKNOWN recovery                      PASS
Duplicate safety                      PASS
Drift detection                       PASS
Audit                                 PASS
Observability                         PASS
Security                              PASS
------------------------------------------
Production Routing                    READY
```

---

# 130. ZuriBeans / Thamani Example

Consider:

```text
Market: ZA
Currency: ZAR
Method: CARD
```

Assume:

```text
Provider A:
  ZuriBeans ZA → ACTIVE
  Thamani ZA   → NOT ACTIVE

Provider B:
  ZuriBeans ZA → ACTIVE
  Thamani ZA   → ACTIVE
```

Then:

```text
ZuriBeans
Eligible = {A, B}
```

while:

```text
Thamani
Eligible = {B}
```

even if both companies:

```text
belong to the same group
use the same Baobab Payments service
operate in ZA
use ZAR
accept CARD
```

Legal-entity activation remains decisive.

---

# 131. External Tenant Example

Suppose:

```text
External Tenant X
 ├── Subsidiary X1
 └── Subsidiary X2
```

Provider C is activated only for X1.

Then:

```text
X1 → Provider C eligible
X2 → Provider C ineligible
```

No special Nabhold-specific routing logic is required.

---

# 132. Safe Failover Example

```text
Payment
   │
   ▼
Attempt 1
Provider A
   │
   X
connector unavailable
BEFORE submission
   │
   ▼
Definitive technical failure
   │
   ▼
Recalculate eligibility
   │
   ▼
Attempt 2
Provider B
   │
   ▼
CAPTURED
```

This is a valid failover pattern.

---

# 133. Unsafe Failover Example

```text
Payment
   │
   ▼
Attempt 1
Provider A
   │
   ▼
Request submitted
   │
   X
network timeout
   │
   ▼
UNKNOWN
   │
   X
DO NOT immediately call Provider B
```

Instead:

```text
UNKNOWN
   │
   ▼
Recover Provider A state
   │
   ├── CAPTURED ───► stop
   │
   ├── FAILED ─────► evaluate failover
   │
   └── UNKNOWN ────► remain protected
```

Availability does not justify duplicate charging.

---

# 134. Hard-Decline Example

```text
Provider A
   │
   ▼
CARD DECLINED
insufficient funds
   │
   ▼
Canonical hard decline
   │
   X
No automatic provider roulette
```

The originating workflow may request:

```text
another payment method
customer action
later retry
```

according to business policy.

---

# 135. Routing Decision Flow

```text
                         PAYMENT
                            │
                            ▼
                   CANONICAL CONTEXT
                            │
                            ▼
                RESOLVE ENGINE INSTANCE
                            │
                            ▼
                 RESOLVE MERCHANT/PROFILE
                            │
                            ▼
                 CERTIFIED PROVIDERS
                            │
                            ▼
                  ACTIVATED PROVIDERS
                            │
                            ▼
                MARKET / CURRENCY FILTER
                            │
                            ▼
                 PAYMENT METHOD FILTER
                            │
                            ▼
                 CAPABILITY / TYPE FILTER
                            │
                            ▼
                  COMPLIANCE FILTER
                            │
                            ▼
                    HEALTH FILTER
                            │
                            ▼
                    ELIGIBLE SET
                            │
                            ▼
                  ROUTING OPTIMISATION
                            │
                            ▼
                    SELECTED ROUTE
                            │
                            ▼
                   PAYMENT ATTEMPT
```

---

# 136. Failover Decision Flow

```text
PaymentAttempt
      │
      ▼
Execution completed?
      │
 ┌────┴────┐
 │         │
YES        NO
 │         │
 ▼         ▼
STOP    Outcome known?
           │
      ┌────┴─────┐
      │          │
     YES         NO
      │          │
      ▼          ▼
Definitive     UNKNOWN
failure?         │
      │          ▼
 ┌────┴───┐    RECOVER
 │        │       │
YES       NO      │
 │        │       X
 ▼        ▼    NO FAILOVER
Recheck   STOP
eligibility
 │
 ▼
Safe to retry?
 │
 ├── NO ─────► STOP
 │
 └── YES
      │
      ▼
Create next
PaymentAttempt
```

---

# 137. Invariants

The following invariants SHALL hold:

1. Eligibility is not routing.
2. Connector existence is not certification.
3. Certification is not activation.
4. Activation is not route selection.
5. A routing algorithm cannot make an ineligible route eligible.
6. Tenant is a hard routing boundary.
7. Legal Entity is a hard routing boundary.
8. Environment is a hard routing boundary.
9. Market eligibility must be explicit.
10. Currency eligibility must be explicit.
11. Payment-method eligibility must be explicit.
12. Required transaction capability must be supported.
13. Group ownership does not imply provider inheritance.
14. Shared Payments infrastructure does not imply shared merchant authority.
15. A healthy connector can still be ineligible.
16. A cheap connector can still be ineligible.
17. A high-success connector can still be ineligible.
18. Unknown eligibility fails closed.
19. Empty eligible route set does not permit arbitrary fallback.
20. Trade does not select PSP routing.
21. Subscriptions does not select PSP routing.
22. Digital estates do not select PSP routing.
23. Control Plane does not become the per-transaction PSP router.
24. HyperSwitch routes only within Baobab-authorised constraints.
25. PaymentAttempt preserves the actual route used.
26. Historical route facts are not rewritten.
27. Provider failover does not erase earlier attempts.
28. `UNKNOWN` blocks unsafe automatic failover.
29. Timeout is not definitive failure.
30. Business decline is not automatically provider failure.
31. `REQUIRES_ACTION` is not a failover trigger.
32. `PROCESSING` is not a failover trigger.
33. Captured payment is not eligible for another collection failover.
34. Authorisation/capture normally remain route-sticky.
35. Refund normally remains route-sticky.
36. Dispute remains associated with the original route.
37. Settlement remains associated with the original route.
38. Provider token portability is never assumed.
39. Mandate portability is never assumed.
40. Cross-provider idempotency is never assumed.
41. Failover eligibility is revalidated.
42. Routing optimisation cannot override compliance.
43. Production never routes through sandbox resources.
44. Simulated payments never move real money.
45. Caller-supplied connector IDs do not establish authority.
46. Manual override cannot bypass hard isolation boundaries.
47. Suspension blocks new execution without destroying history.
48. Routing configuration changes do not rewrite past attempts.
49. No silent cross-legal-entity failover.
50. No silent cross-market failover.
51. No silent currency substitution.
52. No silent payment-method substitution.
53. Unknown external outcomes are resolved conservatively.
54. Availability never takes precedence over duplicate-charge safety.

---

# 138. Consequences

## Positive

This decision:

- preserves tenant and Legal Entity isolation;
- prevents accidental provider misuse;
- makes routing multi-market capable;
- supports multiple PSPs safely;
- enables dynamic HyperSwitch routing;
- enables cost/performance optimisation within governed limits;
- supports provider outages;
- provides safe failover;
- prevents duplicate charges after ambiguous outcomes;
- supports external Baobab tenants;
- preserves provider independence;
- provides explainable routing decisions;
- enables provider suspension without destroying history.

## Negative

It introduces:

- an eligibility-resolution layer;
- governed provider metadata;
- routing-policy management;
- route snapshots;
- health monitoring;
- failure classification;
- recovery before some failovers;
- drift detection;
- additional operational telemetry.

These costs are accepted because uncontrolled failover and provider routing can produce duplicate charges, legal-entity leakage and regulatory violations.

---

# 139. Alternatives Considered

## Let HyperSwitch choose from every configured connector

Rejected.

Configured does not mean authorised for the Baobab business context.

---

## Hard-code provider per Market

Rejected.

Market alone is insufficient and this would couple routing policy to application code.

---

## Let Trade choose the provider

Rejected.

Trade owns commerce, not payment-provider topology.

---

## Let Subscriptions choose the provider

Rejected.

Subscriptions owns billing policy, not payment-provider topology.

---

## Fail over on every provider error

Rejected.

Ambiguous post-submission failures can create duplicate charges.

---

## Fail over on timeout

Rejected as a default.

Timeout does not prove the provider failed to process the transaction.

---

## Share provider activation across subsidiaries automatically

Rejected.

Legal Entities remain financially independent.

---

## Optimise exclusively for cost

Rejected.

Eligibility, compliance, security and duplicate safety take precedence.

---

# 140. Relationship to PAY-0010

PAY-0009 establishes:

```text
WHEN another route may be considered
```

PAY-0010 establishes:

```text
HOW duplicate execution is prevented
```

The relationship is:

```text
PAY-0009
Routing / Failover
      │
      ▼
"Is another attempt permitted?"
      │
      ▼
PAY-0010
Idempotency / Duplicate Safety
      │
      ▼
"Can another attempt be executed
without duplicating money movement?"
```

Both conditions must be satisfied.

---

# 141. Relationship to PAY-0022

PAY-0022 determines whether a provider has passed governance admission:

```text
Provider
   │
   ▼
CERTIFIED
   │
   ▼
ACTIVATED
```

PAY-0009 then determines transaction-level eligibility:

```text
Activated Provider
       │
       ▼
Eligible for THIS context?
       │
       ▼
Candidate Route
```

Thus:

```text
PAY-0022
WHO MAY ENTER THE ROUTING POOL

PAY-0009
WHO MAY SERVE THIS TRANSACTION
```

---

# 142. Complete Routing Architecture

```text
CONTROL PLANE
owns canonical context
      │
      ▼
BAOBAB PAYMENTS
      │
      ├── Merchant/Profile Mapping
      ├── Provider Certification
      ├── Provider Activation
      ├── Market Eligibility
      ├── Currency Eligibility
      ├── Method Eligibility
      ├── Capability Eligibility
      └── Operational Constraints
                 │
                 ▼
          ELIGIBLE ROUTE SET
                 │
                 ▼
             HYPERSWITCH
                 │
                 ├── routing
                 ├── retry
                 └── permitted failover
                 │
                 ▼
          SELECTED CONNECTOR
                 │
                 ▼
              PROVIDER
```

On failure:

```text
PROVIDER FAILURE
      │
      ▼
CLASSIFY OUTCOME
      │
      ├── SUCCESS ─────────────► STOP
      │
      ├── REQUIRES ACTION ─────► WAIT
      │
      ├── PROCESSING ──────────► WAIT
      │
      ├── UNKNOWN ─────────────► RECOVER
      │
      └── DEFINITIVE FAILURE
                    │
                    ▼
            REVALIDATE ELIGIBILITY
                    │
                    ▼
              DUPLICATE SAFE?
                    │
              ┌─────┴─────┐
              │           │
             NO          YES
              │           │
              ▼           ▼
             STOP     NEXT ATTEMPT
```

---

# 143. Final Decision

Baobab payment routing SHALL follow this permanent sequence:

```text
CANONICAL CONTEXT
        │
        ▼
HARD ELIGIBILITY
        │
        ▼
CERTIFIED + ACTIVATED
PROVIDER SET
        │
        ▼
TRANSACTION ELIGIBILITY
        │
        ▼
OPERATIONAL HEALTH
        │
        ▼
ELIGIBLE ROUTE SET
        │
        ▼
ROUTING OPTIMISATION
        │
        ▼
PAYMENT ATTEMPT
```

Failover SHALL follow:

```text
ATTEMPT DID NOT COMPLETE
        │
        ▼
CLASSIFY OUTCOME
        │
        ▼
IS FAILURE DEFINITIVE?
        │
   ┌────┴────┐
   │         │
  NO        YES
   │         │
   ▼         ▼
RECOVER   REVALIDATE
   │       ELIGIBILITY
   │         │
   │         ▼
   │     DUPLICATE SAFE?
   │         │
   │    ┌────┴────┐
   │    │         │
   │   NO        YES
   │    │         │
   ▼    ▼         ▼
 WAIT  STOP   NEXT ATTEMPT
```

The architectural rules are therefore:

```text
Configured
    ≠
Eligible

Certified
    ≠
Activated

Activated
    ≠
Routable for every transaction

Eligible
    ≠
Selected

Timeout
    ≠
Failure

UNKNOWN
    ≠
Permission to fail over

Provider failure
    ≠
Permission to cross legal boundaries

Availability
    ≠
Authority
```

**Baobab determines the permitted payment envelope.  
HyperSwitch optimises only within that envelope.  
A route is used only when it is explicitly eligible for the canonical business context.  
Failover occurs only when the previous outcome is sufficiently known and duplicate execution is safe.  
No availability objective justifies an unauthorised route or a duplicate financial operation.**