# ADR-PAY-0010 — Idempotency, Correlation & Duplicate Safety

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Payment Safety / Distributed Systems |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0009; ADR-PAY-0022; ADR-SHARED-011; applicable Control Plane and Shared contracts |
| **Related** | ADR-PAY-0011; ADR-PAY-0012; ADR-PAY-0013; ADR-PAY-0015; ADR-PAY-0018; ADR-PAY-0019; ADR-PAY-0020 |

---

# 1. Context

Payment execution is inherently distributed.

A single logical operation may traverse:

```text
Digital Estate / Business Engine
              │
              ▼
       baobab-payments
              │
              ▼
         HyperSwitch
              │
              ▼
      Connector / Provider
              │
              ▼
     External Payment Rail
```

At every boundary, the system may experience:

- retries;
- timeouts;
- network failures;
- duplicate API calls;
- concurrent requests;
- process crashes;
- delayed webhooks;
- duplicate webhooks;
- reordered events;
- message redelivery;
- consumer retries;
- failover;
- provider retries;
- ambiguous external outcomes.

For ordinary application operations, a duplicate request may be inconvenient.

For payments, a duplicate request can mean:

```text
CUSTOMER CHARGED TWICE
```

or:

```text
REFUND ISSUED TWICE
```

or:

```text
CAPTURE EXECUTED TWICE
```

Therefore duplicate safety is a fundamental financial invariant.

---

# 2. Problem

Consider:

```text
Trade
  │
  ▼
Create Payment
  │
  ▼
Payments
  │
  ▼
Provider
  │
  ▼
CAPTURED
```

but the response is lost:

```text
Provider
  │
  ▼
CAPTURED
  │
  X
Network failure
```

Trade sees a timeout.

Without idempotency it may retry:

```text
Trade
  │
  ▼
Create Payment again
  │
  ▼
Provider
  │
  ▼
SECOND CAPTURE
```

The customer may now have paid twice.

The same risk exists for:

```text
authorization
capture
refund
void
payout
```

and other financial mutations.

---

# 3. Decision

`baobab-payments` SHALL implement Baobab-level idempotency and duplicate-safety controls independently of HyperSwitch or provider capabilities.

The protection model SHALL distinguish:

```text
Canonical Identity
        ≠
Idempotency Identity
        ≠
Correlation Identity
        ≠
Payment Attempt Identity
        ≠
Provider External Identity
```

These identifiers solve different problems and SHALL NOT be conflated.

---

# 4. Governing Principle

> **A repeated request must never accidentally become a new financial instruction.**

A genuinely new financial attempt SHALL require an explicit, validated decision to create a new attempt.

---

# 5. Exactly-Once Financial Effect

Baobab SHALL NOT claim that distributed payment infrastructure provides exactly-once message delivery.

Instead, Baobab SHALL target:

> **effectively-once canonical processing and duplicate-safe financial effects.**

Conceptually:

```text
AT-LEAST-ONCE DELIVERY
        +
IDEMPOTENT PROCESSING
        +
STATE VALIDATION
        +
EXTERNAL RECOVERY
        +
DUPLICATE DETECTION
        =
DUPLICATE-SAFE FINANCIAL EFFECT
```

---

# 6. Exactly-Once Delivery Is Not Assumed

Across distributed systems:

```text
request sent
response received
event delivered
webhook received
```

cannot universally be guaranteed exactly once.

Baobab SHALL therefore assume duplication and ambiguity are normal failure modes.

---

# 7. Idempotency

An operation is idempotent when repeating the same logical request does not create additional unintended effects.

For example:

```text
Capture Payment X
Idempotency Key K
```

repeated five times SHALL represent:

```text
ONE logical capture request
```

not five captures.

---

# 8. Idempotency Is Operation-Specific

An idempotency key identifies a logical operation.

It SHALL NOT be treated as the canonical identifier of:

```text
PaymentIntent
Payment
PaymentAttempt
Refund
```

Those aggregates retain independent canonical IDs.

---

# 9. Canonical Identity

Canonical IDs identify domain entities.

Examples:

```text
payment_intent_id
payment_id
payment_attempt_id
refund_id
```

These answer:

> Which canonical object is this?

---

# 10. Idempotency Identity

An idempotency key answers:

> Have I already processed this logical command?

Example:

```text
idempotency_key =
"checkout-123-payment-create"
```

The actual key SHOULD normally be opaque rather than semantically parsed.

---

# 11. Correlation Identity

A `correlation_id` answers:

> Which distributed business flow does this operation belong to?

It supports tracing.

It does not establish uniqueness.

Therefore:

```text
correlation_id
      ≠
idempotency_key
```

---

# 12. PaymentAttempt Identity

`payment_attempt_id` answers:

> Which concrete external execution attempt is this?

Multiple attempts may legitimately share the same:

```text
payment_id
correlation_id
```

while having different:

```text
payment_attempt_id
```

---

# 13. Provider Identity

HyperSwitch and external providers may create identifiers such as:

```text
HyperSwitch payment_id
connector transaction_id
acquirer reference
processor reference
```

These remain external references.

They SHALL NOT replace Baobab canonical or idempotency identity.

---

# 14. Identifier Model

Conceptually:

```text
Business Flow
     │
     └── correlation_id
             │
             ▼
       PaymentIntent
       canonical ID
             │
             ▼
          Payment
       canonical ID
             │
        ┌────┴────┐
        │         │
        ▼         ▼
    Attempt 1  Attempt 2
        │         │
        ▼         ▼
 Provider Ref Provider Ref
```

Individual commands within the flow also carry their own idempotency identity.

---

# 15. Idempotency Scope

An idempotency key SHALL be interpreted within a defined scope.

At minimum, scope SHOULD include sufficient authority to prevent cross-context collisions.

Conceptually:

```text
IdempotencyScope
│
├── tenant
├── legal entity where applicable
├── caller/workload
├── operation
└── environment
```

Additional dimensions MAY be included where required.

---

# 16. Tenant Isolation

An idempotency key used by Tenant A SHALL NOT deduplicate an unrelated request from Tenant B.

Therefore:

```text
Tenant A + Key X
```

and:

```text
Tenant B + Key X
```

are distinct scopes.

---

# 17. Legal-Entity Safety

Financial operations SHALL preserve Legal Entity context.

Where relevant:

```text
LegalEntity A + Key X
```

SHALL NOT collide with:

```text
LegalEntity B + Key X
```

---

# 18. Environment Isolation

The same idempotency key used in:

```text
sandbox
```

and:

```text
production
```

SHALL NOT share a financial idempotency record.

---

# 19. Operation Isolation

A key used for:

```text
CREATE_PAYMENT
```

SHALL NOT accidentally deduplicate:

```text
REFUND_PAYMENT
```

The operation forms part of idempotency scope.

---

# 20. Caller-Supplied Idempotency Key

Financial mutation APIs SHOULD require an idempotency key where repeated submission could create duplicate financial effects.

Representative operations include:

```text
CreatePaymentIntent
ExecutePayment
AuthorizePayment
CapturePayment
CancelPayment
CreateRefund
```

Exact API requirements SHALL be governed by the versioned payment contract.

---

# 21. Server-Generated Internal Keys

Internal workflows MAY generate deterministic or opaque idempotency keys for system-generated operations.

The generation scheme SHALL be stable for the same logical operation.

---

# 22. Key Opacity

Consumers SHOULD treat idempotency keys as opaque values.

Business meaning SHALL not depend on parsing a key string.

---

# 23. Key Length and Validation

The API SHALL define:

```text
allowed length
allowed encoding
validation rules
```

to prevent abuse and excessive storage.

---

# 24. Idempotency Record

Payments SHALL persist sufficient idempotency state.

Conceptually:

```text
IdempotencyRecord
│
├── scope
├── idempotency_key
├── operation
├── request_fingerprint
├── status
├── canonical_resource_id
├── response_reference
├── created_at
├── updated_at
└── expiry/retention metadata
```

The exact physical schema is implementation-specific.

---

# 25. Request Fingerprint

Payments SHOULD associate an idempotency key with a stable representation of the request's financially relevant semantics.

For example:

```text
operation
amount
currency
payment_id
legal_entity
market
capture mode
```

as applicable.

Sensitive payment credentials SHALL NOT be unnecessarily stored in the fingerprint.

---

# 26. Same Key, Same Request

If the same key is replayed with semantically equivalent input:

```text
Key K
Request A
```

followed by:

```text
Key K
Request A
```

Payments SHALL return or reconstruct the existing logical result.

It SHALL not create another financial operation.

---

# 27. Same Key, Different Request

If:

```text
Key K
Amount = 100
```

is later replayed as:

```text
Key K
Amount = 500
```

Payments SHALL reject the request as an idempotency conflict.

It SHALL NOT reinterpret the existing key.

---

# 28. Idempotency Conflict

The API SHOULD expose a canonical error equivalent to:

```text
IDEMPOTENCY_KEY_REUSED_WITH_DIFFERENT_REQUEST
```

without leaking sensitive information.

---

# 29. Concurrent Requests

Two identical requests may arrive simultaneously.

Example:

```text
Worker A ───► Capture Payment X
Worker B ───► Capture Payment X
```

Both may carry the same idempotency key.

The persistence layer SHALL ensure that only one becomes the owner of the logical operation.

---

# 30. Atomic Claim

Idempotency ownership SHALL be established atomically.

Conceptually:

```text
INSERT idempotency record
with unique(scope, key)
```

or equivalent transactional/concurrency mechanism.

The implementation SHALL NOT rely solely on:

```text
SELECT
then
INSERT
```

without appropriate concurrency protection.

---

# 31. Concurrent Duplicate Flow

```text
Request A
   │
   ▼
Atomic Idempotency Claim
   │
   ├── won ─────► execute
   │
   └── existing ─► inspect existing operation
```

The losing request SHALL not independently invoke the provider.

---

# 32. In-Progress Duplicate

If the original operation is still executing, a duplicate request MAY:

```text
wait
return processing
return existing operation reference
```

according to API semantics.

It SHALL not start another external financial operation.

---

# 33. Completed Duplicate

If the original operation completed successfully, replay SHOULD return the same logical outcome.

Example:

```text
POST /capture
Key = K
     │
     ▼
CAPTURED
```

later:

```text
POST /capture
Key = K
     │
     ▼
same canonical capture outcome
```

not a second capture.

---

# 34. Failed Duplicate

A previous failure SHALL be interpreted according to failure classification.

A deterministic terminal validation failure MAY safely return the same failure.

An ambiguous external failure SHALL not simply be executed again.

---

# 35. Idempotency Does Not Mean Retry Everything

The following reasoning is prohibited:

```text
Request failed
    │
    ▼
same idempotency key
    │
    ▼
safe to call another provider
```

Cross-provider execution is a new external-risk decision.

PAY-0009 governs whether another attempt is permitted.

---

# 36. Retry Versus New Attempt

Baobab SHALL distinguish:

```text
REPLAY SAME LOGICAL OPERATION
```

from:

```text
CREATE NEW PAYMENT ATTEMPT
```

They are not equivalent.

---

# 37. Replay

Replay means:

> Reprocessing the same logical operation because the caller did not receive a reliable outcome.

Replay SHALL preserve the original idempotency identity.

---

# 38. New Attempt

A new PaymentAttempt means:

> Baobab has explicitly determined that another concrete external execution is permitted and safe.

A new attempt SHALL have a new:

```text
payment_attempt_id
```

and its own external execution identity.

---

# 39. Same-Provider Replay

Where a provider supports idempotency, Baobab MAY replay the same operation to the same provider using the same provider idempotency identity.

Conceptually:

```text
Baobab Attempt A
      │
      ▼
Provider Request P
      │
      X timeout
      │
      ▼
Replay Provider Request P
same provider idempotency identity
```

This remains one semantic PaymentAttempt.

---

# 40. New Provider Means New Attempt

If PAY-0009 authorises failover:

```text
Provider A
     │
     ▼
definitive safe failure
     │
     ▼
Provider B
```

Provider B execution SHALL normally be represented as a new PaymentAttempt.

---

# 41. Provider Idempotency

Provider-native idempotency SHALL be used where supported.

But:

```text
Provider Idempotency
        ≠
Baobab Idempotency
```

Baobab SHALL maintain its own protection regardless of provider capability.

---

# 42. Why Baobab-Level Idempotency Is Required

Provider idempotency may:

- have limited retention;
- use provider-specific semantics;
- be unavailable for some operations;
- not span multiple connectors;
- not span HyperSwitch;
- not protect internal canonical duplication;
- not protect event processing;
- not protect cross-provider failover.

Therefore it cannot be the sole safety mechanism.

---

# 43. HyperSwitch Idempotency

HyperSwitch idempotency MAY form another protection layer.

Conceptually:

```text
Caller
  │
  ▼
Baobab Idempotency
  │
  ▼
HyperSwitch Idempotency
  │
  ▼
Provider Idempotency
```

Each layer has a distinct scope.

---

# 44. Layered Protection

Where all layers support it:

```text
Baobab
  +
HyperSwitch
  +
Provider
```

SHOULD cooperate to reduce duplicate risk.

But Baobab SHALL not assume downstream protection exists.

---

# 45. Cross-Provider Idempotency

The same provider idempotency key SHALL NOT be assumed meaningful across different providers.

Therefore:

```text
Provider A + Key X
```

does not protect:

```text
Provider B + Key X
```

from duplicate financial effect.

---

# 46. PaymentAttempt and Provider Keys

Each PaymentAttempt SHOULD maintain sufficient mapping to:

```text
provider request identity
provider idempotency key
HyperSwitch reference
provider transaction reference
```

where available.

---

# 47. Deterministic Provider Key Derivation

Payments MAY derive provider idempotency identity from stable internal execution identity.

Conceptually:

```text
provider_key =
f(payment_attempt_id, operation)
```

provided the derivation:

- is deterministic;
- does not expose sensitive information;
- respects provider key constraints;
- remains stable across replay.

---

# 48. Different Operations Need Different Keys

For the same Payment:

```text
AUTHORIZE
CAPTURE
REFUND
```

are distinct financial operations.

They SHALL not accidentally share a provider idempotency identity.

---

# 49. Capture Idempotency

A capture command SHALL be protected independently.

Example:

```text
Payment P
Capture 100 ZAR
Key C1
```

repeated SHALL not capture 200 ZAR.

---

# 50. Partial Capture

If multiple partial captures are valid:

```text
Capture 1 = 400 ZAR
Capture 2 = 600 ZAR
```

they are distinct logical operations and require distinct idempotency identities.

Replaying Capture 1 SHALL not create Capture 3.

---

# 51. Refund Idempotency

Each Refund SHALL have:

```text
refund_id
```

and an idempotent creation/execution path.

A repeated request for the same Refund SHALL not create another refund.

---

# 52. Multiple Refunds

Legitimate multiple partial refunds require separate canonical Refund objects.

Example:

```text
Refund R1 = 100
Refund R2 = 50
```

These are not duplicates merely because they refer to the same Payment.

---

# 53. Refund Amount Safety

Before creating a new Refund, Payments SHALL enforce:

```text
total successful refunds
+
pending financially committed refunds
+
requested new refund
≤
refundable amount
```

subject to the detailed refund model in PAY-0013.

---

# 54. Cancellation Idempotency

Repeated cancellation requests SHALL not create multiple financial reversal operations.

If already cancelled, the canonical existing outcome SHOULD be returned.

---

# 55. PaymentIntent Creation

Originating engines SHALL use stable idempotency identity when creating a PaymentIntent for the same business obligation.

This prevents:

```text
Order X
    │
    ├── PaymentIntent A
    └── PaymentIntent B
```

from accidental duplicate request delivery.

---

# 56. Obligation Identity

The originating business engine SHALL provide a stable:

```text
source_engine
source_reference
```

representing the commercial obligation.

Payments MAY enforce uniqueness or duplicate detection rules appropriate to the contract.

However:

```text
source_reference
      ≠
idempotency_key
```

because one obligation may legitimately require multiple payment operations over its lifecycle.

---

# 57. Source Reference Safety

Payments SHOULD detect suspicious duplicate PaymentIntent creation for the same:

```text
tenant
legal entity
source engine
source reference
obligation version where applicable
```

even when callers misuse idempotency keys.

This is defence in depth.

---

# 58. Idempotency Is Not Business Uniqueness

A caller could accidentally use two different idempotency keys for the same logical obligation.

Therefore idempotency alone does not prove business uniqueness.

Domain invariants SHALL provide additional protection.

---

# 59. Defence-in-Depth Model

Duplicate protection SHALL combine:

```text
API idempotency
       +
domain uniqueness
       +
state-machine validation
       +
attempt tracking
       +
provider idempotency
       +
external reconciliation
```

where applicable.

---

# 60. Correlation

Every payment workflow SHALL carry a correlation identifier.

The correlation ID SHOULD propagate across:

```text
Trade / Subscriptions
        │
        ▼
Payments
        │
        ▼
HyperSwitch where supported
        │
        ▼
Provider metadata where safe
        │
        ▼
Events / Logs / Traces
```

---

# 61. Correlation Is Not Authority

A caller SHALL NOT gain access to another Payment by supplying its correlation ID.

Correlation IDs are observability metadata, not authorisation credentials.

---

# 62. Correlation Is Not Deduplication

Two distinct financial operations may share a correlation ID.

Example:

```text
Order Flow Correlation C
   │
   ├── Authorize
   ├── Capture
   └── Refund
```

Therefore:

```text
correlation_id
      ≠
idempotency_key
```

---

# 63. Correlation Hierarchy

Baobab MAY additionally use:

```text
trace_id
span_id
causation_id
```

for observability/event lineage.

These SHALL remain semantically distinct.

---

# 64. Causation

Where canonical events trigger subsequent commands, a `causation_id` or equivalent SHOULD identify the preceding event/command responsible for the action.

Conceptually:

```text
payment.authorized
event E1
    │
    ▼
Capture Command C1
caused_by = E1
```

---

# 65. Event Identity

Every canonical event SHALL have a unique event identifier.

Example:

```text
event_id
```

Consumers SHALL use event identity for duplicate-event processing where appropriate.

---

# 66. Event Idempotency

A duplicate event delivery SHALL not produce duplicate business effects.

Example:

```text
payment.captured event E1
```

delivered twice SHALL not create two accounting projections.

---

# 67. Consumer Responsibility

Consumers of canonical payment events SHALL themselves implement idempotent event consumption.

Payments cannot guarantee that a downstream consumer processes an event exactly once.

---

# 68. Consumer Inbox

Critical consumers SHOULD use an inbox/deduplication pattern or equivalent.

Conceptually:

```text
Event E1
   │
   ▼
Consumer Inbox
   │
   ├── unseen ───► process
   └── seen ─────► ignore/replay result
```

---

# 69. Payments Outbox

Canonical state mutation and event publication SHALL use an outbox or equivalent reliable publication pattern.

Conceptually:

```text
Database Transaction
│
├── update Payment
└── insert Outbox Event
        │
        ▼
commit
        │
        ▼
Publisher
        │
        ▼
Event Bus
```

---

# 70. Why Outbox Is Required

Without an outbox:

```text
Payment CAPTURED
      │
      ▼
database commit
      │
      X
process crash
      │
      ▼
event never published
```

could leave consumers permanently unaware of the financial fact.

---

# 71. Outbox Delivery

Outbox publication MAY be at least once.

Duplicate publication is acceptable provided event identity remains stable and consumers deduplicate.

---

# 72. Stable Event Identity

Retrying publication of the same outbox record SHALL preserve the same canonical:

```text
event_id
```

It SHALL not create a new event identity for every publish retry.

---

# 73. Webhook Duplication

Provider and HyperSwitch webhooks SHALL be assumed to be:

```text
duplicated
delayed
reordered
replayed
```

PAY-0011 governs webhook validation.

PAY-0010 requires duplicate-safe canonical processing.

---

# 74. Webhook Identity

Where an external webhook provides a stable external event ID, Payments SHOULD retain and deduplicate it within the appropriate provider scope.

---

# 75. Missing External Event ID

If no stable webhook ID exists, Payments SHALL use appropriate deduplication strategies such as:

```text
provider reference
event type
provider sequence
payload fingerprint
canonical state validation
```

without relying solely on timestamps.

---

# 76. Duplicate Webhook

If:

```text
provider.capture_succeeded
```

arrives twice for the same external operation, canonical processing SHALL produce one semantic capture fact.

---

# 77. Reordered Webhook

If:

```text
CAPTURED
```

arrives before:

```text
PROCESSING
```

the later stale PROCESSING event SHALL not regress canonical financial state.

PAY-0008 state-machine rules remain authoritative.

---

# 78. Lost HTTP Response

A lost response SHALL not cause automatic creation of a new PaymentAttempt.

Conceptually:

```text
Payments
   │
   ▼
Provider
   │
   ▼
operation possibly succeeded
   │
   X
response lost
   │
   ▼
UNKNOWN
```

Recovery is required.

---

# 79. Ambiguous Outcome

An ambiguous external result SHALL transition the attempt into an appropriate:

```text
UNKNOWN
PROCESSING
```

condition rather than terminal failure.

---

# 80. Recovery

Recovery SHOULD attempt to determine whether the original external operation exists.

Mechanisms MAY include:

```text
provider query by external reference
HyperSwitch query
idempotent replay
webhook correlation
settlement/reconciliation evidence
```

depending on operation and provider capability.

---

# 81. Recovery Identity

Recovery SHALL use the original PaymentAttempt and provider execution identity.

It SHALL not silently manufacture a new attempt.

---

# 82. UNKNOWN and Failover

PAY-0009 established:

```text
UNKNOWN
    │
    X
automatic cross-provider failover
```

unless duplicate safety can be established.

PAY-0010 confirms this as a duplicate-safety invariant.

---

# 83. Definitive Failure

A new PaymentAttempt MAY be created only after the previous attempt is known sufficiently not to have produced the conflicting financial effect, or another explicit safe mechanism exists.

---

# 84. New Attempt Decision

The decision to create another attempt SHOULD be explicit and auditable.

Conceptually:

```text
Previous Attempt
      │
      ▼
Outcome Classification
      │
      ▼
Failover Permitted?
      │
      ▼
Duplicate Safe?
      │
      ▼
Create New Attempt
```

---

# 85. New Attempt Is Not Idempotency Replay

Once a new attempt is deliberately created:

```text
Attempt 1
Attempt 2
```

these are separate execution identities.

Idempotency prevents accidental duplication of each attempt; it does not collapse legitimate failover attempts into one.

---

# 86. Duplicate Attempt Creation

The command that creates a new PaymentAttempt SHALL itself be idempotent.

Otherwise concurrent failover workers could produce:

```text
Attempt 2
Attempt 3
```

simultaneously.

---

# 87. Attempt Sequence Concurrency

Payments SHALL use locking, optimistic concurrency, unique constraints, transactional state transitions or equivalent mechanisms to prevent concurrent unsafe attempt creation.

---

# 88. Payment Revision

PAY-0008 established aggregate concurrency.

A Payment SHOULD maintain:

```text
revision
```

or equivalent concurrency protection.

New attempt creation SHALL validate the expected aggregate state.

---

# 89. Concurrent Capture

Consider:

```text
Worker A ─► Capture Payment P
Worker B ─► Capture Payment P
```

Both may arrive with different accidental idempotency keys.

State-machine and financial amount invariants SHALL still prevent unsafe duplicate capture where possible.

Idempotency keys are not the only line of defence.

---

# 90. Aggregate Locking

Critical financial transitions MAY require transaction-level serialization for a given Payment.

Conceptually:

```text
Payment P
    │
    ▼
serialize financial mutation
    │
    ├── Capture A
    └── Capture B waits/re-evaluates
```

---

# 91. Amount Invariants

Before capture, Payments SHALL evaluate:

```text
captured amount
+
financially committed capture amount
+
new capture amount
≤
capturable amount
```

where applicable.

This protects against distinct-key concurrent requests.

---

# 92. Financially Committed Operations

Operations with uncertain external outcomes MAY need to reserve/commit the relevant amount locally until resolved.

Otherwise another worker could incorrectly conclude that the amount remains available.

---

# 93. UNKNOWN Amount Reservation

If a 100 ZAR capture is `UNKNOWN`, Payments SHOULD NOT assume the 100 ZAR remains safely capturable elsewhere.

Conceptually:

```text
Authorized = 100

Attempt A
Capture 100
UNKNOWN

Available for another capture
≠ 100
```

until recovery establishes the outcome.

---

# 94. Refund Reservation

Likewise, a refund submitted externally but not yet resolved may need to reduce available refundable balance for duplicate-safety purposes.

---

# 95. Reservation Is Not Final Financial Fact

A local safety reservation SHALL not falsely report:

```text
CAPTURED
REFUNDED
```

before authoritative outcome exists.

It is a concurrency/safety mechanism, not an external financial fact.

---

# 96. Command Lifecycle

A financially significant command MAY conceptually progress:

```text
RECEIVED
   │
   ▼
CLAIMED
   │
   ▼
VALIDATED
   │
   ▼
EXTERNAL_EXECUTION
   │
   ├── SUCCEEDED
   ├── FAILED
   └── UNKNOWN
```

Idempotency state SHOULD preserve enough information for safe replay.

---

# 97. Idempotency Record State

A conceptual record MAY have:

```text
IN_PROGRESS
SUCCEEDED
FAILED_FINAL
UNKNOWN
```

or equivalent implementation states.

These are idempotency-processing states, not Payment states.

---

# 98. Idempotency State Is Not Payment State

The following distinction SHALL hold:

```text
IdempotencyRecord.status
        ≠
Payment.status
```

An idempotency record tracks command processing.

A Payment tracks canonical payment execution.

---

# 99. Crash Before External Submission

If Payments crashes after claiming the idempotency key but before external submission, recovery SHALL determine that no external financial operation occurred before safely resuming.

---

# 100. Crash After External Submission

If Payments crashes after external submission but before persisting the response, recovery SHALL assume the external outcome may exist.

It SHALL NOT blindly execute again.

---

# 101. Critical Failure Window

The most dangerous window is:

```text
External provider succeeds
         │
         ▼
Payments process crashes
         │
         X
before local success persisted
```

The system SHALL be designed specifically for this case.

---

# 102. External Reference Before Execution

Where provider/HyperSwitch APIs permit, Payments SHOULD establish a stable external request reference before or as part of submission.

This improves recovery after ambiguous failures.

---

# 103. Persist Before Side Effect

Payments SHOULD persist sufficient execution intent before invoking an external financial side effect.

Conceptually:

```text
persist PaymentAttempt
persist execution identity
commit
      │
      ▼
invoke provider
```

rather than invoking the provider with no recoverable local identity.

---

# 104. Local Transaction Boundary

Database transactions SHALL protect local canonical consistency.

They SHALL NOT be held open across slow external provider calls merely to simulate a distributed transaction.

---

# 105. No Distributed Database Transaction Requirement

Baobab SHALL not require a two-phase commit spanning:

```text
Baobab PostgreSQL
HyperSwitch
Provider
```

Correctness SHALL instead come from:

```text
durable intent
idempotency
recovery
state machines
reconciliation
```

---

# 106. Saga-Like Recovery

Financial workflows MAY use saga-like recovery patterns.

However, compensation SHALL respect financial semantics.

A second financial operation is not automatically a rollback.

For example:

```text
capture
then refund
```

is not equivalent to:

```text
capture never happened
```

---

# 107. Idempotency Retention

Idempotency records SHALL be retained long enough to cover realistic:

```text
client retries
network delays
provider processing
webhook delays
recovery
operational incidents
```

Financially significant idempotency records SHOULD not use arbitrarily short expiry periods.

---

# 108. Retention Policy

Exact retention SHALL be determined through:

```text
operation type
provider behaviour
regulatory requirements
audit requirements
storage constraints
```

and documented operationally.

---

# 109. Expired Key Reuse

Reusing an expired idempotency key SHALL not silently create duplicate financial effects if domain state still reveals that the operation already occurred.

Domain invariants remain the final defence.

---

# 110. Idempotency Garbage Collection

Cleanup SHALL not destroy canonical financial records or external-reference mappings required for reconciliation.

Idempotency metadata and canonical financial history have different retention concerns.

---

# 111. Security

Idempotency keys SHALL not be treated as secrets or authentication credentials.

Possession of a key SHALL not grant access to the associated Payment.

---

# 112. Key Guessing

APIs SHALL enforce normal authentication and authorisation before exposing idempotent operation results.

A guessed key cannot be used to retrieve another tenant's transaction.

---

# 113. Key Scope Validation

The authenticated canonical context SHALL determine the idempotency scope.

Caller-supplied tenant or Legal Entity identifiers SHALL not override trusted resolved context.

---

# 114. Sensitive Data

Idempotency records SHALL NOT unnecessarily contain:

```text
PAN
CVV
raw bank credentials
raw wallet secrets
```

PAY-0012 governs sensitive payment information.

---

# 115. Logging

Logs MAY contain:

```text
idempotency key hash/reference
correlation_id
payment_id
payment_attempt_id
operation
result
```

but SHOULD avoid exposing unnecessarily reusable external identifiers or sensitive values.

---

# 116. Observability

Payments SHOULD make idempotency behaviour observable.

Representative telemetry:

```text
idempotency.claimed
idempotency.replayed
idempotency.conflict
idempotency.in_progress
duplicate.prevented
duplicate.suspected
attempt.recovery.started
attempt.recovery.completed
```

---

# 117. Metrics

Useful metrics MAY include:

```text
idempotency hit rate
idempotency conflict rate
concurrent duplicate rate
duplicate prevented count
ambiguous operation count
UNKNOWN duration
provider replay count
recovery success rate
new-attempt-after-recovery count
event duplicate rate
webhook duplicate rate
```

---

# 118. Alerting

Operational alerts SHOULD detect:

```text
unexpected duplicate payment references
multiple captures exceeding amount
multiple refunds exceeding refundable amount
high UNKNOWN rate
idempotency conflict spike
provider duplicate transaction evidence
recovery backlog
outbox backlog
```

---

# 119. Duplicate Detection

Payments SHOULD support retrospective duplicate detection using signals such as:

```text
same canonical Payment
same amount
same currency
close timestamps
same payment method reference
multiple provider references
same source obligation
```

Such detection is a safety/operations mechanism.

It SHALL not automatically declare two legitimate payments duplicates without sufficient evidence.

---

# 120. Reconciliation

Settlement/provider reconciliation SHALL serve as an additional duplicate-detection layer.

For example:

```text
Baobab expects one capture
Provider reports two
```

SHALL generate a reconciliation exception.

PAY-0015 governs reconciliation.

---

# 121. Duplicate Incident

A suspected duplicate financial effect SHALL be treated as a high-severity operational condition.

The system SHOULD preserve:

```text
all attempts
all provider references
all idempotency records
all events
all correlation data
```

for investigation.

---

# 122. Automatic Compensation

Baobab SHALL NOT automatically refund a suspected duplicate unless policy and evidence establish that compensation is appropriate.

An automatic refund itself is another financial operation and carries risk.

---

# 123. Manual Recovery

Privileged operators MAY initiate controlled recovery actions.

Such actions SHALL require:

```text
authentication
authorisation
reason
audit
correlation
state validation
```

under PAY-0018.

---

# 124. Manual Retry

A manual “retry” button SHALL not simply invoke the provider again.

It SHALL execute the same safety decision process as automated recovery:

```text
current state
      │
      ▼
previous attempt outcome
      │
      ▼
duplicate safe?
      │
      ▼
retry / new attempt / reject
```

---

# 125. Scheduled Recovery

Recovery workers MAY periodically examine:

```text
UNKNOWN
PROCESSING
stuck external operations
```

and attempt safe reconciliation.

They SHALL not bypass the state machine or idempotency layer.

---

# 126. Recovery Worker Concurrency

Multiple recovery workers SHALL not simultaneously create multiple new attempts for the same Payment.

Distributed locking, aggregate concurrency or equivalent protection SHALL apply.

---

# 127. Multi-Region Deployment

If Payments operates across multiple regions, idempotency guarantees SHALL remain valid across the applicable write topology.

A request routed to Region B after Region A becomes unavailable SHALL not accidentally become a second financial operation.

---

# 128. Regional Failover

Production architecture SHALL therefore define a consistent strategy for:

```text
idempotency ownership
canonical Payment writes
attempt creation
regional failover
```

PAY-0020 governs broader disaster recovery.

---

# 129. Split-Brain Safety

Payment availability SHALL not be increased by permitting two regions to independently believe they own the same financial mutation.

During uncertainty:

```text
fail closed / recover
```

is preferable to:

```text
charge twice
```

---

# 130. Database Constraints

The persistence layer SHOULD enforce duplicate safety with constraints where appropriate.

Examples MAY include:

```text
UNIQUE(idempotency_scope, idempotency_key)
UNIQUE(provider_scope, external_event_id)
```

and domain-specific uniqueness constraints.

Application-level checks alone SHALL not be the only protection against concurrent duplicates.

---

# 131. Transaction Isolation

Financial mutation transactions SHALL use database concurrency controls appropriate to the invariant being protected.

The implementation SHALL not assume that ordinary read-then-write logic is sufficient under concurrent execution.

---

# 132. Canonical Resource Creation

Where an idempotent create operation wins the atomic claim:

```text
Idempotency Record
      │
      ▼
Canonical Resource
```

the relationship SHOULD be durably recorded so later replays return the same resource.

---

# 133. Response Replay

Payments MAY persist or reconstruct the original canonical response for completed idempotent requests.

Replay SHALL reflect the same logical operation.

---

# 134. Current State Versus Original Response

APIs SHALL define whether an idempotent replay returns:

```text
the original response
```

or:

```text
the current representation of the same resource
```

This SHALL be consistent and documented.

It SHALL never trigger another financial effect merely to reconstruct the response.

---

# 135. Error Replay

Stable validation failures MAY be replayed.

Transient infrastructure errors SHOULD be distinguished from final domain outcomes.

---

# 136. Client Timeout Guidance

Clients SHALL be designed so that a timeout leads to:

```text
retry same logical request
with same idempotency key
```

rather than:

```text
generate new key
and try again
```

unless intentionally creating a new business operation.

---

# 137. SDK Behaviour

Future Baobab Payments SDKs SHOULD automatically preserve idempotency identity across safe transport retries.

They SHALL not silently generate a fresh key after a timeout.

---

# 138. Trade Integration

Trade SHALL provide stable idempotency keys for financial commands.

Example:

```text
Order O
Payment Collection C
Capture Command C1
```

Transport retries of C1 SHALL preserve the same idempotency identity.

---

# 139. Subscription Integration

Subscriptions SHALL similarly preserve command identity for a billing collection attempt.

Scheduler reruns SHALL not accidentally create another payment for the same collection instruction.

---

# 140. Scheduler Safety

A recurring billing scheduler may run twice because of:

```text
worker retry
leader failover
deployment
clock overlap
message redelivery
```

The same billing obligation SHALL not therefore become two unintended collections.

---

# 141. INTERNAL Subscription Boundary

Per ADR-SHARED-011, INTERNAL/zero-charge subscriptions do not invoke Payments.

Idempotency SHALL not be used to disguise a payment call that should not exist.

---

# 142. ERP Integration

ERP event consumption SHALL deduplicate canonical payment events.

A duplicate:

```text
payment.captured
```

event SHALL not create duplicate accounting consequences.

---

# 143. Accounting Identity

ERP accounting document identity remains separate from:

```text
payment_id
idempotency_key
event_id
```

but SHALL preserve references necessary for duplicate-safe projection.

---

# 144. Canonical Duplicate-Safety Flow

```text
CLIENT / ENGINE
      │
      │ command + idempotency key
      ▼
AUTHENTICATE / AUTHORISE
      │
      ▼
RESOLVE CANONICAL CONTEXT
      │
      ▼
BUILD REQUEST FINGERPRINT
      │
      ▼
ATOMIC IDEMPOTENCY CLAIM
      │
 ┌────┴──────────┐
 │               │
NEW            EXISTING
 │               │
 ▼               ▼
VALIDATE      SAME REQUEST?
 │           ┌────┴────┐
 ▼           │         │
PERSIST     YES        NO
INTENT       │          │
 │           ▼          ▼
 ▼       RETURN /    REJECT
EXTERNAL   RESUME     CONFLICT
EXECUTION
 │
 ├── SUCCESS
 │      │
 │      ▼
 │   PERSIST FACT
 │
 ├── DEFINITIVE FAILURE
 │      │
 │      ▼
 │   PERSIST FAILURE
 │
 └── AMBIGUOUS
        │
        ▼
      UNKNOWN
        │
        ▼
      RECOVER
```

---

# 145. Crash-Safe Execution Flow

```text
Create durable PaymentAttempt
          │
          ▼
Create provider execution identity
          │
          ▼
COMMIT LOCAL INTENT
          │
          ▼
Call HyperSwitch / Provider
          │
    ┌─────┼──────────┐
    │     │          │
SUCCESS FAILURE   RESPONSE LOST
    │     │          │
    ▼     ▼          ▼
Persist Persist    UNKNOWN
Fact    Failure       │
                      ▼
                   RECOVER
```

---

# 146. New-Attempt Decision Flow

```text
Current Attempt
      │
      ▼
Outcome definitive?
      │
 ┌────┴────┐
 │         │
NO        YES
 │         │
 ▼         ▼
RECOVER   Did financial
 │        effect occur?
 │         │
 │    ┌────┴────┐
 │    │         │
 │   YES        NO
 │    │         │
 ▼    ▼         ▼
WAIT STOP    PAY-0009
             routing eligibility
                  │
                  ▼
             duplicate safe?
                  │
             ┌────┴────┐
             │         │
            NO        YES
             │         │
             ▼         ▼
            STOP    NEW ATTEMPT
```

---

# 147. Identity Relationship

```text
correlation_id
│
└── distributed business flow
      │
      └── payment_intent_id
           │
           └── payment_id
                │
                ├── payment_attempt_id A
                │      └── provider reference A
                │
                └── payment_attempt_id B
                       └── provider reference B

Each command:
    idempotency_key

Each event:
    event_id

Each causal relationship:
    causation_id where applicable
```

No identifier substitutes for another.

---

# 148. Production Readiness

Representative duplicate-safety gate:

```text
IDEMPOTENCY & DUPLICATE SAFETY

Canonical IDs                         PASS
Correlation propagation               PASS
Idempotency API contract              PASS
Scoped idempotency keys               PASS
Request fingerprinting                PASS
Atomic key claim                      PASS
Concurrent duplicate protection       PASS
Same-key conflict detection           PASS
Payment aggregate concurrency         PASS
PaymentAttempt concurrency            PASS
Amount invariants                     PASS
Unknown-state reservation             PASS
Provider idempotency integration      PASS
HyperSwitch idempotency integration   PASS
Lost-response recovery                PASS
UNKNOWN-state handling                PASS
Safe failover integration             PASS
Webhook deduplication                 PASS
Event identity                        PASS
Outbox                                PASS
Consumer inbox guidance               PASS
Recovery workers                      PASS
Duplicate detection                   PASS
Reconciliation                        PASS
Observability                         PASS
Audit                                 PASS
------------------------------------------
Duplicate Safety                      READY
```

---

# 149. Invariants

The following invariants SHALL hold:

1. Canonical identity is not idempotency identity.
2. Idempotency identity is not correlation identity.
3. Correlation identity is not PaymentAttempt identity.
4. Provider identity is not canonical identity.
5. A repeated logical command does not create a new financial instruction.
6. Same idempotency key plus same request maps to the same logical operation.
7. Same idempotency key plus materially different request is rejected.
8. Idempotency scope includes trusted business/security context.
9. Idempotency never crosses tenant boundaries accidentally.
10. Idempotency never crosses production/sandbox boundaries accidentally.
11. Financially distinct operations use distinct idempotency identities.
12. Idempotency ownership is claimed atomically.
13. Concurrent duplicate requests cannot independently invoke the provider.
14. Provider idempotency does not replace Baobab idempotency.
15. HyperSwitch idempotency does not replace Baobab idempotency.
16. Cross-provider idempotency is never assumed.
17. Retry is not automatically a new PaymentAttempt.
18. New PaymentAttempt is not an idempotency replay.
19. Same-provider idempotent transport replay may remain the same PaymentAttempt.
20. Cross-provider failover normally creates a new PaymentAttempt.
21. New-attempt creation is itself idempotent.
22. Timeout does not prove financial failure.
23. Lost response does not authorise another charge.
24. `UNKNOWN` does not authorise automatic failover.
25. Recovery precedes new execution when the previous outcome is ambiguous.
26. An unresolved capture reserves duplicate-risk capacity.
27. An unresolved refund reserves duplicate-risk capacity.
28. A safety reservation is not a final financial fact.
29. Distinct idempotency keys cannot bypass aggregate financial invariants.
30. Captured amount cannot exceed permitted capturable amount through concurrency.
31. Refunded amount cannot exceed permitted refundable amount through concurrency.
32. Payment state transitions remain concurrency-controlled.
33. Duplicate webhooks produce one semantic financial fact.
34. Reordered webhooks cannot regress established financial facts.
35. Duplicate event delivery does not imply duplicate business effect.
36. Canonical events have stable event identities.
37. Outbox retries preserve event identity.
38. Consumers are responsible for idempotent event processing.
39. Correlation IDs never provide authorisation.
40. Idempotency keys never provide authentication.
41. Sensitive payment credentials are not stored merely for idempotency.
42. Canonical financial history survives idempotency cleanup.
43. Regional failover cannot create two independent financial writers.
44. Manual retry obeys the same duplicate-safety rules as automated retry.
45. Scheduler retries do not create duplicate collections.
46. Reconciliation remains an independent duplicate-detection defence.
47. Provider success followed by local crash is treated as a recoverable ambiguity, not automatic failure.
48. Availability does not take precedence over duplicate-charge prevention.

---

# 150. Consequences

## Positive

This decision:

- prevents duplicate charges;
- prevents duplicate captures;
- prevents duplicate refunds;
- supports safe client retries;
- supports safe scheduler retries;
- supports concurrent workers;
- makes webhook redelivery safe;
- makes event redelivery safe;
- provides recovery from lost responses;
- strengthens PAY-0009 failover;
- supports multi-provider execution;
- improves auditability;
- provides a basis for multi-region payment safety.

## Negative

It introduces:

- persistent idempotency records;
- request fingerprinting;
- uniqueness constraints;
- concurrency control;
- amount reservations;
- recovery workers;
- event inbox/outbox patterns;
- additional provider-reference tracking;
- longer-lived operational state.

These costs are accepted because accidental duplicate financial effects are materially more expensive than the additional infrastructure.

---

# 151. Alternatives Considered

## Rely exclusively on HyperSwitch idempotency

Rejected.

Baobab requires protection before, around and after HyperSwitch.

---

## Rely exclusively on PSP idempotency

Rejected.

Provider semantics, scope and retention differ, and cross-provider protection is absent.

---

## Use Payment ID as idempotency key

Rejected.

One Payment can legitimately have multiple distinct commands.

---

## Use correlation ID as idempotency key

Rejected.

A correlation ID can span many legitimate operations.

---

## Generate a new key on every retry

Rejected.

That converts transport retries into new financial instructions.

---

## Treat timeout as failure and retry

Rejected.

The external financial effect may already have occurred.

---

## Depend only on application-level duplicate checks

Rejected.

Concurrent execution requires database-level or equivalent atomic enforcement.

---

## Guarantee exactly-once delivery

Rejected as a distributed-system assumption.

Baobab instead provides idempotent processing and duplicate-safe financial effects.

---

# 152. Relationship to PAY-0008 and PAY-0009

The three decisions form a safety chain:

```text
PAY-0008
CANONICAL STATE
     │
     ▼
What happened?

PAY-0009
ROUTING / FAILOVER
     │
     ▼
May another route be considered?

PAY-0010
IDEMPOTENCY / DUPLICATE SAFETY
     │
     ▼
Can execution occur without
creating a duplicate financial effect?
```

All three must agree before a new PaymentAttempt executes.

---

# 153. Relationship to PAY-0011

PAY-0011 SHALL define webhook and event delivery semantics.

PAY-0010 establishes the prerequisite:

```text
DELIVERY MAY DUPLICATE
       │
       ▼
PROCESSING MUST DEDUPLICATE
```

Therefore PAY-0011 SHALL not require exactly-once transport.

---

# 154. Relationship to PAY-0013

Refund execution SHALL reuse these principles:

```text
canonical refund identity
idempotent refund command
provider execution identity
amount reservation
UNKNOWN recovery
duplicate-safe event processing
```

Refunding twice is treated with the same seriousness as charging twice.

---

# 155. Relationship to PAY-0015

Settlement reconciliation provides an independent evidence channel capable of revealing:

```text
missing capture
duplicate capture
missing refund
duplicate refund
unexpected provider transaction
```

Idempotency prevents duplicates prospectively.

Reconciliation detects discrepancies retrospectively.

Both are required.

---

# 156. Final Duplicate-Safety Model

```text
                    BUSINESS COMMAND
                           │
                           ▼
                  CANONICAL CONTEXT
                           │
                           ▼
                    IDEMPOTENCY
                           │
                           ▼
                  DOMAIN INVARIANTS
                           │
                           ▼
                CONCURRENCY CONTROL
                           │
                           ▼
                   PAYMENT ATTEMPT
                           │
                           ▼
              DOWNSTREAM IDEMPOTENCY
                           │
                           ▼
                 EXTERNAL EXECUTION
                           │
             ┌─────────────┼─────────────┐
             │             │             │
             ▼             ▼             ▼
          SUCCESS      FAILURE        UNKNOWN
             │             │             │
             ▼             ▼             ▼
       CANONICAL FACT   RECORD       RECOVERY
             │                         │
             ▼                         ▼
           OUTBOX                DETERMINE FACT
             │                         │
             ▼                         ▼
           EVENT                 SAFE NEXT STEP
             │
             ▼
       IDEMPOTENT CONSUMER
```

---

# 157. Final Decision

Baobab SHALL treat duplicate safety as a layered financial control:

```text
IDEMPOTENCY
     +
DOMAIN INVARIANTS
     +
CONCURRENCY CONTROL
     +
PAYMENT ATTEMPT IDENTITY
     +
PROVIDER IDEMPOTENCY
     +
STATE RECOVERY
     +
EVENT DEDUPLICATION
     +
RECONCILIATION
```

No single mechanism is sufficient.

The permanent identity distinctions are:

```text
CANONICAL ID
=
which domain object?

IDEMPOTENCY KEY
=
is this the same logical command?

CORRELATION ID
=
which distributed workflow?

PAYMENT ATTEMPT ID
=
which concrete execution attempt?

EVENT ID
=
which immutable fact notification?

PROVIDER REFERENCE
=
which external provider object?
```

And the critical execution distinction is:

```text
REPLAY
=
same logical operation,
same financial intent

NEW ATTEMPT
=
new concrete external execution,
created only after explicit safety validation
```

Therefore:

```text
RETRY
      ≠
NEW PAYMENT

TIMEOUT
      ≠
FAILURE

NEW IDEMPOTENCY KEY
      ≠
PERMISSION TO CHARGE AGAIN

PROVIDER IDEMPOTENCY
      ≠
BAOBAB DUPLICATE SAFETY

AT-LEAST-ONCE DELIVERY
      ≠
AT-LEAST-ONCE FINANCIAL EFFECT
```

**Baobab assumes requests, messages and webhooks can repeat.  
It assumes responses can disappear.  
It assumes workers can race and processes can crash.  
It therefore makes duplication harmless wherever possible and ambiguity conservative wherever it cannot.  
A new external financial attempt is created only when Baobab can establish that doing so is both authorised and duplicate-safe.**