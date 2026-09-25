# ADR-PAY-0016 — Payouts & Beneficiary Payments

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Outbound Payments / Financial Operations |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0015; ADR-PAY-0022; ADR-SHARED-011; applicable Control Plane, IAM, ERP and Shared contracts |
| **Related** | ADR-PAY-0017 through ADR-PAY-0021 |
| **Primary Domains** | Payouts, beneficiaries, disbursements, outbound money movement, payout routing, returns, reconciliation |

---

# 1. Context

Baobab's initial payment architecture primarily addresses collection:

```text id="61ej1w"
Customer
   │
   ▼
Merchant / Legal Entity
```

Future business requirements may also require outbound money movement:

```text id="t0f7fn"
Legal Entity
    │
    ▼
Beneficiary
```

Examples may include:

- supplier payments;
- marketplace/vendor disbursements;
- partner commissions;
- contractor payments;
- rebates;
- approved customer compensation;
- cross-border supplier settlement;
- platform-mediated disbursement;
- other beneficiary payments.

These operations have materially different:

```text id="oxng8d"
authority
identity
risk
compliance
routing
approval
failure
return
reconciliation
```

semantics from customer collections.

---

# 2. Problem

A naïve implementation might represent:

```text id="oynj60"
Payout
=
negative Payment
```

or:

```text id="b72a7e"
Payout
=
Refund to arbitrary bank account
```

Both are rejected.

A Payment answers:

> How did money enter the merchant payment relationship?

A Payout answers:

> Under what authority may money leave a Legal Entity's controlled financial position and be delivered to a specific beneficiary?

These are different domains.

---

# 3. Decision

`baobab-payments` SHALL model:

```text id="ubqg22"
Payout
```

as a distinct canonical financial aggregate and execution lifecycle.

A Payout SHALL NOT be represented as:

```text id="5dlxwi"
negative Payment
negative Refund
negative Settlement
```

or as a generic provider transfer.

---

# 4. Governing Principle

> **Collection authority does not imply disbursement authority. A payout may execute only when the payer, beneficiary, destination, amount, currency, business obligation, approvals and provider route are independently authorised.**

---

# 5. Direction of Money Movement

Baobab SHALL distinguish:

```text id="qll3sq"
COLLECTION

Payer / Customer
       │
       ▼
Legal Entity
```

from:

```text id="eb0gd9"
PAYOUT

Legal Entity
       │
       ▼
Beneficiary
```

---

# 6. Fundamental Non-Equivalences

Baobab SHALL preserve:

```text id="vbrgup"
Payout
    ≠
Payment

Payout
    ≠
Refund

Payout
    ≠
Settlement

Payout
    ≠
Bank Transfer Payment Method

Payout
    ≠
ERP Accounts Payable

Payout
    ≠
Supplier Invoice

Payout
    ≠
Payroll

Payout
    ≠
Treasury Transfer

Payout
    ≠
Provider Settlement

Beneficiary
    ≠
Customer

Beneficiary
    ≠ automatically
Supplier

Beneficiary Destination
    ≠
Canonical Beneficiary Identity
```

---

# 7. Authority Boundaries

| Concern | Authority |
|---|---|
| Business reason money is owed | Originating business domain |
| Supplier commercial obligation | Relevant procurement/ERP/business domain |
| Payout execution | Payments |
| Canonical Payout lifecycle | Payments |
| Beneficiary payment representation | Payments + authoritative business identity reference |
| Canonical Tenant / Organisation / Legal Entity / Market | Control Plane |
| Workload/user identity | IAM |
| Provider routing | Payments / HyperSwitch within authorised envelope |
| Provider certification/activation | Payments under PAY-0022 |
| Accounting obligation | ERP |
| General ledger | ERP |
| Bank accounting | ERP |
| Payment credentials/secrets | PAY-0012 controls |

---

# 8. Business Obligation Versus Payout

A Payout SHALL reference an authorised business obligation.

Conceptually:

```text id="excmqw"
Business Obligation
       │
       ▼
Payout Request
       │
       ▼
Payout
       │
       ▼
Provider Execution
```

Payments does not decide that a supplier is commercially owed money.

---

# 9. Originating Domains

A payout obligation MAY originate from future or existing domains such as:

```text id="i5vuvp"
ERP
Trade
supplier-management capability
commission capability
marketplace capability
approved compensation workflow
```

The originating domain remains authoritative for:

```text id="v7epbc"
why
how much
to whom commercially
under which obligation
```

subject to Payments validation.

---

# 10. Payout Aggregate

Conceptually:

```text id="95wy6c"
Payout
│
├── payout_id
├── tenant_id
├── organisation_id?
├── legal_entity_id
├── market_id?
├── source_engine
├── source_reference
├── beneficiary_id
├── beneficiary_destination_reference
├── amount
├── currency
├── purpose
├── payout_method
├── status
├── approval_state
├── provider_reference?
├── engine_instance_id
├── idempotency_reference
├── correlation_id
├── simulated
├── requested_at
├── submitted_at?
├── completed_at?
└── revision
```

Exact contracts SHALL be versioned through Shared.

---

# 11. Canonical Identity

Every Payout SHALL have:

```text id="p16id0"
payout_id
```

as its canonical Baobab identity.

Provider transfer/payout IDs remain external references.

---

# 12. Payout Obligation Identity

The source obligation SHALL have its own identity.

Therefore:

```text id="8blkk3"
Supplier Invoice
       ≠
Payout
```

One obligation may, depending on business policy, result in:

```text id="g9qk9c"
one payout
multiple partial payouts
```

and one payout MAY eventually aggregate multiple authorised obligations if explicitly supported.

---

# 13. Beneficiary

A Beneficiary represents the party intended to receive the Payout.

A beneficiary MAY be:

```text id="y77bwi"
supplier
contractor
partner
vendor
customer
other authorised counterparty
```

depending on the business model.

---

# 14. Beneficiary Is Not Payment Destination

Baobab SHALL distinguish:

```text id="9ykrgu"
Beneficiary
       │
       ▼
Beneficiary Destination
```

The beneficiary is the receiving party.

The destination is how funds reach that party.

---

# 15. Beneficiary Destination

A destination MAY represent:

```text id="a6dl3d"
bank account
mobile-money account
wallet
provider recipient token
other supported payout endpoint
```

---

# 16. Destination Reference

Canonical Payout contracts SHOULD normally use a secure:

```text id="y5fwq4"
beneficiary_destination_reference
```

rather than repeatedly carrying raw banking/payment details.

---

# 17. Sensitive Destination Data

Bank-account numbers, wallet credentials and equivalent sensitive destination data SHALL follow PAY-0012 minimisation and vaulting principles.

---

# 18. Beneficiary Identity Versus Destination

The following is prohibited:

```text id="0k87hr"
new bank account
      =
new beneficiary
```

A beneficiary may legitimately have multiple destinations.

---

# 19. Multiple Destinations

Conceptually:

```text id="7j7qau"
Beneficiary B1
    │
    ├── Bank Destination D1
    ├── Mobile Money D2
    └── Wallet D3
```

Each destination requires independent eligibility.

---

# 20. Destination Status

A destination SHOULD support governance states such as:

```text id="pmp0id"
PENDING
VERIFIED
ACTIVE
SUSPENDED
RETIRED
```

Exact contract vocabulary may evolve.

---

# 21. Verification

A payout destination SHALL NOT become production-eligible merely because syntactically valid payment details were supplied.

Verification MAY include:

```text id="pnhu7r"
ownership checks
provider verification
bank/account validation
mobile-money validation
business verification
risk controls
```

as appropriate.

---

# 22. Destination Mutation Risk

Changing a beneficiary's destination is a high-risk operation.

It SHALL be separately authenticated, authorised and audited.

---

# 23. Destination Change Does Not Rewrite History

Historical Payouts SHALL preserve the destination reference used when they executed.

---

# 24. Beneficiary Context

A beneficiary SHALL be scoped sufficiently to prevent cross-tenant or cross-Legal-Entity misuse.

---

# 25. Group Structure

Nabhold Group ownership SHALL NOT imply:

```text id="37v5yy"
ZuriBeans beneficiary
       =
Thamani beneficiary authority
```

even where the same supplier serves both subsidiaries.

---

# 26. Shared Counterparty

The same real-world supplier MAY have canonical relationships with multiple Legal Entities.

That does not make payout authority interchangeable.

---

# 27. External Tenants

External customer organisations and their subsidiaries SHALL receive the same isolation semantics.

---

# 28. Payer Legal Entity

Every production Payout SHALL identify the Legal Entity on whose authority funds are being disbursed.

---

# 29. No Parent Inference

A subsidiary Payout SHALL NOT automatically execute under its parent company's payout account.

---

# 30. No Sibling Inference

A payout for ZuriBeans SHALL NOT execute through Thamani's payout account merely because:

```text id="3yd4re"
same group
same bank
same provider
same currency
same beneficiary
```

---

# 31. Market

Where Market affects payout eligibility, it SHALL be explicit.

Market SHALL not be inferred from the beneficiary's bank-country code alone.

---

# 32. Currency

Payout currency SHALL be explicit.

---

# 33. No Silent Currency Conversion

If the obligation is:

```text id="f3rthq"
UGX 5,000,000
```

Payments SHALL NOT silently execute:

```text id="w2p5yb"
USD equivalent
```

without an explicitly authorised FX model.

PAY-0017 governs conversion.

---

# 34. Source Currency Versus Destination Currency

Where FX is authorised, Baobab SHALL preserve:

```text id="jtwj0d"
source amount
source currency
destination amount
destination currency
FX rate
rate source
rate timestamp
fees
```

where applicable.

---

# 35. Payout Method

Canonical payout methods MAY include:

```text id="m7jijc"
BANK_TRANSFER
MOBILE_MONEY
WALLET
CARD_PUSH
OTHER
```

depending on supported provider capabilities.

Provider-specific method codes SHALL not become canonical vocabulary directly.

---

# 36. Payout Method Eligibility

Method eligibility SHALL consider:

```text id="f8e5m8"
Tenant
Legal Entity
Market
Currency
Beneficiary
Destination
Transaction Type
Provider Certification
Provider Activation
Provider Capability
Compliance State
Environment
```

---

# 37. Collection Provider Does Not Imply Payout Provider

A provider certified for:

```text id="2xoz2r"
card collection
```

is not automatically certified for:

```text id="tbjcvq"
supplier payout
```

---

# 38. PAY-0022 Certification

Payout capability SHALL be separately represented during provider certification and activation.

---

# 39. Provider Eligibility

Conceptually:

```text id="ll5tsd"
PayoutProviderEligible
=
CertifiedForPayout
AND
ActivatedForLegalEntity
AND
MarketEligible
AND
CurrencyEligible
AND
MethodEligible
AND
BeneficiaryEligible
AND
DestinationEligible
AND
TransactionCapabilityEligible
AND
EnvironmentValid
AND
OperationallyAvailable
```

---

# 40. Routing

Payout routing follows the PAY-0009 principle:

> Baobab determines the authorised execution envelope. HyperSwitch may optimise only within that envelope.

---

# 41. Separate Routing Policy

Payout routing policy MAY differ materially from collection routing.

---

# 42. No Arbitrary Failover

Payout failover SHALL obey duplicate-safety requirements at least as strictly as collection.

---

# 43. Payout Duplicate Risk

Duplicate Payout is a direct financial-loss scenario.

Therefore:

```text id="jrsx3e"
timeout
   ≠
permission to pay again
```

---

# 44. Idempotency

PAY-0010 applies fully to Payouts.

Every logical Payout execution SHALL have stable idempotency identity.

---

# 45. Payout Idempotency Scope

A Payout idempotency identity SHALL bind sufficiently to:

```text id="9c9h7h"
Tenant
Legal Entity
source obligation
beneficiary
destination
amount
currency
logical operation
environment
```

---

# 46. Same Key, Different Destination

Reusing the same idempotency identity with a different destination SHALL produce an idempotency conflict.

---

# 47. Same Key, Different Amount

Likewise:

```text id="2fhrhh"
same key
+
different amount
```

SHALL NOT execute.

---

# 48. Durable Idempotency

Payout duplicate protection SHALL be durable.

It SHALL NOT rely solely on:

```text id="ih9in0"
Redis
process memory
short-lived cache
```

---

# 49. Provider Idempotency

Where the payout provider supports idempotency, Payments SHALL propagate or derive a stable provider operation identity.

Provider idempotency remains defense-in-depth.

---

# 50. Cross-Provider Idempotency

Baobab SHALL NEVER assume:

```text id="6lvt87"
Provider A idempotency
=
Provider B idempotency
```

---

# 51. Unknown Outcome

If the provider may have received the payout request but Baobab loses the response:

```text id="hsidb3"
SUBMITTED
   │
   X
timeout
   │
   ▼
UNKNOWN
```

Payments SHALL recover the state before unsafe failover.

---

# 52. Unknown Is Not Failed

```text id="a09kce"
UNKNOWN
    ≠
FAILED
```

This invariant is especially critical for outbound money.

---

# 53. Payout State Machine

Canonical Payout state SHOULD support semantics such as:

```text id="sm72je"
REQUESTED
PENDING_APPROVAL
APPROVED
PROCESSING
REQUIRES_ACTION
SUBMITTED
SUCCEEDED
FAILED
CANCELLED
RETURNED
UNKNOWN
```

Exact enumeration may evolve.

---

# 54. Requested Is Not Approved

```text id="0yv6zj"
Payout REQUESTED
       ≠
Payout APPROVED
```

---

# 55. Approved Is Not Submitted

```text id="nb4idj"
APPROVED
    ≠
money moved
```

---

# 56. Submitted Is Not Succeeded

A provider accepting a payout request does not necessarily mean the beneficiary received funds.

---

# 57. Succeeded

`SUCCEEDED` SHALL require sufficient authoritative provider/rail evidence according to the payout method.

---

# 58. Returned

A payout MAY succeed operationally and subsequently be returned.

Therefore:

```text id="3nqvqe"
SUCCEEDED
   │
   ▼
RETURNED
```

may represent valid historical evolution.

The original success SHALL not be erased.

---

# 59. Return Is Not Refund

A returned Payout SHALL NOT be represented as a customer Refund.

---

# 60. Payout Return

Return reasons MAY include:

```text id="81j2pb"
invalid destination
closed account
beneficiary mismatch
bank rejection
compliance rejection
unclaimed funds
rail rejection
other return
```

Provider codes SHALL be normalised where useful.

---

# 61. Payout Cancellation

A Payout MAY be cancellable before provider execution reaches an irreversible state.

---

# 62. Cancellation Is State-Dependent

Baobab SHALL NOT assume every payout can be cancelled.

---

# 63. Post-Success Cancellation

Once money has successfully moved, `cancel` SHALL NOT be used to pretend the Payout never happened.

Any recovery requires a distinct authorised financial process.

---

# 64. Beneficiary Return

If the beneficiary voluntarily returns funds later, that is a new financial fact.

It SHALL not rewrite the original Payout.

---

# 65. Approval

Payouts SHALL support an approval boundary independently of execution.

---

# 66. Approval Authority

Approval MAY depend on:

```text id="znks78"
Legal Entity
amount
currency
beneficiary type
destination
market
purpose
source obligation
risk
role
```

---

# 67. Separation of Duties

High-risk Payouts SHOULD support:

```text id="c1m3xe"
maker
   ≠
checker
```

where required.

---

# 68. Four-Eyes Principle

A Legal Entity MAY require:

```text id="1yww3e"
Requester
    │
    ▼
Approver
    │
    ▼
Payments execution
```

for configured payout classes.

---

# 69. Approval Does Not Override Eligibility

An approved Payout may still fail eligibility because of:

```text id="eq9m3r"
provider suspension
destination suspension
currency restriction
compliance restriction
market restriction
```

---

# 70. Approval Expiry

Approval MAY expire if execution does not occur within a configured interval or if material payout details change.

---

# 71. Material Change

Changing:

```text id="8fbv5e"
beneficiary
destination
amount
currency
Legal Entity
```

after approval SHALL normally invalidate that approval.

---

# 72. No Approval Reuse

Approval for one Payout SHALL NOT be reused for another Payout unless the governing business workflow explicitly supports batch authority.

---

# 73. Batch Payout

Baobab MAY later support:

```text id="8dhjtv"
PayoutBatch
```

as an orchestration aggregate.

A batch SHALL NOT eliminate individual payout identity.

---

# 74. Batch Identity

Conceptually:

```text id="d3zpxe"
PayoutBatch
   │
   ├── Payout P1
   ├── Payout P2
   └── Payout P3
```

Each child remains independently traceable.

---

# 75. Batch Partial Failure

A batch MAY partially succeed.

Therefore:

```text id="x2sq9f"
Batch submitted
     ≠
all payouts succeeded
```

---

# 76. Compliance Eligibility

Payout execution SHALL permit integration with applicable compliance controls.

Potential controls may include:

```text id="07l7nj"
beneficiary verification
sanctions screening
restricted-party screening
jurisdiction restrictions
transaction limits
source-of-funds controls
purpose restrictions
provider requirements
```

where legally or contractually required.

---

# 77. Compliance Authority

Payments SHALL enforce required payout eligibility but SHALL NOT invent legal/compliance determinations outside its authoritative inputs.

---

# 78. Unknown Compliance State

Where a required compliance determination is unavailable:

```text id="mytf8h"
UNKNOWN
```

SHALL fail closed for new payout execution.

---

# 79. Compliance Result Provenance

Payments SHOULD retain sufficient reference to determine:

```text id="xgk19q"
which check
which policy/version
which result
when
which subject
```

authorised the execution.

---

# 80. No Permanent Compliance Assumption

A beneficiary passing a check once SHALL not imply perpetual eligibility.

Re-evaluation policy MAY depend on risk and regulation.

---

# 81. Provider Compliance

A provider performing its own compliance checks does not automatically satisfy all Baobab/Legal Entity obligations.

---

# 82. Provider Rejection

A provider compliance rejection SHALL be translated into canonical failure semantics without exposing unnecessarily sensitive provider internals.

---

# 83. Beneficiary Data Minimisation

Payments SHALL store only beneficiary data necessary for payout execution, audit and regulatory obligations.

---

# 84. Canonical Counterparty Boundary

If Baobab later establishes a canonical Counterparty domain, Payments SHALL reference it rather than create a competing enterprise counterparty master.

---

# 85. No Shadow Supplier Master

Payments SHALL NOT recreate supplier master data from ERP/Trade merely to execute payouts.

---

# 86. Supplier Example

Conceptually:

```text id="4jdcqp"
ERP / Procurement
      │
      ▼
Supplier Obligation
      │
      ▼
Approved Payout Instruction
      │
      ▼
Baobab Payments
      │
      ▼
Beneficiary / Destination validation
      │
      ▼
Provider execution
```

---

# 87. ZuriBeans Example

A future ZuriBeans supplier payout may be:

```text id="5g5dfv"
ZuriBeans Legal Entity
       │
       ▼
Approved Supplier Obligation
       │
       ▼
Supplier Beneficiary
       │
       ▼
Verified Destination
       │
       ▼
Eligible Payout Provider
       │
       ▼
Supplier
```

---

# 88. Thamani Example

A Thamani vendor payout follows its own:

```text id="af05rv"
Legal Entity
merchant/provider configuration
approval authority
beneficiary relationship
settlement/reconciliation
```

even if the supplier is also used by ZuriBeans.

---

# 89. Same Beneficiary Across Subsidiaries

Conceptually:

```text id="6v5pj5"
Supplier S
   ▲                 ▲
   │                 │
ZuriBeans          Thamani
   │                 │
Payout P1         Payout P2
```

P1 and P2 remain independent obligations and financial operations.

---

# 90. Marketplace Payout

If Baobab later supports marketplace operations:

```text id="l7gpyk"
Customer Collection
       │
       ▼
Platform / Merchant
       │
       ▼
Marketplace Obligation
       │
       ▼
Vendor Payout
```

The collection and payout remain distinct aggregates.

---

# 91. No Automatic Collection-to-Payout Coupling

A successful customer Payment SHALL NOT automatically authorise a beneficiary Payout.

---

# 92. Funds Availability

Future payout policy MAY require proof that funds are:

```text id="qdrdve"
captured
settled
cleared
otherwise available
```

before payout.

The originating business/treasury policy SHALL define the requirement.

---

# 93. Settlement Does Not Automatically Authorise Payout

```text id="01zx61"
Settlement received
       ≠
Payout approved
```

---

# 94. Payout Funding Source

A Payout MAY be funded from:

```text id="up2y5g"
provider balance
merchant balance
bank account
wallet
other approved source
```

depending on provider architecture.

Funding source SHALL be governed configuration.

---

# 95. Funding Source Is Not Legal Entity

The account from which funds are technically drawn does not independently define the canonical payer Legal Entity.

---

# 96. Treasury Transfer

Moving money between a Legal Entity's own accounts is not automatically a beneficiary Payout.

A future treasury domain may own such transfers.

---

# 97. Intercompany Transfer

Payments between related Legal Entities SHALL NOT be hidden as ordinary Payouts merely because they involve outbound money.

Intercompany transactions require appropriate commercial/accounting authority.

---

# 98. Payroll

Employee payroll SHALL NOT automatically be modelled as generic Payouts.

A payroll system may use the payout execution capability while retaining payroll-domain authority.

---

# 99. Refund Alternative Destination

PAY-0013 prohibits silently redirecting a Refund to an arbitrary destination.

If business policy lawfully requires separate compensation to another destination, that operation MAY be represented as an authorised Payout rather than falsifying Refund semantics.

---

# 100. Payout Execution Path

Preferred architecture:

```text id="dh0ofc"
Originating Business Engine
        │
        ▼
Payout Obligation
        │
        ▼
Baobab Payments API
        │
        ▼
Canonical Payout
        │
        ▼
Eligibility / Approval
        │
        ▼
PaymentProvider / Payout Port
        │
        ▼
HyperSwitch Adapter
        │
        ▼
HyperSwitch
        │
        ▼
Eligible Payout Provider / Rail
        │
        ▼
Beneficiary
```

where HyperSwitch supports the required payout capability.

---

# 101. HyperSwitch Capability

Baobab SHALL NOT assume HyperSwitch supports every payout rail or lifecycle required by Baobab.

Actual supported capabilities SHALL be verified during implementation and provider certification.

---

# 102. Provider Abstraction

The Payments provider abstraction SHALL be capable of representing payout operations without forcing them through collection-only semantics.

---

# 103. Adapter Boundary

Provider-specific payout concepts SHALL remain behind:

```text id="kgycc7"
PaymentProvider / Payout Provider adapter
```

or an equivalent clean port.

---

# 104. No Direct Estate Integration

ZuriBeans, Thamani and other estates SHALL NOT integrate directly with payout PSP APIs where Baobab Payments governs the operation.

---

# 105. No Direct ERP-to-PSP Integration

ERP SHALL not normally execute PSP payout APIs directly.

ERP may originate the authorised obligation/instruction.

Payments executes the financial operation.

---

# 106. Payout Webhooks

Provider payout callbacks SHALL follow PAY-0011:

```text id="vuxdtp"
receive
verify
deduplicate
map
validate
transition
outbox
publish
```

---

# 107. Untrusted Callback

A callback stating:

```text id="nkk7hm"
payout succeeded
```

does not become canonical truth until verified.

---

# 108. Unknown Payout Callback

A valid provider payout event with no known canonical Payout SHALL enter reconciliation.

It SHALL NOT silently create a new authorised Payout.

---

# 109. Out-of-Band Payout

A provider-console payout performed outside Baobab governance is a significant financial-control anomaly unless explicitly approved.

---

# 110. Out-of-Band Reconciliation

Such a payout SHALL be:

```text id="m25bkj"
detected
investigated
reconciled
audited
```

---

# 111. Payout Events

Representative canonical events MAY include:

```text id="f6p6yi"
payout.requested
payout.approved
payout.processing
payout.submitted
payout.succeeded
payout.failed
payout.cancelled
payout.returned
payout.unknown
```

Exact names belong in Shared contracts.

---

# 112. Event Facts

```text id="b6vdc4"
payout.approved
```

does not mean money moved.

Likewise:

```text id="2jhhj7"
payout.submitted
```

does not necessarily mean the beneficiary received funds.

---

# 113. Transactional Outbox

Payout events SHALL follow PAY-0011 transactional-outbox semantics.

---

# 114. Consumer Idempotency

ERP and originating business engines SHALL consume payout events idempotently.

---

# 115. Accounting Boundary

A Payout is not an accounting journal.

ERP determines:

```text id="4esg39"
payable settlement
cash movement
expense recognition
asset treatment
tax treatment
exchange differences
fees
```

---

# 116. Payout Success Versus Liability

A successful Payout may satisfy an ERP payable, but Payments SHALL not independently decide that the payable is extinguished.

ERP makes that determination.

---

# 117. ERP Projection

Conceptually:

```text id="ltmlq6"
Payout SUCCEEDED
       │
       ▼
Canonical Payout Fact
       │
       ▼
ERP
       │
       ▼
Accounting Consequence
```

---

# 118. ERP Unavailable

ERP unavailability SHALL not rewrite a provider-confirmed Payout as failed.

The financial fact is durably projected later.

---

# 119. Settlement/Reconciliation

Payouts SHALL participate in PAY-0015 reconciliation where applicable.

---

# 120. Payout Reconciliation

Baobab SHOULD reconcile:

```text id="o17mjd"
Payout request
Payout provider execution
Provider payout status
Provider fee
Payout return
Provider settlement/balance effect
ERP projection
```

---

# 121. Payout Fee

Payout fees SHALL remain distinct from payout principal.

---

# 122. Beneficiary Amount

Where:

```text id="t9wshh"
Payout principal = 1,000
Provider fee = 20
```

Baobab SHALL explicitly distinguish whether:

```text id="g2h36g"
beneficiary receives 1,000
payer debited 1,020
```

or:

```text id="k35bwo"
beneficiary receives 980
payer debited 1,000
```

according to the authorised fee model.

---

# 123. Fee Model

Fee-bearing semantics SHALL be explicit before execution.

Payments SHALL not infer them after the provider response.

---

# 124. Provider Return

A returned payout may generate:

```text id="3azrt8"
principal return
fee non-return
additional return fee
FX difference
```

These SHALL be represented separately where available.

---

# 125. Reconciliation Exception

Examples include:

```text id="r2y7mq"
PAYOUT_NOT_FOUND
BENEFICIARY_MISMATCH
DESTINATION_MISMATCH
AMOUNT_MISMATCH
CURRENCY_MISMATCH
DUPLICATE_PAYOUT
PROVIDER_STATUS_MISMATCH
RETURN_UNMATCHED
FEE_MISMATCH
ERP_PROJECTION_MISSING
```

---

# 126. Destination Mismatch

A provider reporting a destination inconsistent with the authorised Payout SHALL be treated as a high-severity exception.

---

# 127. Amount Mismatch

A provider execution amount inconsistent with the authorised amount SHALL not be silently reconciled.

---

# 128. Observability

A Payout SHOULD be traceable through:

```text id="a8wxip"
payout_id
source_engine
source_reference
tenant_id
legal_entity_id
beneficiary_id
destination_reference
amount
currency
payout_method
provider
external_payout_reference
approval_reference
correlation_id
```

without exposing sensitive destination details.

---

# 129. Metrics

Representative metrics MAY include:

```text id="n0a7wo"
payout_requested_total
payout_approved_total
payout_submitted_total
payout_succeeded_total
payout_failed_total
payout_returned_total
payout_unknown_total
payout_amount_total
payout_duplicate_prevented_total
payout_approval_duration
payout_processing_duration
payout_reconciliation_exception_total
out_of_band_payout_total
```

---

# 130. Alerting

Alerts SHOULD cover:

```text id="4r36p5"
high-value payout
unusual payout volume
destination changed before payout
duplicate payout attempt
payout unknown state
payout return spike
provider payout outage
cross-Legal-Entity mismatch
out-of-band payout
approval bypass attempt
reconciliation mismatch
```

---

# 131. Audit

Payout audit evidence SHOULD preserve:

```text id="v9apdb"
payout_id
source obligation
Tenant
Legal Entity
beneficiary
destination reference
amount
currency
purpose
requesting identity
approving identity
approval policy
provider
provider reference
state transitions
idempotency identity
correlation
timestamps
```

---

# 132. No Sensitive Destination in Logs

Logs SHALL NOT contain full:

```text id="6z1q44"
bank-account numbers
wallet credentials
mobile-money secrets
provider credentials
```

---

# 133. API Design

Canonical APIs SHOULD expose:

```text id="w3kb2v"
create payout
approve payout
cancel payout
query payout
```

rather than provider-specific transfer endpoints.

---

# 134. Client Cannot Select Provider Arbitrarily

The caller SHALL NOT normally specify:

```text id="k5ogab"
PSP
connector
HyperSwitch Merchant
Business Profile
provider credential
```

to bypass routing policy.

---

# 135. Client Cannot Override Payer

A caller SHALL NOT change the payer Legal Entity through arbitrary request metadata.

---

# 136. Client Cannot Override Beneficiary After Approval

Material beneficiary/destination changes require a new validation/approval cycle.

---

# 137. Production Payout Readiness

A Legal Entity SHALL NOT execute production Payouts until at least:

```text id="j6g85s"
Payout capability bound
Legal Entity authorised
Provider certified
Provider activated
Beneficiary governance available
Destination controls available
Approval policy configured
Currency/method eligible
Idempotency operational
Unknown-state recovery operational
Webhook/reconciliation operational
ERP projection operational
Audit/observability operational
```

---

# 138. Sandbox

Sandbox Payouts SHALL be:

```text id="68q7g5"
simulated = true
```

when using Baobab's simulated provider.

---

# 139. Sandbox Cannot Move Production Money

The service SHALL refuse simulated payout mechanisms in production money-movement paths.

---

# 140. Production Testing

Provider production activation SHOULD require controlled payout certification, including safe low-value testing where permitted.

---

# 141. Test Matrix

At minimum:

```text id="kx9dhq"
successful payout
approval required
approval rejected
approval expired
destination suspended
destination changed after approval
duplicate request
concurrent duplicate request
same key different amount
same key different destination
provider timeout before submission
provider timeout after submission
UNKNOWN recovery
provider failure
safe retry
unsafe failover blocked
payout returned
partial batch failure
provider webhook duplicate
provider webhook out of order
unknown external payout
out-of-band payout
cross-tenant attack
cross-Legal-Entity attack
currency mismatch
fee mismatch
ERP unavailable
provider deactivation
```

---

# 142. Standard Payout Flow

```text id="tr2kks"
Business Obligation
       │
       ▼
Payout Requested
       │
       ▼
Resolve Tenant
       │
       ▼
Resolve Legal Entity
       │
       ▼
Resolve Beneficiary
       │
       ▼
Resolve Destination
       │
       ▼
Validate Amount / Currency
       │
       ▼
Compliance Eligibility
       │
       ▼
Approval
       │
       ▼
Provider Eligibility
       │
       ▼
Routing
       │
       ▼
Execution
       │
       ▼
Provider Outcome
       │
       ▼
Canonical Payout Fact
       │
       ├────────► Originating Domain
       └────────► ERP
```

---

# 143. Unknown-Outcome Flow

```text id="0pbzw6"
Payout P1
   │
   ▼
Provider submission
   │
   X
Connection lost
   │
   ▼
UNKNOWN
   │
   ▼
DO NOT CREATE P2
   │
   ▼
Query provider / HyperSwitch
   │
   ├── found succeeded
   │        │
   │        ▼
   │    P1 SUCCEEDED
   │
   ├── found failed
   │        │
   │        ▼
   │    P1 FAILED
   │
   └── still unknown
            │
            ▼
        continue recovery /
        operator escalation
```

---

# 144. Approval Flow

```text id="76kv1m"
Payout Request
     │
     ▼
Policy Evaluation
     │
 ┌───┴──────────────┐
 ▼                  ▼
No approval       Approval required
required              │
 │                    ▼
 │                 Requester
 │                    │
 │                    ▼
 │                 Approver
 │                    │
 │              ┌─────┴─────┐
 │              ▼           ▼
 │           APPROVED     REJECTED
 │              │
 └──────────────┘
        │
        ▼
Revalidate material context
        │
        ▼
Execute
```

---

# 145. Destination-Change Flow

```text id="q1t1d7"
Payout APPROVED
      │
      ▼
Destination changed
      │
      ▼
Existing approval invalidated
      │
      ▼
Reverify destination
      │
      ▼
Re-evaluate policy
      │
      ▼
New approval if required
```

---

# 146. Payout Return Flow

```text id="izuk0k"
Payout
SUCCEEDED
   │
   ▼
Provider / Bank later returns funds
   │
   ▼
Verified return event
   │
   ▼
Payout RETURNED
   │
   ▼
Canonical return fact
   │
   ├────► Originating Domain
   ├────► ERP
   └────► Reconciliation
```

The original successful execution remains historical fact.

---

# 147. Multi-Tenant / Multi-Legal-Entity Architecture

```text id="b94q6z"
                       BAOBAB
                          │
          ┌───────────────┼────────────────┐
          ▼               ▼                ▼
      Tenant A        Tenant B        External Tenant
          │               │                │
     ┌────┴────┐          │           ┌────┴────┐
     ▼         ▼          ▼           ▼         ▼
 ZuriBeans  Thamani    Entity B1   Subsidiary X Y
     │         │          │           │         │
     ▼         ▼          ▼           ▼         ▼
Payout Ctx Payout Ctx Payout Ctx  Payout Ctx Payout Ctx
     │         │          │           │         │
     ▼         ▼          ▼           ▼         ▼
Eligible Providers constrained independently
```

No corporate relationship silently widens payout authority.

---

# 148. Production Readiness Gate

```text id="96urxq"
PAYOUT & BENEFICIARY READINESS

Canonical Payout aggregate                    PASS
Source-obligation linkage                     PASS
Beneficiary model                             PASS
Destination model                             PASS
Destination verification                     PASS
Sensitive-data protection                     PASS
Tenant isolation                              PASS
Legal Entity isolation                        PASS
Currency semantics                            PASS
Payout-method model                           PASS
Provider certification                        PASS
Provider activation                           PASS
Provider eligibility                          PASS
Payout routing                                PASS
Approval workflow                             PASS
Separation of duties                          PASS
Material-change invalidation                  PASS
Idempotency                                   PASS
Concurrency protection                        PASS
Provider idempotency                          PASS
Unknown-outcome recovery                      PASS
Failover safety                               PASS
Return lifecycle                              PASS
Webhook handling                              PASS
Canonical events                              PASS
Transactional outbox                          PASS
Compliance eligibility boundary               PASS
ERP projection                                PASS
Settlement/reconciliation                     PASS
Out-of-band detection                         PASS
Observability                                 PASS
Audit                                         PASS
Sandbox/prod separation                       PASS
Production certification tests                PASS
---------------------------------------------------
Payout & Beneficiary Plane                    READY
```

---

# 149. Invariants

The following invariants SHALL hold:

1. Payout is not Payment.
2. Payout is not Refund.
3. Payout is not Settlement.
4. Payout is not Accounts Payable.
5. Payout is not Supplier Invoice.
6. Payout is not Treasury Transfer.
7. Payout is not Payroll.
8. Payout has its own canonical identity.
9. Provider payout ID remains external.
10. Every Payout identifies the payer Legal Entity.
11. Every Payout identifies its authorised business source.
12. Beneficiary is distinct from destination.
13. Destination does not define canonical beneficiary identity.
14. A beneficiary may have multiple destinations.
15. Destination eligibility is independently governed.
16. Destination changes are high-risk mutations.
17. Historical Payout destination is immutable as history.
18. Group membership does not imply payout authority.
19. Shared supplier identity does not imply shared payout authority.
20. Parent and subsidiary payout authority remain distinct.
21. Sibling subsidiaries cannot use each other's payout context implicitly.
22. External tenants receive equivalent isolation.
23. Payout currency is explicit.
24. Silent FX is prohibited.
25. Payout method is canonical/provider-independent.
26. Collection-provider certification does not imply payout certification.
27. Payout provider activation is capability-specific.
28. Provider eligibility is evaluated before routing.
29. Routing cannot make an ineligible provider eligible.
30. Duplicate payout prevention is mandatory.
31. Timeout is not permission to pay again.
32. UNKNOWN is not FAILED.
33. Provider idempotency is defense-in-depth.
34. Cross-provider idempotency is never assumed.
35. Durable Baobab idempotency is mandatory.
36. Same idempotency key with changed amount is rejected.
37. Same idempotency key with changed destination is rejected.
38. Requested is not approved.
39. Approved is not submitted.
40. Submitted is not succeeded.
41. Payout success does not itself prove ERP liability settlement.
42. Approval does not override provider eligibility.
43. Material change invalidates approval where policy requires.
44. High-risk payouts may require separation of duties.
45. Collection success does not automatically authorise payout.
46. Settlement receipt does not automatically authorise payout.
47. Beneficiary compliance eligibility is explicit where required.
48. Unknown required compliance state fails closed.
49. Payments does not invent compliance determinations.
50. Payments does not recreate supplier master data.
51. ERP remains authoritative for accounting.
52. Payments remains authoritative for payout execution state.
53. Payout fee is distinct from payout principal.
54. Fee semantics are explicit.
55. Returned Payout does not erase historical success.
56. Payout return is not Refund.
57. Post-success cancellation does not rewrite money movement.
58. Payout events use transactional-outbox semantics.
59. Consumers are idempotent.
60. Provider webhooks are untrusted until verified.
61. Unknown external payouts enter reconciliation.
62. Out-of-band payouts are financial-control exceptions.
63. Caller cannot override payer Legal Entity.
64. Caller cannot arbitrarily choose provider/connector.
65. Sensitive destination data is minimised.
66. Sensitive destination data is not logged.
67. Payouts participate in settlement/reconciliation.
68. ERP outage does not rewrite payout financial truth.
69. Sandbox and production payouts remain isolated.
70. Simulated payout execution cannot move production money.
71. Current destination changes do not rewrite historical Payouts.
72. Provider deactivation preserves necessary historical payout/reconciliation operations.
73. Payout routing and collection routing may differ.
74. A batch does not erase individual payout identity.
75. Partial batch failure is representable.
76. No availability objective justifies duplicate or unauthorised outbound money movement.

---

# 150. Consequences

## Positive

This decision provides:

- a clean outbound-money domain;
- strong Legal Entity isolation;
- explicit beneficiary governance;
- destination security;
- approval controls;
- duplicate-payment protection;
- provider independence;
- multi-market payout support;
- cross-border readiness;
- safe supplier disbursement;
- ERP separation;
- reconciliation;
- future marketplace support;
- future external-tenant support.

## Negative

It introduces:

- a dedicated Payout aggregate;
- beneficiary/destination governance;
- approval workflows;
- provider-specific payout adapters;
- compliance integration points;
- return handling;
- payout reconciliation;
- additional security controls;
- potentially substantial regulatory obligations.

These costs are accepted because outbound money movement has materially different risk from payment collection.

---

# 151. Alternatives Considered

## Model Payout as negative Payment

Rejected.

Direction, authority, beneficiary, approval and risk differ.

---

## Model Payout as Refund

Rejected.

Refund is tied to returning value associated with an original Payment.

---

## Let ERP call payout PSP directly

Rejected as the normal architecture.

ERP owns the obligation/accounting; Payments owns external payment execution.

---

## Let each digital estate integrate payout providers

Rejected.

This fragments provider governance, credentials, audit and routing.

---

## Reuse customer identity as beneficiary automatically

Rejected.

The receiving party and payout destination require explicit payout semantics.

---

## Allow caller to provide arbitrary bank details for each Payout

Rejected as the default.

It weakens destination governance and increases fraud risk.

---

## Automatically pay when provider settlement arrives

Rejected.

Settlement availability is not business approval.

---

## Retry through another provider after timeout

Rejected unless the original external outcome is sufficiently known.

---

## Treat returned Payout as failed Payment

Rejected.

A return is a distinct outbound-money lifecycle fact.

---

# 152. Relationship to PAY-0002

PAY-0002 established:

```text id="wlvwsa"
Payout
    ≠
Payment
```

PAY-0016 defines that distinct aggregate and authority boundary.

---

# 153. Relationship to PAY-0003 / PAY-0004

Tenant, Organisation and Legal Entity context remains canonical Baobab context.

Provider payout accounts SHALL NOT redefine organisational hierarchy.

---

# 154. Relationship to PAY-0005

Market, currency and payment-method context principles extend to outbound execution, while payout-specific methods and destination eligibility remain independently modelled.

---

# 155. Relationship to PAY-0008

PAY-0008 owns canonical collection lifecycle principles.

PAY-0016 applies equivalent rigor to a distinct Payout lifecycle without forcing Payout into Payment states.

---

# 156. Relationship to PAY-0009

PAY-0009 routing principles apply:

```text id="5qsp0f"
Eligible Payout Routes
        │
        ▼
Routing
        │
        ▼
Selected Payout Route
```

HyperSwitch may select only among routes Baobab has authorised.

---

# 157. Relationship to PAY-0010

Payout duplicate safety is among the highest-risk applications of PAY-0010.

The governing rule remains:

> **Retry the command, not the money movement.**

---

# 158. Relationship to PAY-0011

Payout provider callbacks follow canonical webhook verification, deduplication, state-transition and outbox semantics.

---

# 159. Relationship to PAY-0012

Beneficiary destinations and provider payout credentials are sensitive payment data.

Tokenisation/reference-based handling and strict access controls apply.

---

# 160. Relationship to PAY-0013

A Refund returns value in relation to an original Payment.

A Payout sends money to an independently authorised beneficiary.

```text id="spnl2c"
Original Payment
      │
      ▼
Refund
```

is different from:

```text id="8umqkb"
Business Obligation
      │
      ▼
Beneficiary Payout
```

---

# 161. Relationship to PAY-0014

Dispute/chargeback recovery SHALL NOT be hidden as arbitrary Payout.

Any outbound money arising from dispute operations remains linked to the dispute financial lifecycle.

---

# 162. Relationship to PAY-0015

PAY-0015 reconciles Payout-related:

```text id="86d3y4"
principal
fees
returns
provider balance movement
ERP projection
```

where applicable.

Payout execution does not become accounting authority.

---

# 163. Relationship to PAY-0017

PAY-0017 is especially important for cross-border Payouts.

It SHALL define:

```text id="tf7ehd"
source currency
destination currency
FX authority
rate source
rate lock
conversion
cross-border context
settlement currency
```

without hiding FX inside provider execution.

---

# 164. Relationship to PAY-0018

PAY-0018 SHALL define detailed authorization for:

```text id="nngv1k"
payout create
payout approve
payout cancel
beneficiary read
destination create
destination verify
destination change
payout reconciliation
administrative recovery
```

including separation of duties.

---

# 165. Relationship to PAY-0019

PAY-0019 SHALL monitor:

```text id="pb17cd"
payout latency
payout failure
unknown outcomes
returns
approval backlog
provider outage
reconciliation backlog
out-of-band payouts
```

---

# 166. Relationship to PAY-0020

Disaster recovery SHALL preserve:

```text id="n7mlgp"
Payout state
idempotency state
approval state
beneficiary references
destination references
provider references
outbox
reconciliation state
```

before outbound execution resumes.

A stale DR region SHALL NOT independently replay Payouts.

---

# 167. Relationship to PAY-0021

HyperSwitch upgrades SHALL be assessed for changes to:

```text id="7o03l1"
payout APIs
recipient models
transfer models
payout routing
webhooks
idempotency
return semantics
provider connectors
```

where those capabilities are used.

---

# 168. Relationship to PAY-0022

Production payout provider activation requires certification specific to outbound money movement.

Conceptually:

```text id="bvh9xp"
Provider
   │
   ▼
Connector Integrated
   │
   ▼
Payout Capability Certified
   │
   ▼
Legal Entity Activated
   │
   ▼
Market/Currency/Method Eligible
   │
   ▼
Payout Route Eligible
```

Collection certification alone is insufficient.

---

# 169. Complete Outbound-Money Architecture

```text id="wrqjsf"
                   BUSINESS DOMAIN
                         │
                         ▼
                FINANCIAL OBLIGATION
                         │
                         ▼
                  PAYOUT REQUEST
                         │
                         ▼
                  BAOBAB PAYMENTS
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
      Beneficiary    Destination     Context
      Validation     Validation      Validation
          │              │              │
          └──────────────┼──────────────┘
                         ▼
                 Compliance Checks
                         │
                         ▼
                      Approval
                         │
                         ▼
                Provider Eligibility
                         │
                         ▼
                       Routing
                         │
                         ▼
                    HyperSwitch
                         │
                         ▼
                 Payout Provider
                         │
                         ▼
                    Beneficiary
                         │
                         ▼
               Canonical Payout Fact
                         │
               ┌─────────┼─────────┐
               ▼         ▼         ▼
          Source Domain  ERP  Reconciliation
```

---

# 170. Final Decision

Baobab SHALL permanently distinguish:

```text id="xxfj04"
COLLECTION
     ≠
PAYOUT

PAYMENT
     ≠
PAYOUT

REFUND
     ≠
PAYOUT

SETTLEMENT
     ≠
PAYOUT

BENEFICIARY
     ≠
DESTINATION

SUPPLIER
     ≠
PAYOUT

SUPPLIER INVOICE
     ≠
PAYOUT

PAYOUT APPROVAL
     ≠
PAYOUT EXECUTION

PAYOUT SUBMISSION
     ≠
PAYOUT SUCCESS

PAYOUT SUCCESS
     ≠
ERP RECONCILIATION

PAYOUT RETURN
     ≠
REFUND

PROVIDER PAYOUT ID
     ≠
BAOBAB PAYOUT ID
```

The authoritative model is:

```text id="dt7yy5"
BUSINESS DOMAIN
determines why money is owed
        │
        ▼
CANONICAL BUSINESS OBLIGATION
        │
        ▼
BAOBAB PAYMENTS
validates payer + beneficiary + destination
        │
        ▼
COMPLIANCE / POLICY
establishes eligibility where required
        │
        ▼
APPROVAL
authorises outbound movement
        │
        ▼
PAYMENTS
determines eligible execution routes
        │
        ▼
HYPERSWITCH / PROVIDER
executes the outbound financial operation
        │
        ▼
PAYMENTS
records what actually happened
        │
        ├──────────► originating business domain
        ├──────────► reconciliation
        └──────────► ERP
                       │
                       ▼
              accounting consequence
```

**The originating business domain determines why money is owed.  
The canonical context determines which Legal Entity may pay it.  
Beneficiary governance determines who may receive it.  
Destination governance determines where it may be sent.  
Approval determines whether outbound movement is authorised.  
Baobab Payments determines how that authorised Payout may execute.  
HyperSwitch and the provider execute only within the authorised envelope.  
Payments records what happened.  
ERP determines the accounting consequence.**

**The ability to collect money never grants the authority to disburse it. The ability to pay one beneficiary never grants authority to pay another. And no timeout, corporate relationship, provider configuration, available balance, or successful collection may substitute for explicit outbound-payment authority.**