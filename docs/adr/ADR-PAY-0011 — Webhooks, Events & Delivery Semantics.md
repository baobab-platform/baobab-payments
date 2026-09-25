# ADR-PAY-0011 — Webhooks, Events & Delivery Semantics

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Integration / Eventing / Reliability |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0010; ADR-PAY-0022; ADR-SHARED-011; applicable Control Plane and Shared contracts |
| **Related** | ADR-PAY-0012; ADR-PAY-0013; ADR-PAY-0014; ADR-PAY-0015; ADR-PAY-0018; ADR-PAY-0019; ADR-PAY-0020; ADR-PAY-0021 |

---

# 1. Context

Payment execution spans multiple independently operating systems:

```text
Business Engine
     │
     ▼
Baobab Payments
     │
     ▼
HyperSwitch
     │
     ▼
Connector / PSP / Acquirer / Rail
```

Some payment operations complete synchronously.

Many do not.

A payment may remain:

```text
REQUIRES_ACTION
PROCESSING
AUTHORIZED
UNKNOWN
```

until an external system later reports a change.

Such changes may arrive through:

- HyperSwitch webhooks;
- provider callbacks;
- status-query recovery;
- scheduled reconciliation;
- settlement files;
- operational recovery;
- asynchronous payment rails.

Baobab therefore requires a reliable mechanism for converting external payment observations into trusted canonical financial facts and distributing those facts to downstream engines.

---

# 2. Problem

External webhook delivery cannot be assumed to be:

```text
exactly once
ordered
immediate
complete
authentic without verification
canonical
```

A provider may send:

```text
Event A
Event A
Event B
Event A
Event C
```

or:

```text
CAPTURED
PROCESSING
```

in an order that does not correspond to Baobab's canonical lifecycle.

An attacker may also attempt to submit a forged callback.

Therefore:

> Receiving a webhook does not mean that the claimed financial fact is true.

---

# 3. Decision

Baobab SHALL implement a strict separation between:

```text
EXTERNAL OBSERVATION
```

and:

```text
CANONICAL BAOBAB EVENT
```

The processing pipeline SHALL conceptually be:

```text
External Callback
       │
       ▼
Webhook Ingress
       │
       ▼
Authentication / Verification
       │
       ▼
External Event Deduplication
       │
       ▼
Context / Mapping Resolution
       │
       ▼
Canonical State Validation
       │
       ▼
Transactional State Mutation
       │
       ▼
Outbox
       │
       ▼
Canonical Baobab Event
       │
       ▼
Event Transport
       │
       ▼
Idempotent Consumers
```

Raw provider callbacks SHALL NOT be published directly as canonical Baobab events.

---

# 4. Governing Principle

> **Webhooks report claims about external payment activity. Baobab Payments validates those claims before they become canonical financial facts.**

Therefore:

```text
PROVIDER EVENT
      ≠
CANONICAL EVENT
```

and:

```text
WEBHOOK RECEIVED
      ≠
PAYMENT STATE CHANGED
```

---

# 5. Event Authority

Authority SHALL remain separated.

| Information | Authority |
|---|---|
| External provider observation | Provider / HyperSwitch |
| External event authenticity | Payments validation |
| Mapping to canonical Payment | Payments |
| Canonical payment lifecycle | Payments |
| Canonical Refund lifecycle | Payments |
| Canonical dispute observation | Payments |
| Canonical settlement observation | Payments |
| Commerce consequence | Trade |
| Subscription consequence | Subscriptions |
| Accounting consequence | ERP |
| Tenant / Legal Entity / Market context | Control Plane |
| Identity and access | IAM |

---

# 6. Webhook Ingress

External callbacks SHALL enter through dedicated Payments ingress endpoints.

Conceptually:

```text
Internet / Provider
       │
       ▼
Payments Webhook Edge
       │
       ▼
Provider-Specific Verification
       │
       ▼
Internal Normalisation
```

Digital estates SHALL NOT expose provider webhook endpoints as canonical payment authorities.

---

# 7. HyperSwitch Webhooks

Where HyperSwitch is the integration boundary:

```text
Provider
   │
   ▼
HyperSwitch
   │
   ▼
baobab-payments
```

Baobab Payments SHALL consume the appropriate HyperSwitch callback/event interface.

Trade, Subscriptions and ERP SHALL not independently consume HyperSwitch payment callbacks as payment authority.

---

# 8. Direct Provider Webhooks

If a future connector requires provider callbacks to Baobab directly, those callbacks SHALL still enter through Payments.

The same validation, deduplication and canonicalisation rules apply.

---

# 9. No Direct Webhook to Trade

The following architecture is prohibited:

```text
Provider
   │
   ▼
Trade
   │
   ▼
mark order paid
```

Instead:

```text
Provider
   │
   ▼
HyperSwitch / Payments
   │
   ▼
Canonical Payment State
   │
   ▼
payment.captured
   │
   ▼
Trade
```

---

# 10. No Direct Webhook to Subscriptions

Subscriptions SHALL consume canonical Payments facts.

It SHALL NOT independently determine successful collection from provider callbacks.

---

# 11. No Direct Webhook to ERP

ERP SHALL consume governed financial facts or reconciliation integration.

Raw provider webhook payloads SHALL NOT become accounting entries directly.

---

# 12. Webhooks Are Untrusted Input

Every webhook SHALL be treated as hostile or untrusted until verified.

Validation MAY include:

```text
signature
shared secret / cryptographic verification
timestamp
provider identity
event identifier
expected endpoint
payload integrity
merchant/profile context
external transaction mapping
```

according to provider capabilities.

---

# 13. Signature Verification

Where the webhook source supports signatures, signature verification SHALL occur before financial state mutation.

A failed signature SHALL result in rejection.

---

# 14. Verification Before Parsing Business Meaning

Where technically practical:

```text
receive raw payload
      │
      ▼
verify authenticity
      │
      ▼
interpret financial semantics
```

is preferred over trusting parsed fields before authenticity is established.

---

# 15. Raw Payload Integrity

If signature verification depends on the exact request body, the original bytes SHALL be available for verification.

Middleware SHALL NOT irreversibly transform the payload before signature validation.

---

# 16. Timestamp Validation

Where supported, webhook timestamps SHOULD be validated against an acceptable replay window.

Timestamp validation supplements signature verification.

It does not replace event deduplication.

---

# 17. Replay Protection

A valid historical webhook may still be maliciously or accidentally replayed.

Therefore:

```text
valid signature
      ≠
new event
```

External event deduplication remains required.

---

# 18. External Event Identity

Payments SHOULD retain stable external event identifiers where supplied.

Conceptually:

```text
ExternalEventReference
│
├── source
├── engine_instance
├── environment
├── external_event_id
└── received_at
```

---

# 19. External Event Scope

An external event identifier SHALL be interpreted within its provider/engine/environment scope.

The same literal ID from two unrelated providers SHALL not collide.

---

# 20. Missing Event Identifier

Where a source does not provide a reliable event ID, Payments SHALL use defence-in-depth deduplication.

Possible inputs include:

```text
external transaction reference
event type
provider sequence number
payload fingerprint
provider timestamp
canonical target
current canonical state
```

No single heuristic SHALL be assumed universally sufficient.

---

# 21. Webhook Inbox

Payments SHOULD persist webhook ingress state before or as part of durable processing.

Conceptually:

```text
WebhookInbox
│
├── internal_ingress_id
├── source
├── external_event_id
├── received_at
├── verification_status
├── processing_status
├── canonical_target
├── payload_reference
└── processing_error
```

The exact schema is implementation-specific.

---

# 22. Webhook Processing States

A webhook ingress record MAY conceptually progress through:

```text
RECEIVED
   │
   ▼
VERIFIED
   │
   ▼
DEDUPLICATED
   │
   ▼
MAPPED
   │
   ▼
PROCESSED
```

or:

```text
REJECTED
QUARANTINED
RETRY_PENDING
UNRESOLVED
```

where appropriate.

These are webhook-processing states, not Payment states.

---

# 23. Durable Receipt

Payments SHOULD be able to acknowledge receipt after the webhook has been durably captured according to the ingress design.

It SHOULD NOT require all downstream consumers to finish before responding to the webhook source.

---

# 24. Fast Acknowledgement

Webhook handlers SHOULD avoid performing unnecessary long-running downstream work synchronously.

Preferred model:

```text
Receive
  │
  ▼
Verify
  │
  ▼
Durably Record
  │
  ▼
Acknowledge
  │
  ▼
Continue Reliable Processing
```

subject to source-specific requirements.

---

# 25. Verification Failure

An unverifiable webhook SHALL NOT mutate canonical payment state.

It SHOULD produce appropriate security telemetry.

---

# 26. Unknown Provider

A webhook from an unknown or inactive source SHALL fail closed.

Payments SHALL NOT infer provider identity from untrusted payload fields.

---

# 27. Tenant Resolution

Tenant SHALL NOT be selected solely from webhook-provided metadata.

Payments SHALL resolve canonical context through trusted external mappings.

---

# 28. Legal Entity Resolution

Legal Entity SHALL similarly be derived from trusted Baobab mappings.

An external callback cannot switch the canonical Legal Entity merely by claiming another merchant identifier.

---

# 29. Merchant / Profile Resolution

HyperSwitch Merchant and Business Profile references MAY participate in trusted mapping.

They remain external operational identifiers.

PAY-0003 and PAY-0004 govern the mapping boundary.

---

# 30. External Payment Resolution

The callback SHALL resolve to the corresponding canonical:

```text
Payment
PaymentAttempt
Refund
Dispute
Settlement
```

as applicable.

Unknown references SHALL not automatically create financial aggregates.

---

# 31. Orphan Webhook

If a valid webhook references an external transaction that cannot be mapped:

```text
VALID EXTERNAL EVENT
        │
        ▼
NO CANONICAL MAPPING
        │
        ▼
UNRESOLVED / QUARANTINED
```

rather than inventing a Payment.

---

# 32. Orphan Recovery

Operations MAY reconcile orphan callbacks against:

```text
PaymentAttempts
provider references
HyperSwitch references
provisioning mappings
reconciliation records
```

under controlled recovery.

---

# 33. Canonicalisation

Provider-specific event semantics SHALL be translated into Baobab canonical semantics.

Example:

```text
Provider:
payment_intent.succeeded

Provider:
charge.completed

Provider:
transaction.success
```

may, where semantically equivalent, result in:

```text
payment.captured
```

after validation.

---

# 34. Translation Layer

Provider terminology SHALL remain inside:

```text
HyperSwitch adapter
provider adapter
webhook translation layer
```

Canonical consumers SHALL not need provider-specific event knowledge.

---

# 35. Raw Events Are Not Canonical Contracts

The following is prohibited:

```text
Event Bus
   │
   ▼
raw_hyperswitch_webhook_payload
```

as the Baobab platform event contract.

Provider payloads MAY be retained operationally where appropriate, but canonical contracts remain provider-independent.

---

# 36. Canonical Events

Representative canonical Payments events include:

```text
payment.created
payment.processing
payment.requires_action
payment.authorized
payment.capture_requested
payment.captured
payment.failed
payment.cancelled

refund.requested
refund.processing
refund.succeeded
refund.failed

dispute.opened
dispute.updated
dispute.closed

settlement.received
```

Exact schemas and event names SHALL remain versioned contracts.

---

# 37. Event Means Fact

Canonical events SHOULD use fact-oriented semantics.

For example:

```text
payment.captured
```

means Baobab Payments has established the canonical fact that capture occurred.

It is not a request to capture.

---

# 38. Commands Versus Events

The distinction SHALL remain explicit:

```text
COMMAND
=
please perform an action
```

```text
EVENT
=
an authoritative fact has been established
```

Example:

```text
CapturePayment
```

is not:

```text
payment.captured
```

---

# 39. Requested Versus Observed

Baobab SHALL distinguish:

```text
capture requested
```

from:

```text
capture observed/confirmed
```

An API command being accepted does not itself prove external completion.

---

# 40. Canonical Event Envelope

Canonical payment events SHOULD use a common versioned envelope.

Conceptually:

```text
EventEnvelope
│
├── event_id
├── event_type
├── event_version
├── occurred_at
├── recorded_at
├── correlation_id
├── causation_id?
├── tenant_id
├── organisation_id?
├── legal_entity_id
├── market_id?
├── source_engine
├── aggregate_type
├── aggregate_id
├── aggregate_revision
├── simulated
└── data
```

The exact canonical envelope belongs in Shared contracts.

---

# 41. Event ID

Every canonical event SHALL have a globally unique or sufficiently unique canonical event identifier.

The same canonical event SHALL preserve the same event ID across transport retries.

---

# 42. Event Type

`event_type` SHALL describe the canonical Baobab fact.

It SHALL NOT encode provider-specific event names.

---

# 43. Event Version

Canonical event schemas SHALL be explicitly versioned.

Consumers SHALL NOT infer schema compatibility from event name alone.

---

# 44. Occurred At

`occurred_at` represents when the financial/business fact is understood to have occurred.

It MAY originate from verified external information where reliable.

---

# 45. Recorded At

`recorded_at` represents when Baobab recorded the canonical fact.

Therefore:

```text
occurred_at
      ≠ necessarily
recorded_at
```

---

# 46. Aggregate Revision

Events SHOULD carry aggregate revision/version where useful.

This assists consumers with:

```text
ordering
stale-event detection
projection consistency
```

---

# 47. Simulated

Sandbox/simulated events SHALL carry:

```text
simulated = true
```

as required by the existing payment architecture.

Production consumers SHALL not mistake simulated financial facts for real-money events.

---

# 48. Event Context

Canonical events SHALL contain enough canonical context for authorised consumers to process them correctly without deriving tenant or Legal Entity from provider identifiers.

---

# 49. Data Minimisation

Events SHALL contain only the information needed by consumers.

They SHALL NOT become a transport for:

```text
PAN
CVV
provider credentials
raw bank credentials
private keys
unnecessary personal data
```

PAY-0012 governs sensitive payment data.

---

# 50. Provider References

Canonical events MAY include opaque provider/external references where consumers legitimately need them.

Such references SHALL remain clearly external.

---

# 51. Event Production

Canonical events SHALL be produced from committed canonical state changes.

The event must correspond to an established Baobab fact.

---

# 52. Transactional Outbox

Payments SHALL use a transactional outbox or equivalent reliable publication mechanism.

Conceptually:

```text
BEGIN DATABASE TRANSACTION

Update Payment
Insert Canonical Event into Outbox

COMMIT
```

Then:

```text
Outbox Publisher
      │
      ▼
Event Transport
```

---

# 53. Why Outbox Is Mandatory

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
payment.captured never published
```

could permanently diverge consumers from canonical payment truth.

---

# 54. Atomic State + Event Intent

The canonical state mutation and intent to publish its event SHALL commit atomically within Payments' local database boundary.

---

# 55. No Distributed Transaction

Payments SHALL NOT require a distributed transaction spanning:

```text
Payments PostgreSQL
Event Broker
HyperSwitch
Provider
Trade
ERP
```

Reliability comes from:

```text
local transaction
+
outbox
+
idempotent publication
+
idempotent consumption
+
recovery
```

---

# 56. At-Least-Once Publication

Canonical event publication SHALL assume at-least-once semantics.

The outbox publisher may publish the same event more than once.

This is acceptable.

---

# 57. Stable Event Identity on Redelivery

Publishing the same outbox entry repeatedly SHALL preserve:

```text
event_id
```

The publisher SHALL NOT manufacture a new event ID merely because transport delivery was retried.

---

# 58. Consumer Idempotency

Every consumer of financially meaningful payment events SHALL process events idempotently.

Example:

```text
payment.captured
event_id = E1
```

delivered five times SHALL produce one semantic consumer-side effect.

---

# 59. Consumer Inbox

Critical consumers SHOULD use an inbox/deduplication mechanism.

Conceptually:

```text
Event E1
   │
   ▼
Consumer Inbox
   │
 ┌─┴────────────┐
 │              │
NEW          ALREADY SEEN
 │              │
 ▼              ▼
PROCESS        ACK
```

---

# 60. Consumer Transaction

Where possible:

```text
record event consumed
+
apply local projection
```

SHOULD occur in the same local transaction.

---

# 61. Trade Consumer

Trade MAY consume canonical payment events to update commerce projections.

For example:

```text
payment.captured
       │
       ▼
Trade payment projection
       │
       ▼
commerce policy evaluates
order / fulfilment consequence
```

Payments does not directly dictate order lifecycle.

---

# 62. Subscription Consumer

Subscriptions MAY consume:

```text
payment.captured
payment.failed
refund.succeeded
```

where relevant to billing/delinquency state.

Subscriptions remains authoritative for subscription lifecycle.

---

# 63. ERP Consumer

ERP MAY consume canonical financial facts for accounting integration.

ERP SHALL independently deduplicate them.

One `payment.captured` fact SHALL not create two accounting consequences because of transport redelivery.

---

# 64. Pulse Consumer

Pulse MAY consume appropriately governed payment events for analytics/intelligence.

Analytical consumption SHALL not become financial authority.

---

# 65. Control Plane Consumer

Control Plane MAY consume operational/payment events where required for capability health, governance or lifecycle coordination.

It SHALL not become the payment lifecycle authority.

---

# 66. Ordering

Global event ordering SHALL NOT be assumed.

The platform SHALL not depend on:

```text
all payment events everywhere
```

having one total order.

---

# 67. Aggregate Ordering

Where practical, ordering SHOULD be preserved or reconstructable within a canonical aggregate.

For example:

```text
Payment P
revision 7
revision 8
revision 9
```

---

# 68. Partitioning

If an event broker supports partition keys, canonical aggregate identity SHOULD normally be considered for partitioning when ordering benefits justify it.

Exact broker topology remains infrastructure-specific.

---

# 69. Reordering

Consumers SHALL tolerate reordered delivery.

Example:

```text
payment.captured revision 8
```

may arrive before:

```text
payment.processing revision 7
```

The consumer SHALL not regress its projection.

---

# 70. Canonical State Does Not Regress

Webhook/event ordering SHALL never override PAY-0008 lifecycle invariants.

A stale external callback cannot turn:

```text
CAPTURED
```

back into:

```text
PROCESSING
```

---

# 71. Duplicate External Event

A duplicate external callback SHALL not create a duplicate canonical event if no new canonical fact exists.

---

# 72. Duplicate Canonical Publication

A duplicate publication of the same canonical event MAY occur because of transport retry.

It retains the same event ID.

---

# 73. Distinct Facts

Two distinct canonical facts SHALL have distinct event IDs even if they relate to the same Payment.

Example:

```text
payment.authorized
event E1

payment.captured
event E2
```

---

# 74. State Change Without New Fact

A duplicate webhook that confirms already-known state need not create a new business event.

It MAY update operational metadata such as:

```text
last_seen_at
external observation count
```

without creating another canonical financial fact.

---

# 75. Delayed Webhook

A delayed webhook SHALL be evaluated against current canonical state.

It SHALL not be applied merely because its signature is valid.

---

# 76. Stale Webhook

If the webhook describes a superseded state, Payments SHOULD record it operationally if useful but SHALL not regress canonical state.

---

# 77. Future / Unexpected Transition

If an authenticated callback implies an invalid or impossible canonical transition:

```text
current state
      │
      X
unexpected transition
```

Payments SHALL not force the transition.

It SHOULD:

```text
quarantine
alert
reconcile
```

as appropriate.

---

# 78. Conflicting Evidence

If provider evidence conflicts with canonical state, Payments SHALL enter controlled reconciliation rather than silently overwrite financial history.

---

# 79. Webhook Not Received

Absence of a webhook SHALL NOT prove that the external operation failed.

Providers may delay or lose callbacks.

---

# 80. Status Recovery

Payments SHALL support recovery through authoritative status queries where available.

Conceptually:

```text
Expected webhook missing
        │
        ▼
Recovery Scheduler
        │
        ▼
HyperSwitch / Provider Query
        │
        ▼
Validated Observation
        │
        ▼
Canonical State Transition
        │
        ▼
Canonical Event
```

---

# 81. Query-Based Observation

A status-query response SHALL pass through the same canonical validation principles as a webhook.

Different transport does not imply different financial truth rules.

---

# 82. Webhook and Polling Convergence

If webhook and polling both observe the same external fact:

```text
Webhook ──┐
          ├──► same canonical state
Polling ──┘
```

Payments SHALL converge without duplicate financial facts.

---

# 83. Reconciliation as Evidence

Later reconciliation may establish a financial fact that was not observed reliably in real time.

Such recovery SHALL preserve:

```text
original occurrence time where known
recorded/reconciled time
evidence source
audit history
```

---

# 84. Canonical Event After Recovery

If recovery establishes a previously unknown capture, Payments SHALL emit the appropriate canonical event.

The event represents when Baobab established the fact, while metadata may preserve when the provider says it occurred.

---

# 85. Event Replay

Baobab SHOULD support controlled replay of canonical events where operationally necessary.

Replay SHALL mean:

```text
redeliver existing canonical event
```

not:

```text
recreate financial operation
```

---

# 86. Replay Identity

Replayed canonical events SHALL retain their original:

```text
event_id
event_type
event_version
aggregate identity
```

unless a deliberate migration process creates a new derived event.

---

# 87. Replay Safety

Consumers SHALL treat replayed events idempotently.

Replay SHALL not create:

```text
second refund
second capture
duplicate accounting entry
duplicate fulfilment
```

---

# 88. Replay Authorisation

Bulk or targeted event replay SHALL be a privileged operational action.

It SHOULD be auditable.

---

# 89. Replay Scope

Replay SHOULD support controlled selection such as:

```text
event ID
aggregate
tenant
Legal Entity
time range
event type
consumer
```

subject to authorisation.

---

# 90. Consumer-Specific Redelivery

Where infrastructure permits, redelivering an event to one failed consumer SHOULD not require republishing it as a new canonical fact to every consumer.

---

# 91. Dead-Letter Handling

Events that repeatedly fail processing SHOULD enter a dead-letter or quarantine mechanism.

Conceptually:

```text
Event
  │
  ▼
Consumer
  │
  X repeated failure
  │
  ▼
DLQ / Quarantine
```

---

# 92. Dead Letter Is Not Disposal

A dead-letter queue SHALL NOT be treated as a place where financial events can be forgotten.

Financially significant failures require:

```text
alert
investigation
repair
replay
closure
```

---

# 93. DLQ Metadata

Dead-letter records SHOULD preserve:

```text
event identity
consumer
failure reason
attempt count
first failure
latest failure
correlation information
```

without unnecessary sensitive data.

---

# 94. Poison Events

A malformed canonical event or incompatible consumer may create a poison-message loop.

Retry policy SHALL include bounded retries before quarantine.

---

# 95. Transient Failures

Transient infrastructure failures SHOULD use retry with appropriate:

```text
backoff
jitter
bounded attempt policy
```

where applicable.

---

# 96. Permanent Failures

Schema incompatibility or violated domain assumptions SHOULD not be retried indefinitely.

They require quarantine/operational intervention.

---

# 97. Backpressure

Event processing SHALL support backpressure.

A slow consumer SHALL not force payment execution itself to wait indefinitely after canonical state has been safely committed.

---

# 98. Consumer Isolation

Failure of:

```text
ERP consumer
```

SHALL not prevent:

```text
Trade consumer
```

from receiving the same canonical event where infrastructure topology allows independent consumption.

---

# 99. Event Broker Independence

Canonical event contracts SHALL not depend on one particular event broker.

The architecture MAY use:

```text
Kafka
NATS
cloud event infrastructure
other supported transport
```

as infrastructure evolves.

The event contract remains transport-independent.

---

# 100. Transport Metadata

Broker-specific metadata SHALL remain outside canonical business semantics where possible.

---

# 101. Schema Registry

The platform SHOULD maintain governed event schemas through Shared contracts and, where useful, schema-registry tooling.

Consumers SHALL know which event versions they support.

---

# 102. Schema Evolution

Event evolution SHALL prioritise backward-compatible changes.

Adding optional fields is generally preferable to silently changing existing field meaning.

---

# 103. Breaking Event Change

A breaking schema change SHALL require a new version.

For example:

```text
payment.captured v1
payment.captured v2
```

or equivalent contract-versioning mechanism.

---

# 104. Semantic Compatibility

Schema compatibility is not enough.

A field SHALL NOT retain the same name while changing its financial meaning incompatibly.

---

# 105. Deprecation

Old event versions SHOULD have:

```text
documented deprecation
consumer migration period
observability
retirement criteria
```

before removal.

---

# 106. Unknown Event Version

Consumers SHALL fail safely when receiving an unsupported event version.

They SHALL not guess the financial meaning of unknown fields.

---

# 107. Event Contract Ownership

Canonical payment event contracts belong to the Baobab shared-contract architecture.

`baobab-payments` owns production of payment-domain facts.

Shared defines reusable contract shape where platform-wide.

---

# 108. Provider Schema Isolation

HyperSwitch or provider schema changes SHALL be absorbed by the adapter/translation boundary where possible.

They SHALL not automatically force platform-wide event schema changes.

---

# 109. Correlation

Canonical events SHALL carry:

```text
correlation_id
```

where available.

This enables distributed tracing across:

```text
Trade
Payments
HyperSwitch
Provider
Events
ERP
Subscriptions
Pulse
```

---

# 110. Causation

Where useful, events SHALL carry:

```text
causation_id
```

to describe why the fact-processing path occurred.

---

# 111. Correlation Is Not Event Identity

The following distinction remains mandatory:

```text
event_id
=
this specific canonical event
```

```text
correlation_id
=
this wider distributed workflow
```

```text
causation_id
=
the preceding operation/event that caused this one
```

---

# 112. Source

Canonical events SHOULD identify the authoritative producing engine.

For payment events:

```text
source = baobab-payments
```

not the leaf digital estate.

---

# 113. Provider Provenance

Payments MAY preserve:

```text
provider
connector
external event reference
external transaction reference
```

as controlled provenance metadata.

Provider provenance does not alter canonical authority.

---

# 114. Audit

For canonical transitions originating from asynchronous evidence, audit SHOULD preserve:

```text
external source
external event reference
verification result
canonical target
previous state
new state
event ID
processing timestamp
correlation
```

as appropriate.

---

# 115. Raw Webhook Retention

Raw webhook payload retention SHALL be governed by:

```text
security
PCI scope
privacy
audit need
provider dispute need
retention policy
```

Payments SHALL NOT retain raw payloads indefinitely merely because storage is available.

---

# 116. Sensitive Raw Payloads

Where raw payloads may contain sensitive payment or personal data, storage SHALL be minimised and protected.

PAY-0012 governs detailed handling.

---

# 117. Encryption

Webhook/event persistence containing sensitive operational data SHALL use appropriate encryption at rest and in transit according to platform security standards.

---

# 118. Access Control

Access to:

```text
raw webhooks
provider payloads
event replay
DLQ contents
event administration
```

SHALL be privileged and audited where appropriate.

---

# 119. Event Mutation

Canonical events are immutable facts.

Published events SHALL not be edited in place.

Corrections SHALL occur through:

```text
new canonical state
new corrective event
reconciliation record
```

as appropriate.

---

# 120. Event Deletion

Financial canonical events SHALL not be casually deleted because they are inconvenient or later superseded.

Retention follows financial, legal, audit and privacy policy.

---

# 121. Observability

Payments SHALL expose observability across the complete pipeline:

```text
webhook received
webhook verified
webhook rejected
duplicate detected
mapping resolved
state transitioned
outbox written
event published
publication retried
event quarantined
recovery initiated
```

---

# 122. Metrics

Representative metrics include:

```text
webhooks_received_total
webhooks_verified_total
webhooks_rejected_total
webhook_signature_failures_total
webhook_duplicates_total
webhook_unresolved_total
webhook_processing_latency
webhook_processing_failures

outbox_pending
outbox_publish_latency
outbox_publish_failures
event_publish_retries

canonical_events_total
event_replay_total
dlq_depth

unknown_payment_attempts
recovery_latency
reconciliation_corrections
```

Exact telemetry naming is implementation-specific.

---

# 123. Alerting

High-priority alerts SHOULD cover:

```text
signature failure spike
provider webhook silence
webhook backlog
unresolved external references
outbox backlog
event publication failure
DLQ growth
state-transition conflicts
large ordering lag
recovery backlog
```

---

# 124. Provider Webhook Silence

Payments SHOULD detect unexpected absence of callbacks for providers/methods where webhook activity is expected.

Silence does not prove failure but may indicate:

```text
provider outage
configuration drift
network failure
secret rotation problem
endpoint failure
```

---

# 125. Health

Webhook ingress health and event publication health SHALL contribute to Payments operational readiness.

A healthy synchronous API alone does not mean the payment engine is operationally healthy.

---

# 126. Readiness

Payments readiness SHOULD distinguish:

```text
API readiness
provider connectivity
webhook ingress readiness
event publication readiness
database readiness
```

where useful.

---

# 127. Deployment

Deployments SHALL preserve unprocessed:

```text
webhook inbox records
outbox records
recovery work
```

across process restarts.

In-memory queues alone are insufficient for critical financial work.

---

# 128. Shutdown

Graceful shutdown SHOULD stop taking new work where appropriate while allowing safe completion or durable handoff of in-flight processing.

---

# 129. Crash Recovery

After restart, Payments SHALL resume:

```text
unprocessed verified webhooks
pending outbox publication
recoverable asynchronous operations
```

without creating duplicate financial facts.

---

# 130. Disaster Recovery

PAY-0020 SHALL define wider DR.

Recovered Payments infrastructure SHALL preserve or reconstruct:

```text
canonical state
webhook deduplication state
external mappings
outbox state
event identity
```

sufficiently to avoid unsafe replay.

---

# 131. Multi-Region Ingress

If Payments operates across regions, webhook routing SHALL preserve correct:

```text
engine instance
tenant isolation
Legal Entity mapping
regional authority
deduplication
```

A webhook arriving at the wrong region SHALL not be blindly processed.

---

# 132. Global Duplicate Webhook Safety

If the same provider webhook can reach multiple regional ingress points, the architecture SHALL prevent both regions from independently establishing duplicate canonical facts.

---

# 133. Regional Event Publication

Regional event infrastructure MAY differ physically.

Canonical event semantics SHALL remain consistent.

---

# 134. Data Residency

Raw provider payloads MAY have stricter residency requirements than canonical event projections.

The platform SHOULD minimise unnecessary cross-region propagation of raw payment data.

---

# 135. Event Consumers and Tenant Isolation

Consumers SHALL enforce tenant isolation.

A consumer subscribed to platform events does not automatically have authority to process every tenant's financial information.

---

# 136. Event Filtering Is Not Sole Authorisation

Broker topic/subscription filtering MAY assist isolation.

It SHALL not be the only authorisation control where consumer access is security-sensitive.

---

# 137. Webhook Endpoint Discovery

Provider webhook URLs SHALL be configured through controlled infrastructure/payment configuration.

Digital estates SHALL not dynamically replace canonical Payments webhook endpoints.

---

# 138. Secret Rotation

Webhook verification secrets or keys SHALL support controlled rotation without unnecessary event loss.

Where providers support overlapping keys, rotation SHOULD avoid a hard cutover window.

---

# 139. Rotation Failure

A secret rotation problem SHALL fail closed for unverifiable events while triggering urgent operational alerts and recovery procedures.

It SHALL not justify disabling verification.

---

# 140. Provider Retry

Providers often retry callbacks after non-successful responses.

Payments SHALL design webhook handlers so legitimate retry is safe.

---

# 141. Acknowledgement Semantics

Payments SHOULD return source-appropriate acknowledgement only after satisfying the minimum durable/verification conditions defined for that integration.

The exact HTTP status semantics remain adapter-specific.

---

# 142. Malformed Payload

Malformed input SHALL not enter canonical processing.

It SHOULD produce controlled telemetry without leaking internal details to the sender.

---

# 143. Oversized Payload

Ingress SHALL enforce reasonable request-size limits appropriate to the integration.

Webhook endpoints SHALL not accept arbitrary unbounded payloads.

---

# 144. Rate Limiting

Webhook ingress MAY apply source-aware rate controls and abuse protection.

Controls SHALL account for legitimate provider retry bursts.

---

# 145. Denial-of-Service Protection

Webhook endpoints are internet-facing security surfaces.

Infrastructure SHOULD provide appropriate:

```text
TLS
WAF / edge protection where applicable
rate controls
payload limits
monitoring
```

without breaking provider delivery.

---

# 146. IP Allowlisting

Provider IP allowlisting MAY be used as defence in depth where reliable.

It SHALL not replace cryptographic verification where signatures are supported.

---

# 147. Clock Dependence

Timestamp validation SHALL tolerate reasonable clock skew.

Platform time synchronisation is operationally important.

---

# 148. Canonical Event Example

Conceptually:

```text
{
  "event_id": "evt_baobab_...",
  "event_type": "payment.captured",
  "event_version": 1,
  "occurred_at": "...",
  "recorded_at": "...",
  "correlation_id": "...",
  "tenant_id": "...",
  "legal_entity_id": "...",
  "market_id": "...",
  "aggregate_type": "payment",
  "aggregate_id": "pay_...",
  "aggregate_revision": 8,
  "simulated": false,
  "data": {
    "payment_intent_id": "pi_...",
    "amount": "...",
    "currency": "ZAR"
  }
}
```

This example is conceptual and does not freeze the Shared contract schema.

---

# 149. Successful Asynchronous Flow

```text
Customer
   │
   ▼
Trade
   │
   ▼
Payments
   │
   ▼
HyperSwitch
   │
   ▼
Provider
   │
   ▼
PROCESSING

... later ...

Provider
   │
   ▼
HyperSwitch
   │
   ▼
Payments Webhook Ingress
   │
   ▼
Verify
   │
   ▼
Deduplicate
   │
   ▼
Resolve PaymentAttempt
   │
   ▼
Validate State Transition
   │
   ▼
CAPTURED
   │
   ▼
Outbox
   │
   ▼
payment.captured
   │
   ├────────► Trade
   ├────────► ERP
   └────────► other authorised consumers
```

---

# 150. Duplicate Webhook Flow

```text
Provider Event X
      │
      ├──────────────┐
      ▼              ▼
Delivery 1       Delivery 2
      │              │
      ▼              ▼
Webhook Inbox    Webhook Inbox
      │              │
      ▼              ▼
NEW             DUPLICATE
      │              │
      ▼              └────► no duplicate state mutation
Process
      │
      ▼
Canonical Fact
      │
      ▼
Event E1
```

---

# 151. Reordered Webhook Flow

```text
Provider sends:

PROCESSING
CAPTURED

Network delivers:

CAPTURED
PROCESSING

Payments receives:
     │
     ▼
CAPTURED
     │
     ▼
canonical state = CAPTURED
     │
     ▼
later PROCESSING
     │
     ▼
stale transition detected
     │
     X
NO REGRESSION
```

---

# 152. Lost Webhook Recovery

```text
Provider
   │
   ▼
CAPTURED
   │
   X
Webhook lost
   │
   ▼

Payments still PROCESSING / UNKNOWN
   │
   ▼
Recovery Scheduler
   │
   ▼
Query HyperSwitch / Provider
   │
   ▼
CAPTURED confirmed
   │
   ▼
Canonical state transition
   │
   ▼
Outbox
   │
   ▼
payment.captured
```

---

# 153. Outbox Flow

```text
Webhook / API / Recovery
          │
          ▼
     Domain Handler
          │
          ▼
BEGIN TRANSACTION
          │
          ├── update aggregate
          │
          └── insert outbox event
          │
          ▼
        COMMIT
          │
          ▼
    Outbox Publisher
          │
          ▼
      Event Broker
          │
     ┌────┼────┐
     ▼    ▼    ▼
   Trade ERP  Subscriptions
```

---

# 154. Consumer Flow

```text
Canonical Event
      │
      ▼
Consumer Inbox
      │
 ┌────┴────┐
 │         │
NEW    DUPLICATE
 │         │
 ▼         ▼
BEGIN     ACK
 │
 ├── record event_id
 ├── update projection
 │
 ▼
COMMIT
```

---

# 155. Replay Flow

```text
Existing Canonical Event E1
           │
           ▼
Privileged Replay Request
           │
           ▼
Authorisation / Audit
           │
           ▼
Redeliver E1
same event_id
           │
           ▼
Consumer Inbox
           │
      ┌────┴────┐
      │         │
   already     missing
   processed   previously
      │         │
      ▼         ▼
     ACK      PROCESS
```

Replay does not create a new payment action.

---

# 156. Event Delivery Semantics

Baobab SHALL adopt the following baseline:

| Layer | Delivery / Processing Semantics |
|---|---|
| Provider → Payments webhook | At least once / source-dependent |
| Webhook ingress | Durable + deduplicated |
| Canonical state transition | Transactional |
| State → outbox | Atomic local transaction |
| Outbox → broker | At least once |
| Broker → consumer | At least once |
| Consumer processing | Idempotent |
| Financial effect | Duplicate-safe |
| Event ordering | Aggregate-aware; no global ordering assumption |
| Replay | Same event identity |
| Unknown outcome | Recover; do not guess |

---

# 157. Production Readiness Gate

```text
WEBHOOK & EVENT READINESS

Webhook endpoints                      PASS
TLS                                    PASS
Signature verification                 PASS
Raw-body verification support          PASS
Replay protection                      PASS
External event deduplication           PASS
Webhook inbox                          PASS
Canonical mapping                      PASS
State-machine validation               PASS
Orphan-event handling                  PASS
Outbox                                 PASS
Stable canonical event IDs             PASS
At-least-once publication              PASS
Consumer idempotency contract          PASS
Ordering strategy                      PASS
Stale-event protection                 PASS
Schema versioning                      PASS
DLQ / quarantine                       PASS
Retry / backoff                        PASS
Controlled replay                      PASS
Recovery polling                       PASS
Audit                                  PASS
Observability                          PASS
Secret rotation                        PASS
Tenant isolation                       PASS
Security hardening                     PASS
--------------------------------------------
Webhook / Event Plane                  READY
```

---

# 158. Invariants

The following invariants SHALL hold:

1. Webhook receipt does not establish canonical financial truth.
2. Raw provider events are not canonical Baobab events.
3. External callbacks are untrusted until verified.
4. Failed verification cannot mutate financial state.
5. A valid signature does not prove an event is new.
6. External events are deduplicated.
7. Tenant is resolved from trusted context/mapping.
8. Legal Entity is resolved from trusted context/mapping.
9. Provider metadata cannot redefine canonical business context.
10. Unknown external references do not automatically create Payments.
11. Provider-specific event names do not become platform contracts.
12. Canonical events describe Baobab facts.
13. Commands and events remain distinct.
14. Requested action is not equivalent to completed action.
15. Canonical state transition precedes canonical event publication.
16. State mutation and outbox insertion are locally atomic.
17. Event transport may deliver more than once.
18. Redelivery preserves event identity.
19. Consumers process canonical events idempotently.
20. Duplicate webhook does not create duplicate canonical fact.
21. Duplicate canonical delivery does not create duplicate consumer effect.
22. Global ordering is not assumed.
23. Aggregate ordering/revision is used where appropriate.
24. Stale callbacks cannot regress canonical state.
25. Valid but impossible transitions trigger reconciliation rather than forced mutation.
26. Missing webhook does not imply payment failure.
27. Polling and webhook observations converge on one canonical state.
28. Replay redelivers an existing fact; it does not recreate a financial operation.
29. Event replay preserves event identity.
30. Event replay is privileged and auditable.
31. DLQ is not permanent disposal of financial facts.
32. Canonical events contain no unnecessary payment secrets.
33. Raw webhook retention is minimised and governed.
34. Event schema changes are versioned.
35. Provider schema changes remain behind translation boundaries where possible.
36. Simulated events remain explicitly identified.
37. Event IDs are not correlation IDs.
38. Correlation IDs are not authorisation credentials.
39. Broker transport metadata is not canonical business semantics.
40. Event-broker choice does not define the canonical event contract.
41. A failed consumer does not invalidate canonical payment truth.
42. Consumer failure does not require the payment operation to be repeated.
43. Outbox backlog is recoverable durable work.
44. Webhook processing backlog is recoverable durable work.
45. Process restart does not erase pending financial-event work.
46. Provider retry is expected and safe.
47. Secret rotation does not justify disabling verification.
48. Cross-region delivery cannot create duplicate canonical financial facts.
49. Accounting consumers independently deduplicate.
50. Commerce consumers independently deduplicate.
51. Subscription consumers independently deduplicate.
52. Financial event history remains auditable.
53. Canonical payment truth resides in Payments, not in the event broker.
54. Event delivery is a propagation mechanism, not the source of payment authority.

---

# 159. Consequences

## Positive

This decision provides:

- secure webhook ingress;
- provider-independent canonical events;
- duplicate-safe callback processing;
- durable asynchronous payment handling;
- reliable canonical event publication;
- safe at-least-once delivery;
- consumer isolation;
- event replay;
- failure recovery;
- schema evolution;
- provider abstraction;
- strong tenant and Legal Entity boundaries;
- auditability;
- operational observability.

It also prevents leaf engines from becoming coupled directly to HyperSwitch or individual PSP callback formats.

## Negative

The architecture requires:

- webhook ingress persistence;
- provider-specific verification adapters;
- event deduplication;
- transactional outbox;
- consumer inbox patterns;
- schema governance;
- dead-letter handling;
- replay tooling;
- recovery schedulers;
- additional monitoring and operational procedures.

These costs are accepted because asynchronous financial processing cannot safely depend on best-effort HTTP callbacks.

---

# 160. Alternatives Considered

## Publish raw HyperSwitch webhooks directly

Rejected.

This would expose implementation-specific contracts and bypass canonical validation.

---

## Let Trade consume payment-provider webhooks

Rejected.

Trade is not payment execution authority.

---

## Let ERP consume PSP callbacks directly

Rejected.

Provider observations are not accounting authority.

---

## Assume webhook delivery exactly once

Rejected.

External systems commonly retry.

---

## Require global event ordering

Rejected.

It introduces unnecessary coupling and scalability constraints.

---

## Publish events directly after database commit

Rejected as the sole mechanism.

A process crash between commit and publish can permanently lose the event.

---

## Use only polling

Rejected.

Polling alone adds latency and unnecessary load.

It remains valuable as recovery.

---

## Use only webhooks

Rejected.

Webhooks may be lost or delayed.

Recovery/query/reconciliation remain necessary.

---

## Generate a new event ID on retry

Rejected.

Consumers could mistake redelivery for a new canonical fact.

---

## Retry poison events indefinitely

Rejected.

Persistent failures require quarantine and intervention.

---

# 161. Relationship to PAY-0010

PAY-0010 established:

```text
MESSAGES MAY REPEAT
       │
       ▼
PROCESSING MUST BE IDEMPOTENT
```

PAY-0011 applies that principle across the event plane:

```text
Provider Webhook
      │
      ▼
deduplicate
      │
      ▼
Canonical State
      │
      ▼
Outbox
      │
      ▼
at-least-once publication
      │
      ▼
Consumer Inbox
      │
      ▼
idempotent consequence
```

Together they provide duplicate-safe asynchronous financial processing.

---

# 162. Relationship to PAY-0008

PAY-0008 remains authoritative for valid lifecycle transitions.

PAY-0011 does not permit an external callback to bypass the canonical state machine.

Therefore:

```text
EXTERNAL OBSERVATION
       │
       ▼
PAY-0011 validation
       │
       ▼
PAY-0008 state transition
       │
       ▼
CANONICAL FACT
```

---

# 163. Relationship to PAY-0009

Routing and failover may depend on asynchronous evidence.

A delayed provider callback can resolve an `UNKNOWN` PaymentAttempt.

Therefore:

```text
UNKNOWN
   │
   ▼
Webhook / Recovery
   │
   ▼
Outcome established
   │
   ├── SUCCESS ──► no failover
   │
   └── FAILURE ──► PAY-0009 may evaluate next route
```

Webhook delay SHALL never itself justify failover.

---

# 164. Relationship to PAY-0012

PAY-0011 establishes that:

```text
webhooks
events
logs
DLQs
replay stores
```

are payment-data surfaces.

PAY-0012 SHALL define which payment information may safely exist on each surface and how:

```text
PCI data
tokens
payment credentials
personal data
secrets
```

are isolated and protected.

---

# 165. Relationship to PAY-0015

Real-time event processing and settlement reconciliation are complementary.

Conceptually:

```text
REAL-TIME OBSERVATION
Webhooks / Queries
        │
        ▼
Canonical Payment Facts

LATER FINANCIAL EVIDENCE
Settlement / Reconciliation
        │
        ▼
Verify / Detect Divergence
```

Reconciliation provides independent evidence when asynchronous delivery was incomplete or incorrect.

---

# 166. Complete Event Architecture

```text
                         EXTERNAL PAYMENT WORLD
                                  │
                    ┌─────────────┴─────────────┐
                    │                           │
               HyperSwitch                 Provider
                    │                           │
                    └─────────────┬─────────────┘
                                  │
                                  ▼
                         WEBHOOK INGRESS
                                  │
                                  ▼
                        AUTHENTICITY CHECK
                                  │
                                  ▼
                           DURABLE INBOX
                                  │
                                  ▼
                           DEDUPLICATION
                                  │
                                  ▼
                         CONTEXT RESOLUTION
                                  │
                                  ▼
                      PROVIDER → CANONICAL MAP
                                  │
                                  ▼
                         STATE VALIDATION
                                  │
                                  ▼
                    ┌─────────────────────────┐
                    │   LOCAL TRANSACTION     │
                    │                         │
                    │  Canonical State        │
                    │       +                 │
                    │  Outbox Event           │
                    └────────────┬────────────┘
                                 │
                                 ▼
                         OUTBOX PUBLISHER
                                 │
                                 ▼
                         EVENT TRANSPORT
                                 │
             ┌───────────────────┼───────────────────┐
             │                   │                   │
             ▼                   ▼                   ▼
           TRADE           SUBSCRIPTIONS            ERP
             │                   │                   │
             ▼                   ▼                   ▼
           Inbox               Inbox               Inbox
             │                   │                   │
             ▼                   ▼                   ▼
        Commerce View       Billing View      Accounting View
```

Recovery runs alongside this architecture:

```text
Missing / Ambiguous Observation
              │
              ▼
       Recovery Scheduler
              │
              ▼
     HyperSwitch / Provider
              │
              ▼
      Validated Observation
              │
              └────► Canonical State Pipeline
```

---

# 167. Final Decision

Baobab Payments SHALL operate on the following permanent event model:

```text
EXTERNAL CALLBACK
      ≠
CANONICAL EVENT

WEBHOOK RECEIVED
      ≠
PAYMENT SUCCEEDED

VALID SIGNATURE
      ≠
NEW EVENT

PROVIDER STATE
      ≠
CANONICAL STATE UNTIL VALIDATED

DATABASE COMMIT
      +
OUTBOX
      =
DURABLE EVENT INTENT

AT-LEAST-ONCE DELIVERY
      +
STABLE EVENT IDENTITY
      +
IDEMPOTENT CONSUMER
      =
DUPLICATE-SAFE PROPAGATION

MISSING WEBHOOK
      ≠
PAYMENT FAILURE

EVENT REPLAY
      ≠
FINANCIAL RE-EXECUTION
```

The authoritative flow is therefore:

```text
OBSERVE
   │
   ▼
VERIFY
   │
   ▼
DEDUPLICATE
   │
   ▼
RESOLVE
   │
   ▼
VALIDATE
   │
   ▼
ESTABLISH CANONICAL FACT
   │
   ▼
COMMIT FACT + OUTBOX
   │
   ▼
PUBLISH
   │
   ▼
CONSUME IDEMPOTENTLY
   │
   ▼
RECONCILE
```

**Providers report observations.  
Baobab Payments establishes canonical payment facts.  
The event plane propagates those facts; it does not create them.  
Delivery may repeat, arrive late, arrive out of order, or temporarily disappear.  
Canonical financial truth must remain correct despite all four conditions.**