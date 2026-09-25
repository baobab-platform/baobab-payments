# ADR-PAY-0019 — Observability, SLOs & Incident Response

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Reliability / Observability / Operations |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0018; ADR-PAY-0022; applicable Shared, Control Plane, IAM, ERP and Infrastructure ADRs |
| **Related** | ADR-PAY-0020, ADR-PAY-0021 |
| **Primary Domains** | Metrics, traces, logs, SLOs, alerting, financial correctness, incident response, provider health |

---

# 1. Context

`baobab-payments` is a financial execution engine.

Its operational health cannot be measured solely by:

```text
container running
HTTP 200
CPU normal
database reachable
```

A technically available system may still be financially unsafe.

Examples include:

```text
duplicate capture
duplicate payout
lost provider webhook
stuck UNKNOWN payment
incorrect provider routing
cross-Legal-Entity execution
unreconciled settlement
refund accepted but never executed
outbox backlog
stale provider activation
broken idempotency
```

Baobab therefore requires observability designed around both **technical reliability and financial correctness**.

---

# 2. Problem

Traditional service monitoring commonly asks:

> Is the service up?

Payments must additionally ask:

> Is the service executing the correct financial operation exactly within the authorised envelope, preserving canonical truth, and converging with external financial reality?

These are different questions.

The following equivalences are rejected:

```text
HTTP Success
    ≠
Financial Success

Service Availability
    ≠
Provider Availability

Provider Availability
    ≠
Route Eligibility

Payment Accepted
    ≠
Payment Completed

Payment Captured
    ≠
Payment Settled

Settlement Received
    ≠
ERP Reconciled

Webhook Received
    ≠
Canonical State Updated

Low Latency
    ≠
Correctness

No Exception
    ≠
No Financial Anomaly
```

---

# 3. Decision

Baobab Payments SHALL implement an observability model covering five dimensions:

```text
1. Service Health
2. Financial Execution Health
3. Provider / Route Health
4. Event & State-Convergence Health
5. Settlement / Reconciliation Health
```

These dimensions SHALL feed:

```text
metrics
logs
distributed traces
dashboards
SLOs
alerts
incident response
audit/reconciliation
```

---

# 4. Governing Principle

> **A payment platform is healthy only when it is available, authorised, duplicate-safe, financially correct, externally convergent, observable and reconcilable.**

Availability without correctness is not success.

---

# 5. Observability Architecture

```text
Digital Estate / Engine
          │
          ▼
   Baobab Payments API
          │
          ├──────── Metrics
          │
          ├──────── Logs
          │
          ├──────── Traces
          │
          └──────── Audit
          │
          ▼
    Canonical State
          │
          ▼
 Payment Attempt
          │
          ▼
      HyperSwitch
          │
          ▼
 Provider / Rail
          │
          ▼
 External Financial State
          │
          ▼
 Settlement / Reconciliation
          │
          ▼
          ERP
```

Observability SHALL permit correlation across this path.

---

# 6. OpenTelemetry

Baobab Payments SHOULD use OpenTelemetry-compatible instrumentation for:

```text
traces
metrics
context propagation
```

without making the canonical Payments architecture dependent upon one observability backend.

---

# 7. Vendor Independence

The architecture SHALL permit telemetry export to appropriate systems such as:

```text
Prometheus-compatible metrics systems
OpenTelemetry collectors
distributed tracing backends
log aggregation platforms
cloud-native monitoring systems
```

without embedding vendor-specific semantics into canonical payment contracts.

---

# 8. Observability Is Not Canonical Authority

Metrics, traces and logs are diagnostic projections.

They SHALL NOT become authoritative payment state.

Canonical Payments persistence remains authoritative for operational payment truth.

---

# 9. Correlation Model

Every financial workflow SHOULD be traceable using appropriate identifiers.

Conceptually:

```text
correlation_id
tenant_id
organisation_id
legal_entity_id
market_id
digital_estate_id?
source_engine
source_reference
payment_intent_id
payment_id
payment_attempt_id
refund_id?
dispute_id?
settlement_id?
payout_id?
provider
provider_reference?
hyperswitch_reference?
```

Not every signal requires every identifier.

---

# 10. Correlation ID

A `correlation_id` SHOULD propagate across service boundaries.

Example:

```text
ZuriBeans
   │
   ▼
Trade
   │ correlation_id
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
Webhook
   │
   ▼
Payments
   │
   ▼
ERP
```

---

# 11. Correlation Is Not Identity

Per PAY-0010:

```text
correlation_id
    ≠
payment_id

correlation_id
    ≠
idempotency_key

correlation_id
    ≠
provider_reference
```

---

# 12. Trace Context

Distributed trace context SHOULD propagate through synchronous internal service calls where technically supported.

---

# 13. Async Trace Continuity

For asynchronous processing:

```text
webhooks
event broker
outbox
reconciliation
recovery worker
scheduler
```

trace/correlation relationships SHOULD be preserved through:

```text
correlation
causation
event identity
aggregate identity
```

rather than assuming one continuous synchronous trace.

---

# 14. Financial Trace

A financial operation SHOULD permit reconstruction of:

```text
Business Obligation
        │
        ▼
PaymentIntent
        │
        ▼
Payment
        │
        ▼
PaymentAttempt
        │
        ▼
Provider Request
        │
        ▼
Provider Result
        │
        ▼
Canonical State
        │
        ▼
Canonical Event
        │
        ▼
Settlement
        │
        ▼
ERP Projection
```

---

# 15. Structured Logging

Payments SHALL use structured logs.

Logs SHOULD include stable machine-readable fields rather than relying solely on free-form messages.

---

# 16. Representative Log Context

Where relevant:

```text
timestamp
service
environment
region
engine_instance
tenant_id
legal_entity_id
market_id
operation
resource_type
resource_id
correlation_id
provider
outcome
error_category
```

---

# 17. Sensitive Data

PAY-0012 remains binding.

Logs SHALL NOT contain:

```text
PAN
CVV/CVC/CID
track data
PIN data
raw payment credentials
provider API secrets
private keys
raw bank credentials
unnecessary personal data
```

---

# 18. Provider Payload Logging

Complete provider request/response payloads SHALL NOT be routinely logged in production.

Diagnostic fields SHALL be extracted and sanitised.

---

# 19. Error Logging

Provider errors SHOULD be normalised while retaining sufficient safe diagnostic provenance.

---

# 20. Cardinality Control

Metrics SHALL avoid uncontrolled high-cardinality labels.

Identifiers such as:

```text
payment_id
customer_id
correlation_id
provider_reference
```

SHOULD generally not be metric labels.

They belong in traces/logs where appropriate.

---

# 21. Safe Metric Dimensions

Useful bounded metric dimensions MAY include:

```text
environment
region
provider
connector
market
currency
payment_method
transaction_type
canonical_status
error_category
```

subject to cardinality and privacy review.

---

# 22. Metrics Categories

Payments SHOULD expose at least:

```text
service metrics
API metrics
payment lifecycle metrics
provider metrics
routing metrics
idempotency metrics
webhook metrics
event/outbox metrics
refund metrics
dispute metrics
settlement metrics
reconciliation metrics
payout metrics
FX metrics
security metrics
recovery metrics
```

---

# 23. Service Metrics

Representative service metrics:

```text
http_requests_total
http_request_duration
http_errors_total
active_requests
process_cpu
process_memory
database_connection_usage
database_query_duration
worker_queue_depth
```

Exact metric names remain implementation-specific.

---

# 24. Payment Metrics

Representative financial execution metrics:

```text
payment_intents_created_total
payments_authorized_total
payments_captured_total
payments_failed_total
payments_unknown_total
payments_requires_action_total
payment_attempts_total
payment_attempt_duration
```

---

# 25. Amount Metrics

Financial volume metrics MAY expose aggregate amounts by bounded dimensions such as:

```text
currency
market
provider
transaction type
```

but SHALL avoid exposing sensitive transaction-level values as metric labels.

---

# 26. Idempotency Metrics

PAY-0010 SHOULD be observable through metrics such as:

```text
idempotency_claim_total
idempotency_replay_total
idempotency_conflict_total
duplicate_execution_prevented_total
idempotency_in_progress_total
```

---

# 27. Duplicate-Safety Metric

A detected prevented duplicate is operationally significant.

A suspected executed duplicate is financially critical.

These SHALL NOT be treated as equivalent severity.

---

# 28. UNKNOWN Metrics

PAY-0008 `UNKNOWN` state SHALL be explicitly observable.

Examples:

```text
payment_unknown_total
payment_unknown_current
payment_unknown_age
refund_unknown_current
payout_unknown_current
```

---

# 29. UNKNOWN Age

Age of unresolved financial uncertainty is more meaningful than count alone.

---

# 30. UNKNOWN SLO

Baobab SHOULD establish a bounded operational objective for resolving ambiguous external financial outcomes.

The exact target SHALL be established from provider capabilities and production evidence rather than invented prematurely.

---

# 31. Provider Metrics

Representative metrics:

```text
provider_requests_total
provider_success_total
provider_failure_total
provider_timeout_total
provider_latency
provider_rate_limited_total
provider_unknown_outcome_total
```

---

# 32. Provider Health

Provider health SHALL be evaluated independently from Payments service health.

---

# 33. Connector Health

Connector health SHOULD distinguish:

```text
configured
reachable
authenticated
operational
degraded
unavailable
```

without implying route eligibility.

---

# 34. Health Is Not Eligibility

Per PAY-0009:

```text
Provider Healthy
      ≠
Provider Eligible
```

and:

```text
Provider Unhealthy
```

may temporarily remove an otherwise eligible route.

---

# 35. Routing Metrics

Representative routing metrics MAY include:

```text
routing_decisions_total
routing_no_eligible_route_total
routing_failover_total
routing_failover_blocked_total
routing_route_excluded_total
routing_circuit_open_total
```

---

# 36. Routing Explainability

A routing incident SHOULD allow operators to determine:

```text
which routes were eligible
which were excluded
why they were excluded
which route was selected
whether failover occurred
why failover was allowed or blocked
```

---

# 37. Circuit Breakers

Provider circuit-breaker state SHALL be observable.

---

# 38. Routing Drift

PAY-0009/PAY-0022 drift between Baobab eligibility and HyperSwitch configuration SHALL be observable.

---

# 39. Webhook Metrics

PAY-0011 SHOULD expose:

```text
webhooks_received_total
webhooks_verified_total
webhooks_signature_failed_total
webhooks_duplicate_total
webhooks_unresolved_total
webhooks_processing_failed_total
webhook_processing_duration
```

---

# 40. Webhook Silence

Unexpected absence of provider webhooks MAY itself be an operational signal.

---

# 41. Webhook Silence Is Not Payment Failure

No payment SHALL be marked failed solely because a webhook did not arrive.

---

# 42. Inbox Metrics

Where durable webhook inbox processing exists:

```text
webhook_inbox_pending
webhook_inbox_failed
webhook_inbox_oldest_age
```

SHOULD be observable.

---

# 43. Outbox Metrics

Transactional outbox health SHALL be observable.

Representative signals:

```text
outbox_pending
outbox_publish_failed_total
outbox_oldest_unpublished_age
outbox_publish_duration
```

---

# 44. Event Metrics

Representative canonical-event metrics:

```text
events_published_total
event_publish_failed_total
event_redelivery_total
event_consumer_failure_total
event_dlq_total
```

---

# 45. Event Lag

Critical consumer lag SHOULD be observable.

---

# 46. Consumer Isolation

A failing analytics consumer SHALL NOT be treated identically to a failing ERP financial projection consumer.

Operational severity SHOULD reflect business consequence.

---

# 47. Refund Metrics

PAY-0013 SHOULD expose:

```text
refund_requested_total
refund_succeeded_total
refund_failed_total
refund_unknown_total
refund_amount
refund_processing_age
```

---

# 48. Void Metrics

Representative metrics:

```text
void_requested_total
void_succeeded_total
void_failed_total
void_unknown_total
```

---

# 49. Dispute Metrics

PAY-0014 SHOULD expose:

```text
disputes_opened_total
disputes_won_total
disputes_lost_total
disputes_response_due
dispute_evidence_submission_failed_total
```

where provider semantics permit.

---

# 50. Dispute Deadline Alerting

Approaching evidence deadlines SHOULD be observable and alertable.

---

# 51. Settlement Metrics

PAY-0015 SHOULD expose:

```text
settlements_received_total
settlement_amount
settlement_ingestion_failed_total
settlement_unmatched_total
settlement_variance_total
```

---

# 52. Reconciliation Metrics

Representative metrics:

```text
reconciliation_runs_total
reconciliation_matched_total
reconciliation_unmatched_total
reconciliation_exception_total
reconciliation_exception_age
```

---

# 53. Reconciliation Backlog

A growing reconciliation backlog is a financial-health problem even if transaction APIs remain healthy.

---

# 54. Payout Metrics

PAY-0016 SHOULD expose:

```text
payout_requested_total
payout_approved_total
payout_processing_total
payout_succeeded_total
payout_failed_total
payout_unknown_total
payout_amount
payout_processing_age
```

---

# 55. Payout Severity

Uncertain or duplicate Payout execution SHOULD receive particularly high operational severity because funds are moving outbound.

---

# 56. Beneficiary Metrics

Security-sensitive beneficiary operations MAY expose:

```text
beneficiary_destination_changes_total
beneficiary_destination_change_failures_total
```

without leaking beneficiary banking details.

---

# 57. FX Metrics

PAY-0017 SHOULD expose relevant:

```text
fx_quote_total
fx_quote_expired_total
fx_conversion_total
fx_conversion_failed_total
fx_conversion_unknown_total
fx_rate_variance
fx_reconciliation_exception_total
```

---

# 58. Security Metrics

PAY-0018 SHOULD expose signals such as:

```text
authentication_failure_total
authorization_denied_total
cross_tenant_access_denied_total
cross_legal_entity_access_denied_total
approval_denied_total
break_glass_activation_total
privileged_mutation_total
```

---

# 59. Security Metrics Are Not Audit

Security metrics assist detection.

Immutable audit remains separately required.

---

# 60. Golden Signals

Traditional service golden signals remain useful:

```text
latency
traffic
errors
saturation
```

but SHALL be supplemented with financial correctness signals.

---

# 61. Financial Golden Signals

Baobab Payments SHALL additionally monitor:

```text
Duplicate Safety
Unknown Financial Outcomes
Canonical / Provider Divergence
Event Convergence
Settlement Reconciliation
Cross-Boundary Violations
```

---

# 62. Financial Correctness Indicators

Representative indicators include:

```text
suspected_duplicate_financial_effect
unresolved_unknown
orphan_provider_transaction
canonical_provider_state_mismatch
over_refund_attempt
unexpected_provider_execution
unmatched_settlement
cross_tenant_execution_attempt
late_capture_after_void
```

---

# 63. Financial Correctness Takes Priority

A severe correctness anomaly MAY justify:

```text
provider suspension
route suspension
Payout suspension
capture suspension
```

even when availability is otherwise normal.

---

# 64. Service Level Indicators

SLIs SHALL measure user/business-relevant behaviour.

Potential SLIs include:

```text
API availability
API latency
payment initiation success
canonical-state convergence
provider request success
webhook processing latency
outbox publication latency
UNKNOWN resolution time
reconciliation completion
```

---

# 65. SLO Categories

Payments SHOULD maintain separate SLO categories for:

```text
Platform Availability
Platform Latency
Financial State Convergence
Provider Dependency Health
Event Delivery
Reconciliation Freshness
```

---

# 66. No Single Composite “Uptime”

One global uptime number SHALL NOT hide failure in financially significant subsystems.

---

# 67. Platform Availability SLO

Platform availability SHOULD measure whether Payments can correctly accept/process valid requests within its own authority boundary.

Provider outages SHOULD be distinguishable from internal platform outages.

---

# 68. Dependency-Adjusted Visibility

Dashboards SHOULD permit operators to distinguish:

```text
Baobab failure
HyperSwitch failure
provider failure
bank/rail failure
event infrastructure failure
ERP projection failure
```

---

# 69. Latency SLO

Latency SHALL be measured by operation type.

For example:

```text
PaymentIntent creation
authorization request
capture request
Refund request
Payout request
status query
```

SHALL not necessarily share one latency target.

---

# 70. Async Operations

For asynchronous payment methods, provider acknowledgement latency SHALL NOT be confused with financial completion time.

---

# 71. Completion SLO

Where meaningful, completion SLOs SHOULD be defined separately from request-response SLOs.

---

# 72. State-Convergence SLO

Baobab SHOULD measure how quickly authoritative external facts become reflected in canonical state.

---

# 73. Event-Convergence SLO

Baobab SHOULD measure how quickly committed canonical state is projected to critical downstream consumers.

---

# 74. Reconciliation Freshness SLO

Settlement/reconciliation freshness SHOULD have explicit objectives based on provider settlement cycles.

---

# 75. Provider SLO

Baobab SHALL NOT promise internal control over provider availability that it cannot guarantee.

Provider dependency objectives SHOULD therefore be reported separately.

---

# 76. Error Budget

SLOs MAY use error budgets.

Error-budget policy SHALL NOT permit consumption of financial-correctness safety margins.

---

# 77. Correctness Has No “Duplicate Charge Budget”

Baobab SHALL NOT establish an acceptable routine budget for:

```text
duplicate captures
cross-tenant money movement
unauthorised payouts
```

These are correctness incidents, not normal availability errors.

---

# 78. Target Calibration

Exact numerical SLO targets SHALL be established from:

```text
business requirements
provider contracts
production evidence
regulatory obligations
customer commitments
operational maturity
```

rather than arbitrary ADR values.

---

# 79. Initial SLO Framework

Before production evidence exists, Baobab SHALL at minimum define:

```text
SLI
measurement method
scope
dependencies
owner
alert threshold
review cadence
```

for each critical objective.

---

# 80. SLO Versioning

Material SLO changes SHOULD be versioned/documented.

---

# 81. Tenant-Level Visibility

Operational dashboards SHOULD permit appropriate Tenant-level analysis.

---

# 82. Legal Entity Visibility

Financial operations SHOULD permit Legal Entity-level visibility.

This is particularly important for independent entities such as:

```text
ZuriBeans
Thamani Global
Equator & Estate Co.
```

---

# 83. Group Views

Group-level dashboards MAY aggregate subsidiary data where authorised.

Aggregation SHALL NOT weaken underlying Legal Entity isolation.

---

# 84. External Tenant Isolation

External customer telemetry access SHALL remain isolated from other tenants.

---

# 85. Platform Operator Views

Baobab operators MAY require cross-tenant infrastructure views.

Such views SHOULD minimise business-sensitive data and comply with PAY-0018 authorization.

---

# 86. Dashboards

At minimum, production operations SHOULD provide dashboards for:

```text
Payments Service Health
Payment Execution
Provider Health
Routing
Webhooks
Events / Outbox
UNKNOWN Recovery
Refunds
Disputes
Settlement / Reconciliation
Payouts
FX
Security
```

---

# 87. Executive Dashboard

An executive/business view MAY show:

```text
payment volume
capture success
refund volume
payout volume
provider availability
settlement status
reconciliation exceptions
```

without exposing technical secrets.

---

# 88. Operations Dashboard

Operations SHOULD have deeper visibility into:

```text
provider failures
route decisions
UNKNOWN transactions
webhook backlog
outbox backlog
reconciliation exceptions
worker queues
```

---

# 89. Security Dashboard

Security operations SHOULD see:

```text
authorization failures
cross-boundary attempts
break-glass use
credential anomalies
privileged mutations
```

---

# 90. Alert Design

Alerts SHALL be:

```text
actionable
owned
severity-classified
deduplicated where possible
linked to runbooks
```

---

# 91. Avoid Alert Fatigue

Metrics SHOULD not automatically become alerts.

Alert only when human or automated intervention is useful.

---

# 92. Severity Model

Payments SHOULD use a consistent incident severity model.

Conceptually:

| Severity | Meaning |
|---|---|
| **SEV-1** | Active or credible major financial/security correctness threat |
| **SEV-2** | Material degradation affecting financial execution or reconciliation |
| **SEV-3** | Limited degradation with workaround or low financial exposure |
| **SEV-4** | Minor operational issue / informational defect |

Exact organisational naming MAY evolve.

---

# 93. SEV-1 Examples

Potential SEV-1 conditions include:

```text
confirmed duplicate charges
confirmed duplicate payouts
cross-tenant financial execution
cross-Legal-Entity financial execution
unauthorised money movement
widespread incorrect routing
provider compromise affecting execution
material canonical financial-state corruption
```

---

# 94. SEV-2 Examples

Potential SEV-2 conditions include:

```text
major provider outage
significant UNKNOWN backlog
widespread payment execution failure
critical webhook outage
critical outbox failure
large reconciliation backlog
Payout execution unavailable
```

depending on scope and financial exposure.

---

# 95. SEV-3 Examples

Potential SEV-3 conditions include:

```text
single provider degraded with safe alternative
limited settlement delay
non-critical consumer lag
isolated reconciliation exceptions
```

where correctness remains protected.

---

# 96. Financial Exposure

Incident severity SHOULD consider:

```text
number of transactions
financial amount
currency
Legal Entities affected
Markets affected
customers affected
duration
data exposure
duplicate-risk
recoverability
```

---

# 97. Severity Is Not Just Request Count

One unauthorised high-value Payout may be more severe than thousands of harmless retry errors.

---

# 98. Incident Lifecycle

Payments incidents SHOULD follow:

```text
DETECT
  │
  ▼
TRIAGE
  │
  ▼
CONTAIN
  │
  ▼
ESTABLISH FINANCIAL EXPOSURE
  │
  ▼
RECOVER
  │
  ▼
RECONCILE
  │
  ▼
VERIFY
  │
  ▼
POST-INCIDENT REVIEW
```

---

# 99. Detection

Incidents MAY be detected through:

```text
metrics
alerts
logs
traces
security signals
provider notification
reconciliation
customer report
ERP discrepancy
manual operations review
```

---

# 100. Triage

Initial triage SHALL distinguish:

```text
availability incident
provider incident
financial correctness incident
security incident
data incident
reconciliation incident
```

Multiple classifications may apply.

---

# 101. Containment Before Optimisation

Where financial correctness is uncertain, containment takes precedence over restoring transaction throughput.

---

# 102. Containment Options

Depending on scope:

```text
disable one provider
open circuit breaker
suspend route
suspend one Legal Entity
suspend one Market
suspend payment method
suspend capture
suspend Refund
suspend Payout
revoke credential
```

SHOULD be preferable to unnecessary platform-wide shutdown.

---

# 103. Smallest Safe Blast Radius

Containment SHOULD use the narrowest scope that reliably stops the unsafe behaviour.

---

# 104. No Unsafe Failover

Incident response SHALL NOT bypass PAY-0009 simply to restore availability.

---

# 105. No Blind Retry

Incident response SHALL NOT blindly retry UNKNOWN financial operations.

---

# 106. Exposure Assessment

For a financial correctness incident, responders SHALL establish:

```text
affected operation types
time window
Tenants
Legal Entities
Markets
providers
canonical IDs
provider IDs
amounts/currencies
duplicate possibility
settlement status
ERP projection status
```

---

# 107. Canonical Evidence

Canonical database state, provider evidence, event history and reconciliation data SHALL be correlated before corrective money movement.

---

# 108. Logs Alone Are Insufficient

Logs SHALL NOT be the sole evidence for deciding whether money moved.

---

# 109. Provider Dashboard Alone Is Insufficient

Provider-console state SHALL be reconciled with canonical Baobab state.

---

# 110. Financial Recovery

Recovery SHALL prioritise establishing actual external financial state before initiating compensating transactions.

---

# 111. No Automatic Compensating Refund Without Evidence

Suspected duplicate Payment SHALL NOT automatically cause a Refund until sufficient evidence confirms the duplicate and applicable policy permits correction.

---

# 112. No Automatic Replacement Payout

An uncertain Payout SHALL NOT automatically cause a replacement Payout.

---

# 113. UNKNOWN Recovery

UNKNOWN financial operations SHALL follow dedicated recovery procedures.

Conceptually:

```text
UNKNOWN
   │
   ▼
Query HyperSwitch
   │
   ▼
Query Provider where permitted
   │
   ▼
Check Webhook Inbox
   │
   ▼
Check Settlement / Rail Evidence
   │
   ▼
Determine Financial Fact
   │
   ├── SUCCESS
   ├── FAILURE
   └── STILL UNKNOWN
```

---

# 114. Recovery Workers

Automated recovery workers MAY resolve known safe conditions.

They SHALL use the same:

```text
authorization
state machine
idempotency
mapping
audit
```

rules as ordinary processing.

---

# 115. Manual Recovery

Manual recovery is privileged under PAY-0018.

---

# 116. Reconciliation Is Part of Incident Recovery

An incident involving money movement SHALL not be considered fully resolved merely because APIs recover.

Financial reconciliation must also complete.

---

# 117. Incident Resolution Criteria

Depending on incident type, resolution SHOULD require:

```text
unsafe execution stopped
service restored safely
UNKNOWN operations resolved or tracked
canonical state corrected through governed mechanisms
provider state reconciled
settlement exposure known
ERP projection corrected
customer/business impact understood
monitoring normal
```

---

# 118. Operational Recovery ≠ Financial Resolution

The system may be operationally recovered while financial reconciliation remains open.

These states SHALL be distinguished.

---

# 119. Security Incident Response

PAY-0018 security incidents MAY require:

```text
credential revocation
session revocation
provider suspension
route suspension
secret rotation
break-glass activation
forensic preservation
```

---

# 120. Evidence Preservation

Security/financial incidents SHOULD preserve required evidence without retaining prohibited sensitive data unnecessarily.

---

# 121. Audit Preservation

Audit records SHALL survive incident response.

---

# 122. No History Rewriting During Incident

Responders SHALL NOT rewrite historical payment records to make dashboards appear healthy.

---

# 123. Corrective Facts

Corrections SHOULD be represented through:

```text
new canonical facts
reconciliation corrections
controlled administrative records
compensating financial operations
```

as appropriate.

---

# 124. Communication

Incident response SHOULD identify appropriate:

```text
technical owner
financial owner
security owner
business owner
```

depending on incident type.

---

# 125. Legal Entity Notification

Material incidents SHOULD be attributable to affected Legal Entities so appropriate business owners can be informed.

---

# 126. Provider Escalation

Provider incidents SHOULD have documented escalation paths.

---

# 127. Runbooks

Critical alert classes SHALL have operational runbooks.

---

# 128. Minimum Runbooks

At minimum:

```text
Payments API outage
HyperSwitch outage
Provider outage
Provider timeout spike
UNKNOWN payment backlog
Duplicate-payment suspicion
Webhook outage
Outbox failure
Refund failure
Payout uncertainty
Settlement ingestion failure
Reconciliation backlog
Provider credential compromise
Cross-tenant authorization anomaly
Database degradation
```

---

# 129. Runbook Content

A runbook SHOULD state:

```text
symptoms
likely causes
dashboards
queries/tools
containment actions
unsafe actions to avoid
recovery steps
escalation
verification
```

---

# 130. Unsafe Actions

Runbooks SHOULD explicitly identify dangerous responses.

Example:

```text
DO NOT:
retry UNKNOWN charge blindly
switch provider across Legal Entity
manually mark payment CAPTURED
issue replacement Payout without evidence
disable signature verification
bypass idempotency
```

---

# 131. Automation

Safe deterministic incident responses MAY be automated.

Examples:

```text
open provider circuit breaker
pause route after threshold
scale workers
retry outbox publication
```

---

# 132. Automation Boundaries

Automation SHALL NOT make ambiguous compensating financial decisions without sufficient authoritative evidence.

---

# 133. Auto-Suspension

Baobab MAY automatically suspend a provider/route when predefined safety conditions are met.

---

# 134. Auto-Suspension Audit

Automated containment SHALL be observable and auditable.

---

# 135. Provider Recovery

A recovered provider SHALL NOT necessarily be immediately returned to full traffic.

Controlled recovery MAY include:

```text
health validation
configuration validation
small traffic allocation
monitoring
gradual restoration
```

subject to PAY-0009.

---

# 136. Provider Certification Drift

If an incident reveals that provider behaviour no longer matches PAY-0022 certification assumptions, certification MAY be:

```text
suspended
restricted
expired
revoked
```

pending review.

---

# 137. Incident Postmortem

Material incidents SHALL receive a post-incident review.

---

# 138. Blameless Technical Analysis

The post-incident review SHOULD focus on:

```text
system conditions
control failures
detection gaps
decision points
architectural weaknesses
recovery effectiveness
```

rather than simplistic individual blame.

---

# 139. Postmortem Content

A material postmortem SHOULD include:

```text
summary
timeline
impact
financial exposure
affected contexts
root cause
contributing factors
detection
containment
recovery
reconciliation
what worked
what failed
corrective actions
owners
due dates
```

---

# 140. Financial Exposure in Postmortem

Where applicable, record:

```text
gross affected amount
confirmed incorrect amount
recovered amount
outstanding amount
fees/losses
currencies
```

under appropriate access controls.

---

# 141. Corrective Actions

Corrective actions SHOULD be tracked to completion rather than merely documented.

---

# 142. SLO Review After Incident

Material incidents SHOULD trigger review of:

```text
SLIs
SLOs
alerts
runbooks
provider policy
capacity assumptions
recovery procedures
```

where relevant.

---

# 143. Synthetic Monitoring

Baobab SHOULD use safe synthetic monitoring for critical payment paths.

---

# 144. Synthetic Transactions

Synthetic production monitoring SHALL NOT create uncontrolled real financial movement.

---

# 145. Sandbox Monitoring

Provider sandbox flows MAY be continuously tested to detect integration degradation.

However:

```text
Sandbox Healthy
    ≠
Production Provider Healthy
```

---

# 146. Production Probes

Production probes SHOULD favour non-financial health checks or provider-supported safe diagnostics.

---

# 147. Health Endpoints

Payments SHOULD expose:

```text
liveness
readiness
```

semantics.

---

# 148. Liveness

Liveness answers approximately:

> Should this process be restarted?

It SHALL NOT perform financial operations.

---

# 149. Readiness

Readiness answers approximately:

> Can this instance safely serve its intended class of requests?

---

# 150. Dependency Readiness

A temporary provider outage SHALL not necessarily make the entire Payments service unready if other safe operations remain available.

---

# 151. Capability-Aware Health

Where useful, Baobab SHOULD distinguish:

```text
API healthy
collections degraded
refunds degraded
payouts unavailable
provider X unavailable
reconciliation delayed
```

rather than one binary health signal.

---

# 152. Database Health

Database observability SHALL include:

```text
connection saturation
query latency
lock contention
deadlocks
replication health where applicable
storage
transaction failures
```

---

# 153. Financial Concurrency Monitoring

Lock contention or serialization conflicts affecting:

```text
capture
refund
payout
idempotency
```

SHOULD be separately visible.

---

# 154. Redis / HyperSwitch Dependencies

Where HyperSwitch uses supporting infrastructure such as Redis, failures SHALL be distinguishable from Baobab Payments' own persistence health.

---

# 155. Scheduler Health

HyperSwitch/Payments scheduler and recovery-worker health SHALL be observable where they participate in payment lifecycle recovery.

---

# 156. Queue Health

For every critical asynchronous queue:

```text
depth
oldest message age
processing rate
failure rate
retry rate
```

SHOULD be measurable.

---

# 157. Saturation

Capacity saturation SHALL be monitored before it becomes correctness risk.

---

# 158. Backpressure

Payments SHALL prefer controlled backpressure over uncontrolled overload.

---

# 159. Load Shedding

Where load shedding is required, Baobab SHOULD prioritise:

```text
financial mutation correctness
webhook durability
canonical-state integrity
```

over lower-priority analytical workloads.

---

# 160. Critical Workload Priority

A surge in analytics/reporting SHALL NOT starve:

```text
provider webhook processing
idempotency persistence
canonical financial mutations
outbox publication
```

---

# 161. Capacity Planning

Capacity planning SHOULD consider:

```text
peak payment attempts
provider latency
webhook bursts
event throughput
reconciliation batches
settlement files
Payout batches
recovery workload
```

---

# 162. Dependency Budgets

Timeouts and retry budgets SHALL be explicitly configured.

---

# 163. Timeout Observability

Timeouts SHALL identify:

```text
which dependency
which operation
whether request may have reached provider
whether outcome is ambiguous
```

where known.

---

# 164. Timeout ≠ Failure

Per earlier ADRs:

```text
TIMEOUT
    ≠
DEFINITIVE FINANCIAL FAILURE
```

---

# 165. Retry Observability

Retries SHALL distinguish:

```text
transport retry
same-operation idempotent retry
business collection retry
new PaymentAttempt
provider failover
```

---

# 166. Retry Amplification

Metrics SHOULD detect retry storms.

---

# 167. Provider Rate Limits

Rate-limit responses SHOULD be separately observable from provider outages.

---

# 168. Circuit Breaker Metrics

Circuit state transitions SHOULD be recorded:

```text
CLOSED
OPEN
HALF_OPEN
```

or implementation-equivalent semantics.

---

# 169. Configuration Observability

Material configuration changes SHOULD be traceable.

Examples:

```text
provider activation
routing policy
FX pair activation
credential rotation
market payment-method activation
```

---

# 170. Configuration Change Correlation

Incidents SHOULD permit correlation with recent configuration/deployment changes.

---

# 171. Deployment Markers

Observability systems SHOULD record deployment/version markers.

---

# 172. Version Dimensions

Operators SHOULD be able to identify:

```text
Baobab Payments version
HyperSwitch version
connector version
contract version
```

where material.

---

# 173. Release Monitoring

New releases SHOULD receive heightened monitoring during rollout.

---

# 174. Progressive Delivery

Where infrastructure permits, Payments SHOULD support controlled rollout patterns such as:

```text
canary
progressive traffic
rapid rollback
```

without splitting canonical financial authority unsafely.

---

# 175. Rollback Safety

Code rollback SHALL consider database/schema/event compatibility.

---

# 176. Financial State Is Not Rolled Back

Rolling back application code SHALL NOT roll back already executed financial reality.

---

# 177. Schema Migration Observability

Database migrations SHALL be observable for:

```text
duration
lock impact
failure
rollback/recovery state
```

---

# 178. Contract Compatibility

Telemetry SHOULD detect unexpected contract-version failures between Payments and:

```text
Trade
Subscriptions
ERP
Control Plane
event consumers
```

---

# 179. Provider Contract Drift

Unexpected provider response/schema changes SHOULD be observable.

---

# 180. Unknown Provider State

Unknown/unmapped provider states SHOULD produce:

```text
quarantine
metric
alert where material
```

rather than being silently mapped to success/failure.

---

# 181. Audit Versus Observability

Baobab SHALL distinguish:

```text
Observability
   → understand system behaviour

Audit
   → prove attributable controlled action

Reconciliation
   → establish agreement with financial reality
```

All three are required.

---

# 182. Reconciliation as Observability Backstop

Reconciliation acts as a delayed correctness detector for anomalies that real-time telemetry may miss.

---

# 183. External Financial Truth

Provider/settlement evidence may reveal discrepancies despite apparently successful API execution.

Such discrepancies SHALL be observable.

---

# 184. Orphan Detection

Baobab SHOULD detect:

```text
provider transaction without canonical Payment
canonical Payment without expected provider execution
provider Refund without canonical Refund
settlement line without mapped transaction
```

---

# 185. Drift Detection

Drift detection SHOULD cover:

```text
canonical mappings
HyperSwitch resources
provider activation
merchant/profile configuration
routing configuration
settlement configuration
```

---

# 186. Data Quality

Telemetry pipelines SHOULD detect malformed/missing critical dimensions rather than silently producing misleading dashboards.

---

# 187. Clock Synchronisation

Distributed systems participating in payment processing SHOULD maintain reliable clock synchronisation.

---

# 188. Event Time Versus Record Time

PAY-0011 remains authoritative:

```text
occurred_at
    ≠ necessarily
recorded_at
```

Observability SHOULD preserve both where material.

---

# 189. Timezone

Operational timestamps SHOULD use unambiguous machine-readable time representation.

Business-local display MAY be derived separately.

---

# 190. Retention

Telemetry retention SHALL be governed according to:

```text
operational need
security
privacy
cost
incident investigation
regulatory requirements
```

---

# 191. Logs Are Not Permanent Financial Ledger

Long-term financial reconstruction SHALL not depend solely upon log retention.

---

# 192. Sampling

Trace sampling MAY be used for scale.

---

# 193. Error Trace Sampling

Errors, financial anomalies and high-risk operations SHOULD receive stronger retention/sampling policy than ordinary successful traffic.

---

# 194. Financial Debugging

Operators SHOULD be able to trace an individual canonical transaction using IDs without exposing raw payment credentials.

---

# 195. Privacy

Telemetry SHALL respect applicable privacy and data-residency controls.

---

# 196. Cross-Region Telemetry

Centralised observability SHALL not automatically justify exporting restricted raw data across regions.

---

# 197. Tenant Data

Tenant identifiers in operational telemetry SHALL be protected according to access policy.

---

# 198. Provider Commercial Data

Provider rates, fees and commercial routing information may require restricted dashboard access.

---

# 199. SLO Ownership

Every production SLO SHALL have an accountable owner.

---

# 200. Alert Ownership

Every paging alert SHALL have:

```text
owner
severity
runbook
escalation
```

---

# 201. Provider Ownership

Each production provider integration SHOULD have an operational owner.

---

# 202. Reconciliation Ownership

Settlement/reconciliation exceptions SHALL have clear operational ownership.

---

# 203. Incident Commander

Material incidents SHOULD assign a clear incident coordinator/commander.

---

# 204. Financial Decision Authority

The incident commander SHALL not automatically gain authority to issue financial corrections.

PAY-0018 controlled mutation remains applicable.

---

# 205. Incident Roles

A major payment incident MAY require:

```text
Incident Commander
Technical Lead
Payments Operations
Security
Finance / ERP
Business Representative
Communications
Provider Liaison
```

depending on impact.

---

# 206. Incident Timeline

Material actions SHOULD be timestamped and correlated.

---

# 207. Financial Incident Freeze

For severe correctness incidents, Baobab MAY intentionally freeze affected financial mutation while preserving:

```text
read access
webhook ingestion
evidence capture
reconciliation
```

where safe.

---

# 208. Freeze Does Not Mean Drop Webhooks

Containment SHOULD avoid losing provider observations needed to establish truth.

---

# 209. Freeze Does Not Mean Destroy Queues

Queued financial work SHALL be preserved or explicitly cancelled under controlled policy.

---

# 210. Recovery Gate

Before restoring affected money movement, operators SHOULD verify:

```text
root unsafe condition contained
canonical state coherent
idempotency intact
provider configuration correct
authorization correct
routing eligibility correct
queues understood
UNKNOWN exposure understood
monitoring operational
```

---

# 211. Reopening Traffic

Traffic SHOULD be restored progressively where practical.

---

# 212. Verification

Post-recovery verification SHOULD include both:

```text
technical checks
financial checks
```

---

# 213. Technical Verification

Examples:

```text
API healthy
database healthy
queue draining
webhooks processing
outbox publishing
provider reachable
```

---

# 214. Financial Verification

Examples:

```text
no duplicate execution
UNKNOWN backlog declining
provider/canonical state converging
settlement matching
ERP projections current
no cross-boundary anomalies
```

---

# 215. Production Readiness Gate

```text
OBSERVABILITY / SLO / INCIDENT READINESS

Structured logging                         PASS
Sensitive-data redaction                   PASS
OpenTelemetry-compatible tracing           PASS
Correlation propagation                    PASS
Async correlation                          PASS
Service metrics                            PASS
Payment metrics                            PASS
Provider metrics                           PASS
Routing metrics                            PASS
Idempotency metrics                        PASS
UNKNOWN metrics                            PASS
Webhook metrics                            PASS
Inbox metrics                              PASS
Outbox metrics                             PASS
Event metrics                              PASS
Refund metrics                             PASS
Dispute metrics                            PASS
Settlement metrics                         PASS
Reconciliation metrics                     PASS
Payout metrics                             PASS
FX metrics                                 PASS
Security metrics                           PASS
Financial correctness indicators           PASS
Service SLIs                               PASS
Financial SLIs                             PASS
SLO framework                              PASS
Provider dependency visibility             PASS
Error-budget policy                        PASS
Dashboards                                 PASS
Alert ownership                            PASS
Severity model                             PASS
Runbooks                                   PASS
UNKNOWN recovery                           PASS
Financial exposure analysis                PASS
Containment controls                       PASS
Provider suspension                        PASS
Route suspension                           PASS
Security incident integration              PASS
Reconciliation recovery                    PASS
Post-incident process                      PASS
Deployment markers                         PASS
Release monitoring                         PASS
Capacity monitoring                        PASS
Backpressure                               PASS
Multi-region visibility                    PASS
Telemetry access control                   PASS
DR observability                           PASS
------------------------------------------------
Observability & Incident Plane             READY
```

---

# 216. Required Failure Exercises

Before production maturity is claimed, Baobab SHOULD exercise scenarios including:

```text
Payments process crash
database connection exhaustion
database failover
HyperSwitch unavailable
provider unavailable
provider latency spike
provider rate limiting
provider timeout after possible submission
duplicate client request
duplicate provider webhook
webhook outage
outbox publisher failure
event consumer failure
UNKNOWN payment
UNKNOWN Refund
UNKNOWN Payout
provider/canonical mismatch
settlement ingestion failure
large reconciliation backlog
credential revocation
routing configuration drift
provider certification suspension
cross-tenant access attempt
cross-Legal-Entity access attempt
deployment regression
queue saturation
```

---

# 217. Game Days

Controlled reliability exercises SHOULD be performed periodically as operational maturity grows.

---

# 218. No Unsafe Chaos Testing

Production resilience testing SHALL NOT deliberately create uncontrolled financial effects.

---

# 219. Invariants

The following invariants SHALL hold:

1. HTTP success is not financial success.
2. Service availability is not provider availability.
3. Provider availability is not route eligibility.
4. Provider health is not provider authorization.
5. Payment acceptance is not payment completion.
6. Capture is not settlement.
7. Settlement is not reconciliation.
8. Low latency is not financial correctness.
9. Metrics are not canonical payment state.
10. Logs are not canonical payment state.
11. Traces are not canonical payment state.
12. Observability does not replace audit.
13. Audit does not replace reconciliation.
14. Reconciliation does not replace real-time observability.
15. Correlation ID is not canonical identity.
16. Correlation ID is not idempotency identity.
17. Sensitive credentials never enter ordinary telemetry.
18. Provider payloads are not routinely logged raw.
19. Metrics avoid uncontrolled high-cardinality labels.
20. UNKNOWN financial outcomes are explicitly observable.
21. UNKNOWN age is monitored.
22. Timeout is not automatically failure.
23. Missing webhook is not payment failure.
24. Outbox health is observable.
25. Event-consumer lag is observable.
26. Provider health is separately observable.
27. Routing decisions are explainable.
28. Circuit-breaker state is observable.
29. Routing drift is observable.
30. Refund health is observable.
31. Dispute deadlines are observable.
32. Settlement health is observable.
33. Reconciliation backlog is observable.
34. Payout uncertainty receives high operational attention.
35. Security failures are observable.
36. Security metrics are not security audit.
37. Financial correctness has first-class indicators.
38. A severe correctness anomaly may require containment despite healthy uptime.
39. Platform SLOs and provider dependency health are distinguishable.
40. Async acknowledgement is not financial completion.
41. State convergence is measurable.
42. Event convergence is measurable.
43. Reconciliation freshness is measurable.
44. Provider outages are not disguised as Baobab internal outages.
45. Correctness incidents do not receive a routine duplicate-charge error budget.
46. SLO targets are evidence/business based.
47. Legal Entity-level operational visibility is supported.
48. Group aggregation does not weaken isolation.
49. Alerts are actionable.
50. Paging alerts have owners and runbooks.
51. Severity considers financial exposure.
52. One high-value unsafe transaction may outrank large volumes of harmless errors.
53. Financial incidents establish actual external state before compensation.
54. UNKNOWN operations are not blindly retried.
55. UNKNOWN Payouts are not automatically replaced.
56. Suspected duplicate charges are not automatically refunded without evidence.
57. Containment precedes throughput restoration when correctness is uncertain.
58. Containment uses the smallest safe blast radius.
59. Incident response cannot bypass routing eligibility.
60. Incident response cannot bypass idempotency.
61. Incident response cannot bypass authorization.
62. Logs alone are insufficient evidence that money moved.
63. Provider dashboards alone are insufficient canonical evidence.
64. Reconciliation participates in incident recovery.
65. API recovery does not imply financial resolution.
66. Historical financial records are not rewritten during incidents.
67. Corrective financial actions are explicit.
68. Security incidents preserve audit.
69. Critical alert classes have runbooks.
70. Runbooks identify unsafe actions.
71. Safe containment may be automated.
72. Ambiguous financial compensation is not automatically performed.
73. Provider recovery may be gradual.
74. Provider incidents may trigger certification review.
75. Material incidents receive post-incident analysis.
76. Corrective actions are tracked.
77. Synthetic monitoring does not create uncontrolled money movement.
78. Sandbox health does not prove production health.
79. Liveness does not execute financial operations.
80. Readiness is not necessarily identical to provider availability.
81. Capability-aware degradation is preferable to misleading binary health.
82. Database health is observable.
83. Financial concurrency pressure is observable.
84. Critical queues expose depth and age.
85. Backpressure is preferable to uncontrolled overload.
86. Analytical workloads do not starve critical financial processing.
87. Timeout/retry budgets are observable.
88. Retry types are distinguishable.
89. Retry storms are detectable.
90. Deployment changes are correlated with incidents.
91. Financial state is not rolled back with application code.
92. Unknown provider states are quarantined rather than guessed.
93. Reconciliation is a delayed correctness backstop.
94. Orphan external financial operations are detectable.
95. Configuration drift is detectable.
96. Historical reconstruction does not depend solely on logs.
97. Trace sampling does not hide critical financial anomalies by design.
98. Telemetry respects privacy and data residency.
99. Incident commanders do not automatically gain financial mutation authority.
100. Recovery requires both technical and financial verification.
101. Financial safety takes precedence over availability when the two conflict.

---

# 220. Consequences

## Positive

This decision provides:

- end-to-end payment visibility;
- explicit financial correctness monitoring;
- provider isolation;
- route diagnostics;
- duplicate-risk detection;
- UNKNOWN recovery visibility;
- webhook and event-plane monitoring;
- settlement and reconciliation visibility;
- Payout safety monitoring;
- actionable incident response;
- Legal Entity-level operations;
- measurable reliability objectives;
- safer production releases.

## Negative

It requires substantial investment in:

- instrumentation;
- telemetry infrastructure;
- dashboards;
- SLO definition;
- runbooks;
- on-call practices;
- reconciliation monitoring;
- provider monitoring;
- incident exercises;
- telemetry governance.

These costs are accepted because payment systems cannot be operated safely as ordinary stateless APIs.

---

# 221. Alternatives Considered

## Monitor Only HTTP Availability

Rejected.

A financially incorrect service can remain perfectly reachable.

---

## Depend on Provider Dashboards

Rejected.

Providers do not own Baobab canonical state or complete business context.

---

## Use Logs as the Primary Monitoring System

Rejected.

Logs alone do not provide reliable financial-state, SLO or reconciliation semantics.

---

## Treat All Provider Errors as Payments Downtime

Rejected.

Dependency health must remain distinguishable from platform health.

---

## Retry Everything During Incidents

Rejected.

Ambiguous retries can duplicate financial effects.

---

## Restore Availability Before Establishing Financial Exposure

Rejected.

This can amplify incorrect money movement.

---

## Automatically Refund Suspected Duplicates

Rejected.

Corrective financial movement requires authoritative evidence.

---

## One Global Uptime SLO

Rejected.

It conceals provider, event, reconciliation and financial-correctness failures.

---

# 222. Relationship to PAY-0008

PAY-0008 defines canonical state.

PAY-0019 makes state distribution, transition failures, UNKNOWN conditions and convergence observable.

---

# 223. Relationship to PAY-0009

PAY-0009 defines routing and failover safety.

PAY-0019 exposes:

```text
route eligibility
route selection
failover
blocked failover
provider health
circuit state
routing drift
```

for operations.

---

# 224. Relationship to PAY-0010

PAY-0010 defines duplicate safety.

PAY-0019 monitors:

```text
idempotency replays
idempotency conflicts
prevented duplicates
suspected duplicate financial effects
UNKNOWN retry risks
```

---

# 225. Relationship to PAY-0011

PAY-0011 defines webhook/event delivery.

PAY-0019 monitors:

```text
webhook verification
duplicates
inbox backlog
outbox backlog
event publication
consumer lag
DLQ
```

---

# 226. Relationship to PAY-0012

All telemetry obeys sensitive-data minimisation.

Observability is never justification for recording raw payment credentials.

---

# 227. Relationship to PAY-0013

Refund requests, completion, UNKNOWN outcomes and reconciliation are explicitly observable.

---

# 228. Relationship to PAY-0014

Dispute state and evidence deadlines are operational signals.

---

# 229. Relationship to PAY-0015

Settlement/reconciliation forms a financial correctness layer complementary to real-time monitoring.

---

# 230. Relationship to PAY-0016

Payouts receive enhanced observability because uncertain or duplicate outbound money movement carries substantial risk.

---

# 231. Relationship to PAY-0017

FX quote, conversion, variance and reconciliation health are observable without turning telemetry into accounting authority.

---

# 232. Relationship to PAY-0018

Authentication, authorization, controlled mutations, break-glass activity and cross-boundary violations produce operational/security signals.

Observability SHALL NOT itself grant mutation authority.

---

# 233. Relationship to PAY-0020

PAY-0020 SHALL define availability, backup, recovery and disaster resilience.

PAY-0019 provides the measurements necessary to determine:

```text
whether failover is required
whether recovery succeeded
whether financial state converged
whether normal execution may safely resume
```

---

# 234. Relationship to PAY-0021

HyperSwitch upgrades SHALL be accompanied by telemetry capable of detecting:

```text
latency regression
connector regression
state-mapping regression
webhook regression
routing regression
financial correctness anomalies
```

---

# 235. Relationship to PAY-0022

Provider certification and activation SHOULD establish the monitoring expectations required before a provider becomes production routable.

Provider behaviour drifting from certified expectations may trigger:

```text
alert
route exclusion
suspension
recertification
```

---

# 236. Complete Operational Model

```text
                    BUSINESS REQUEST
                           │
                           ▼
                    BAOBAB PAYMENTS
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
           Metrics        Logs        Traces
              │            │            │
              └────────────┼────────────┘
                           ▼
                    Operational View
                           │
                           ▼
                    Payment Execution
                           │
                           ▼
                       HyperSwitch
                           │
                           ▼
                        Provider
                           │
                           ▼
                  External Financial Fact
                           │
                 ┌─────────┴──────────┐
                 ▼                    ▼
              Webhook             Settlement
                 │                    │
                 ▼                    ▼
          Canonical State       Reconciliation
                 │                    │
                 └─────────┬──────────┘
                           ▼
                          ERP
                           │
                           ▼
                  Financial Convergence

OBSERVABILITY watches the whole chain.

AUDIT proves controlled actions.

RECONCILIATION establishes agreement with
external financial reality.

INCIDENT RESPONSE contains and repairs
unsafe divergence.
```

---

# 237. Final Decision

Baobab SHALL permanently distinguish:

```text
UP
    from
HEALTHY

AVAILABLE
    from
FINANCIALLY CORRECT

HTTP SUCCESS
    from
FINANCIAL SUCCESS

PROVIDER HEALTH
    from
PROVIDER ELIGIBILITY

REQUEST LATENCY
    from
FINANCIAL COMPLETION TIME

PAYMENT CAPTURE
    from
SETTLEMENT

SETTLEMENT
    from
RECONCILIATION

OBSERVABILITY
    from
AUDIT

AUDIT
    from
RECONCILIATION

OPERATIONAL RECOVERY
    from
FINANCIAL RESOLUTION

TIMEOUT
    from
FAILURE

UNKNOWN
    from
PERMISSION TO RETRY

SERVICE RESTORATION
    from
PERMISSION TO RESUME UNSAFE MONEY MOVEMENT
```

The operational model is:

```text
INSTRUMENT
     │
     ▼
OBSERVE
     │
     ▼
MEASURE
     │
     ▼
COMPARE AGAINST SLO
     │
     ▼
DETECT ANOMALY
     │
     ▼
ESTABLISH SEVERITY
     │
     ▼
CONTAIN
     │
     ▼
ESTABLISH FINANCIAL REALITY
     │
     ▼
RECOVER
     │
     ▼
RECONCILE
     │
     ▼
VERIFY
     │
     ▼
RESTORE NORMAL EXECUTION
     │
     ▼
LEARN AND HARDEN
```

**Metrics tell Baobab how the system is behaving.  
Logs provide diagnostic context.  
Traces connect distributed execution.  
Audit establishes attributable controlled action.  
Canonical Payments state records operational financial truth.  
Provider and settlement evidence establish external financial facts.  
Reconciliation detects divergence between those worlds.  
SLOs define the reliability expected from the platform.  
Alerts identify conditions requiring intervention.  
Incident response contains unsafe behaviour before it amplifies financial exposure.**

**A payment platform is not healthy merely because requests succeed quickly. It is healthy when authorised financial instructions execute safely, duplicate financial effects are prevented, uncertain outcomes are recovered, provider behaviour remains inside the governed execution envelope, canonical state converges with external financial reality, settlements reconcile, and operators can detect, contain, explain and recover failures without compromising financial truth.**