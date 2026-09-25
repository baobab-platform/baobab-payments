# ADR-PAY-0017 — FX & Cross-Border Payment Semantics

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / FX / Cross-Border Payments |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0016; ADR-PAY-0022; ADR-SHARED-011; applicable Control Plane, ERP, IAM and Shared contracts |
| **Related** | ADR-PAY-0018 through ADR-PAY-0021 |
| **Primary Domains** | Currency, FX, cross-border collection, settlement, refunds, payouts, reconciliation |

---

# 1. Context

Baobab is inherently multi-market.

Initial markets include:

```text
Uganda
South Africa
```

with architecture intended to support additional markets such as:

```text
Kenya
Tanzania
Rwanda
```

and external Baobab customers operating across other jurisdictions.

Consequently, a financial transaction may involve several currencies and jurisdictions simultaneously.

For example:

```text
Customer
   │
   │ pays UGX
   ▼
ZuriBeans
   │
   │ provider converts
   ▼
Settlement account
   │
   │ receives USD
   ▼
ERP
```

A supplier Payout may similarly involve:

```text
Source obligation: USD
Payout funding: ZAR
Beneficiary receives: UGX
```

These currencies cannot safely be represented by one generic `currency` field.

---

# 2. Problem

A naïve implementation might assume:

```text
Market
   │
   ▼
Currency
```

or:

```text
Payment currency
      =
Settlement currency
```

or:

```text
Provider returned different amount
      =
Provider fee
```

or:

```text
Cross-border
      =
different currencies
```

All are unsafe assumptions.

A transaction may be cross-border while remaining in one currency.

A transaction may involve FX while remaining within one country.

A provider may perform currency conversion at:

```text
authorisation
capture
settlement
payout
```

and the resulting rate, spread, fee and risk may belong to different parties.

Baobab therefore requires explicit FX and cross-border semantics.

---

# 3. Decision

Baobab SHALL represent:

```text
currency context
FX conversion
FX quote
FX rate
FX provenance
cross-border context
settlement currency
beneficiary currency
```

as explicit financial concepts.

Currency conversion SHALL NOT be treated as an invisible implementation detail.

---

# 4. Governing Principle

> **Currency describes monetary denomination. FX describes an authorised transformation between monetary denominations. Cross-border describes jurisdictional context. None of these concepts is interchangeable.**

---

# 5. Fundamental Non-Equivalences

Baobab SHALL preserve:

```text
Market
    ≠
Currency

Currency
    ≠
Country

FX
    ≠
Cross-Border

Cross-Border
    ≠
FX

Transaction Currency
    ≠ necessarily
Settlement Currency

Settlement Currency
    ≠ necessarily
Bank Account Currency

Payout Currency
    ≠ necessarily
Beneficiary Currency

FX Spread
    ≠
Provider Fee

FX Quote
    ≠
FX Conversion

FX Rate
    ≠
Accounting Exchange Rate

Provider FX Rate
    ≠
ERP Reporting Rate
```

---

# 6. Currency Roles

Baobab SHALL distinguish monetary roles rather than relying on a generic currency field.

Relevant roles MAY include:

| Currency role | Meaning |
|---|---|
| Pricing Currency | Currency used to express commercial price |
| Transaction Currency | Currency of the authorised customer/business obligation |
| Presentment Currency | Currency presented to the payer where distinct |
| Processing Currency | Currency used by a processor internally where relevant |
| Capture Currency | Currency in which provider capture occurs where distinct |
| Settlement Currency | Currency in which provider settles the merchant |
| Funding Currency | Currency from which an outbound Payout is funded |
| Payout Currency | Currency of the authorised outbound instruction |
| Beneficiary Currency | Currency delivered to the beneficiary |
| Refund Currency | Currency of the Refund obligation/execution |
| Accounting Currency | Currency used for a particular ERP accounting record |
| Functional Currency | Legal Entity accounting functional currency |
| Reporting Currency | Currency used for group/reporting purposes |

Not every transaction uses every role.

---

# 7. Commercial Authority

The originating commercial domain owns the authoritative monetary obligation.

Examples:

```text
Trade
   → order amount + transaction currency

Subscriptions
   → billing obligation + transaction currency

ERP / procurement
   → supplier obligation + payout currency
```

Payments SHALL NOT silently re-price these obligations.

---

# 8. Transaction Currency

Transaction Currency SHALL represent the denomination of the authorised financial obligation handed to Payments.

Once external execution begins, it SHALL remain historically immutable.

---

# 9. Market Does Not Determine Currency Alone

A Market MAY permit:

```text
one currency
multiple currencies
```

depending on business configuration.

Therefore:

```text
ZA Market
```

does not architecturally mean:

```text
currency = ZAR
```

even if ZAR is the primary configured currency.

---

# 10. No Country-to-Currency Hardcoding

Leaf applications SHALL NOT contain assumptions such as:

```text
if market == "UG":
    currency = "UGX"
```

as payment authority.

Market/currency eligibility SHALL come from governed configuration.

---

# 11. ISO Currency Representation

Canonical currency identifiers SHOULD use ISO 4217-compatible codes where applicable.

Non-standard monetary assets, should Baobab ever support them, require explicit future architecture rather than overloading fiat-currency semantics.

---

# 12. Monetary Precision

Baobab SHALL NOT assume:

```text
all currencies = 2 decimal places
```

Currency metadata determines applicable precision/minor-unit semantics.

---

# 13. Exact Arithmetic

Authoritative monetary calculations SHALL use exact decimal or integer-minor-unit semantics.

Binary floating point SHALL NOT be used for authoritative FX or monetary calculations.

---

# 14. FX Conversion

An FX Conversion represents an actual or contractually authoritative transformation:

```text
Money A
   │
   │ FX
   ▼
Money B
```

It SHALL have explicit provenance.

---

# 15. Conceptual FX Conversion Model

```text
FXConversion
│
├── fx_conversion_id
├── source_amount
├── source_currency
├── destination_amount
├── destination_currency
├── rate
├── rate_type
├── rate_source
├── quote_reference?
├── provider_reference?
├── spread?
├── explicit_fee?
├── quoted_at?
├── expires_at?
├── executed_at?
├── legal_entity_id
├── market_id?
├── source_transaction_type
├── source_transaction_id
├── correlation_id
└── simulated
```

Exact canonical representation SHALL evolve through Shared contracts.

---

# 16. FX Identity

Where an FX Conversion is material to financial reconstruction, it SHOULD have its own Baobab identity.

Provider FX references remain external references.

---

# 17. FX Quote

An FX Quote represents proposed conversion terms.

Conceptually:

```text
FXQuote
│
├── quote_id
├── source_currency
├── destination_currency
├── source_amount?
├── destination_amount?
├── rate
├── rate_source
├── spread?
├── fee?
├── quoted_at
├── expires_at?
├── provider?
└── constraints
```

---

# 18. Quote Is Not Conversion

```text
FX Quote
    ≠
FX Conversion
```

A quote does not prove conversion occurred.

---

# 19. Quote Expiry

An expired quote SHALL NOT silently be used for a new financial execution.

---

# 20. Quote Binding

Where provider semantics permit, Baobab SHOULD preserve the relationship:

```text
FX Quote
    │
    ▼
FX Conversion
```

to explain which rate governed the transaction.

---

# 21. FX Rate

An FX rate SHALL identify its direction unambiguously.

For example:

```text
1 USD = X UGX
```

is not interchangeable with:

```text
1 UGX = Y USD
```

without explicit inversion.

---

# 22. No Ambiguous Rate Field

A numeric value such as:

```text
3700
```

SHALL NOT be considered a complete FX rate without:

```text
base/source currency
quote/destination currency
rate convention
timestamp
source
```

---

# 23. Rate Provenance

Every financially material FX rate SHOULD retain:

```text
source
timestamp
currency pair
rate
quote/provider reference
```

and, where applicable:

```text
spread
fee
expiry
```

---

# 24. Rate Sources

Rate sources MAY include:

```text
payment provider
bank
FX provider
treasury provider
authorised internal rate service
```

depending on the transaction.

The source SHALL be explicit.

---

# 25. Market Reference Rates

Market/reference rates MAY be useful for:

```text
analytics
comparison
reconciliation
variance detection
```

but SHALL NOT automatically replace the actual transactional rate.

---

# 26. Accounting Rate

ERP MAY apply an accounting exchange rate that differs from the payment provider's execution rate.

Therefore:

```text
Provider FX Rate
      ≠
ERP Accounting Rate
```

is valid.

---

# 27. Payments Does Not Own Accounting FX Policy

Payments records operational FX facts.

ERP determines:

```text
functional-currency conversion
reporting-currency conversion
realised FX gain/loss
unrealised FX gain/loss
accounting-period rate
```

according to accounting policy.

---

# 28. FX Spread

Where available, FX spread SHALL remain conceptually distinguishable from:

```text
explicit processing fee
network fee
payout fee
```

---

# 29. Unknown Spread

If the provider embeds FX economics in its rate and does not expose the spread, Baobab SHALL NOT fabricate one.

---

# 30. Explicit Fee

An explicit FX fee SHOULD be represented separately where provider evidence permits.

---

# 31. No Double Counting

Baobab SHALL avoid counting:

```text
FX spread
+
same economic cost represented again as fee
```

without evidence that they are genuinely distinct.

---

# 32. Who Performs FX?

FX may be performed by:

```text
merchant
payment provider
acquirer
bank
payout provider
beneficiary bank
external FX provider
```

Baobab SHALL preserve the known conversion actor where material.

---

# 33. Provider FX

If the payment provider performs FX:

```text
Transaction Currency
        │
        ▼
Provider Conversion
        │
        ▼
Settlement Currency
```

Payments SHALL preserve the provider FX evidence received.

---

# 34. Merchant-Controlled FX

If Baobab or a Legal Entity obtains an FX quote before execution, the selected quote/rate SHALL become part of the authorised transaction context.

---

# 35. Customer Choice

If a payer chooses between supported presentment currencies, the commercial system SHALL establish the resulting authoritative obligation before Payments executes it.

Payments SHALL not independently change the commercial price.

---

# 36. Dynamic Currency Conversion

Provider/acquirer dynamic currency conversion, if ever supported, SHALL require explicit architecture and customer-consent semantics.

It SHALL NOT be silently enabled merely because a connector supports it.

---

# 37. Cross-Border Context

Cross-border status SHALL not be inferred solely from currency difference.

Relevant dimensions MAY include:

```text
payer jurisdiction
merchant Legal Entity jurisdiction
canonical Market
provider processing jurisdiction
acquirer jurisdiction
settlement-account jurisdiction
beneficiary jurisdiction
funding jurisdiction
```

---

# 38. Cross-Border Is Multi-Dimensional

Conceptually:

```text
CrossBorderContext
│
├── payer_country?
├── legal_entity_country
├── market_id
├── processing_country?
├── settlement_country?
├── beneficiary_country?
├── funding_country?
└── classification / evidence
```

Exact contract shape may evolve.

---

# 39. Same Currency Can Be Cross-Border

For example:

```text
Country A
   │
   │ USD
   ▼
Country B
```

may be cross-border despite no FX conversion.

---

# 40. Different Currency Can Be Domestic

A jurisdiction may lawfully support multiple transaction currencies.

Therefore currency difference alone does not establish cross-border status.

---

# 41. Cross-Border Classification

Where cross-border classification affects:

```text
provider eligibility
fees
compliance
tax
reporting
routing
settlement
```

it SHALL derive from authoritative context and governed policy.

---

# 42. Customer IP Is Not Authority

IP geolocation MAY be used as:

```text
risk signal
UX default
fraud signal
```

but SHALL NOT independently establish authoritative payment jurisdiction.

---

# 43. Browser Locale Is Not Authority

Likewise:

```text
browser language
device timezone
locale
```

do not establish financial jurisdiction.

---

# 44. Market Remains Canonical

Canonical Market continues to come from Baobab Control Plane/business context.

Provider country configuration SHALL not redefine it.

---

# 45. Legal Entity Remains Canonical

Provider merchant country or settlement account location SHALL not redefine the canonical Legal Entity.

---

# 46. Collection FX

A collection may follow:

```text
Commercial obligation
UGX 500,000
      │
      ▼
Payment
UGX 500,000
      │
      ▼
Provider FX
      │
      ▼
Settlement
USD 135.xx
```

The Payment remains UGX 500,000.

---

# 47. Settlement FX

PAY-0015 SHALL preserve:

```text
transaction amount
transaction currency
settlement amount
settlement currency
provider FX evidence
fees
```

where available.

---

# 48. Settlement Difference Is Not Automatically FX

A difference between captured and settled amounts may result from:

```text
FX
fees
refunds
chargebacks
reserves
adjustments
```

Reconciliation SHALL classify using evidence.

---

# 49. Refund Currency

A Refund SHALL preserve the currency semantics established by PAY-0013.

Payments SHALL NOT arbitrarily choose a new refund currency for operational convenience.

---

# 50. Refund FX

Where a Refund requires conversion, Baobab SHALL preserve the actual refund FX terms separately from the original payment FX.

---

# 51. Original FX Rate Need Not Apply to Refund

The rate applicable to the original Payment MAY differ from the rate at Refund time.

Baobab SHALL not assume:

```text
refund FX rate
=
original payment FX rate
```

unless provider/contract semantics explicitly require it.

---

# 52. Refund FX Difference

Any resulting economic difference SHALL be represented and reconciled rather than rewriting the original Payment.

---

# 53. Chargeback FX

Chargebacks MAY be reported or settled in a currency differing from the original transaction or merchant settlement currency.

PAY-0014 and PAY-0015 SHALL preserve these financial facts.

---

# 54. Payout FX

PAY-0016 Payouts MAY involve:

```text
Funding Currency
       │
       ▼
FX Conversion
       │
       ▼
Beneficiary Currency
```

---

# 55. Payout Obligation Currency

The business obligation currency SHALL remain distinguishable from the payout execution/destination currency.

---

# 56. Beneficiary Currency

Beneficiary Currency SHALL be explicit where it differs from the Payout Currency.

---

# 57. Beneficiary Amount

Where FX occurs before beneficiary receipt, the authorised model SHALL establish whether:

```text
source amount is fixed
```

or:

```text
beneficiary amount is fixed
```

before execution.

---

# 58. Fixed Source Amount

Example:

```text
Pay ZAR 10,000 equivalent in UGX
```

means the source amount is fixed and beneficiary amount varies with the authorised FX terms.

---

# 59. Fixed Destination Amount

Example:

```text
Beneficiary must receive UGX 2,000,000
```

means the destination amount is fixed and required funding amount may vary.

These are materially different instructions.

---

# 60. No Ambiguous Payout FX Instruction

A cross-currency Payout SHALL establish which side is authoritative.

---

# 61. Payout Fee Allocation

Payout FX SHALL also establish, where relevant, whether fees are:

```text
paid by sender
deducted from beneficiary proceeds
embedded in rate
separately charged
```

---

# 62. Correspondent / Intermediary Fees

Where bank or intermediary deductions cannot be known in advance, Baobab SHALL not falsely guarantee a beneficiary amount.

The uncertainty SHOULD be represented where material.

---

# 63. FX Eligibility

FX capability SHALL itself be governed.

A provider supporting:

```text
ZAR collections
and
UGX collections
```

does not necessarily support:

```text
ZAR → UGX conversion
```

---

# 64. PAY-0022 Certification

Provider certification SHALL capture applicable:

```text
source currencies
destination currencies
currency pairs
markets
Legal Entities
transaction types
FX capabilities
settlement currencies
payout currencies
```

---

# 65. FX Pair Certification

A provider certified for:

```text
USD → ZAR
```

is not automatically certified for:

```text
USD → UGX
```

---

# 66. Direction Matters

Certification for:

```text
USD → UGX
```

does not necessarily imply:

```text
UGX → USD
```

---

# 67. Provider Activation

FX capability SHALL be activated per relevant Legal Entity and business context.

---

# 68. Group Membership

Nabhold Group ownership does not permit one subsidiary to inherit another subsidiary's FX/provider arrangement.

---

# 69. External Tenants

External Baobab customers receive identical FX isolation.

---

# 70. Routing

PAY-0009 applies.

FX-capable routing occurs only among providers already eligible for:

```text
Legal Entity
Market
source currency
destination currency
transaction type
payment/payout method
FX capability
environment
```

---

# 71. No Routing-Induced Currency Change

A routing engine SHALL NOT alter the authorised currency merely to access another provider.

---

# 72. Failover

Failover SHALL preserve authorised monetary semantics.

A fallback provider cannot silently change:

```text
transaction currency
beneficiary currency
fixed-source/fixed-destination semantics
FX quote
```

---

# 73. Quote-Bound Failover

If execution is bound to a provider-specific FX quote, failover to another provider normally requires:

```text
new quote
new validation
and possibly
new customer/business approval
```

rather than silent substitution.

---

# 74. Token Portability

Provider-bound token restrictions under PAY-0009 remain applicable and may further constrain cross-currency failover.

---

# 75. Idempotency

PAY-0010 applies to FX operations.

A repeated conversion command SHALL not accidentally execute another conversion.

---

# 76. FX Idempotency Fingerprint

Material fields SHOULD include:

```text
source amount
source currency
destination amount where fixed
destination currency
quote reference where applicable
Legal Entity
transaction reference
```

---

# 77. Quote Refresh Is Not Retry

Obtaining a new FX quote is a new economic decision.

It SHALL not be hidden as a technical retry of an expired quote.

---

# 78. Unknown FX Outcome

If a conversion may have executed but its outcome is unknown:

```text
UNKNOWN
```

SHALL be recovered before another potentially duplicative conversion.

---

# 79. Concurrency

Concurrent FX execution against the same authorised operation SHALL be prevented using PAY-0010 concurrency and idempotency controls.

---

# 80. FX Events

Representative canonical facts MAY include:

```text
fx.quote.created
fx.quote.expired
fx.conversion.requested
fx.conversion.completed
fx.conversion.failed
```

if dedicated FX events are justified.

Exact event contracts belong in Shared.

---

# 81. Avoid Event Proliferation

If FX is merely an attribute of another canonical financial event and no independent lifecycle exists, Baobab SHOULD avoid unnecessary standalone event types.

---

# 82. Payment Events

Where FX materially affects a Payment, canonical payment facts MAY reference the applicable FX Conversion without embedding provider-specific payloads.

---

# 83. Payout Events

Likewise, Payout events MAY reference:

```text
fx_conversion_id
```

or equivalent canonical FX evidence.

---

# 84. Settlement Events

Settlement events SHALL preserve settlement currency and relevant FX reference/evidence.

---

# 85. Sensitive Data

FX information SHALL follow PAY-0012 where it contains:

```text
banking information
counterparty information
provider secrets
sensitive commercial rates
```

---

# 86. Commercial Sensitivity

FX rates and spreads may be commercially sensitive even where they are not PCI data.

Access SHALL therefore be appropriately controlled.

---

# 87. API Contract

Canonical APIs SHOULD avoid an ambiguous request such as:

```text
{
  "currency": "USD"
}
```

when multiple currency roles are possible.

Contracts SHOULD name the semantic role.

---

# 88. Example Contract Semantics

Conceptually:

```text
transaction_money:
  amount
  currency

settlement_money:
  amount
  currency

fx:
  source_money
  destination_money
  rate
  rate_source
```

Exact schema belongs in Shared.

---

# 89. Currency Pair

A currency pair SHALL be represented unambiguously:

```text
source_currency
destination_currency
```

rather than an opaque string where direction could be misunderstood.

---

# 90. Monetary Amount

Conceptually:

```text
Money
├── amount
└── currency
```

SHALL remain the basic canonical monetary concept.

---

# 91. No Amount Without Currency

Financial APIs/events/storage SHALL not represent authoritative monetary amounts without associated currency semantics.

---

# 92. No Currency Without Role Where Ambiguous

Where an object contains multiple monetary roles, generic:

```text
currency
```

SHALL be avoided.

---

# 93. Historical Integrity

Historical FX facts SHALL not change when:

```text
current exchange rate changes
provider changes
market configuration changes
Legal Entity changes provider
```

---

# 94. No Revaluation in Payments

Payments SHALL not periodically rewrite historical transactions using current FX rates.

---

# 95. ERP Revaluation

ERP owns any required accounting revaluation.

---

# 96. Rate Correction

If provider later corrects FX evidence, Baobab SHALL preserve:

```text
original evidence
corrected evidence
supersession/correction relationship
```

rather than silently rewriting history.

---

# 97. Audit

FX audit SHOULD preserve:

```text
source amount
source currency
destination amount
destination currency
rate
rate convention
rate source
quote reference
provider
Legal Entity
Market
transaction reference
quoted_at
executed_at
fees/spread where known
operator/workload where relevant
correlation
```

---

# 98. Explainability

For a converted financial transaction, an authorised operator SHOULD be able to answer:

```text
What amount entered FX?

In which currency?

What amount emerged?

In which currency?

Which rate was used?

Who supplied the rate?

When?

Which provider performed the conversion?

Which transaction required it?

Which Legal Entity owned it?
```

---

# 99. Observability

FX telemetry SHOULD allow correlation across:

```text
PaymentIntent
Payment
Payout
Refund
Settlement
FX Quote
FX Conversion
Provider
ERP projection
```

without leaking sensitive data.

---

# 100. Metrics

Representative metrics MAY include:

```text
fx_quote_total
fx_quote_expired_total
fx_conversion_total
fx_conversion_failed_total
fx_conversion_amount
fx_conversion_latency
fx_rate_variance
fx_provider_spread
cross_border_payment_total
cross_border_payout_total
settlement_currency_mismatch_total
fx_reconciliation_exception_total
```

Exact names remain implementation-specific.

---

# 101. Rate Variance Monitoring

Baobab MAY compare executed provider rates against an authorised benchmark for:

```text
anomaly detection
commercial monitoring
reconciliation
```

without substituting that benchmark for the actual executed rate.

---

# 102. Alerts

Potential alerts include:

```text
unexpected currency conversion
unsupported currency pair
large FX variance
expired quote execution attempt
settlement currency mismatch
unexpected beneficiary currency
provider FX capability drift
missing FX evidence
cross-border configuration mismatch
```

---

# 103. Reconciliation

PAY-0015 SHALL reconcile FX-related differences using explicit FX facts.

---

# 104. No Residual-Bucket FX

Reconciliation SHALL NOT classify every unexplained difference as:

```text
FX variance
```

merely to force a match.

---

# 105. ERP Projection

Payments SHALL project actual operational FX evidence to ERP where relevant.

ERP remains authoritative for accounting interpretation.

---

# 106. Accounting Consequences

ERP determines:

```text
realised FX gain/loss
unrealised FX gain/loss
functional-currency value
reporting-currency value
period treatment
```

---

# 107. Cross-Border Fees

Cross-border provider fees SHALL remain distinct from FX conversion unless provider evidence makes them inseparable.

---

# 108. Tax and Regulatory Treatment

Payments SHALL preserve relevant financial facts.

It SHALL NOT become the tax or regulatory determination engine.

---

# 109. Compliance Boundary

Cross-border transactions MAY require additional compliance controls.

Payments SHALL consume/enforce authoritative eligibility outcomes where required rather than inventing legal determinations.

---

# 110. Unknown Required Cross-Border Eligibility

Where a required compliance/market/provider determination is unknown:

```text
UNKNOWN
```

SHALL fail closed for new execution.

---

# 111. Data Residency

Cross-border payment execution does not automatically authorise cross-border movement of unrelated personal or sensitive data.

Data-residency rules remain governed independently.

---

# 112. Cross-Border Money ≠ Cross-Border Data

Baobab SHALL preserve:

```text
cross-border financial transaction
        ≠
permission for unrestricted cross-border data transfer
```

---

# 113. Multi-Region Architecture

FX and cross-border context SHALL survive multi-region routing.

A failover region SHALL not recompute business context from local defaults.

---

# 114. Engine Instance

The resolved Payments Engine Instance remains part of authoritative execution context.

FX routing across instances must preserve canonical Tenant and Legal Entity isolation.

---

# 115. Sandbox

Simulated FX SHALL carry:

```text
simulated = true
```

---

# 116. Sandbox Rate

A sandbox/simulated rate SHALL never be interpreted as an actual financial-market rate.

---

# 117. Test Data

Development/test environments SHALL use synthetic or provider-approved test FX data.

---

# 118. Production FX Activation

Production FX SHALL not be enabled merely because:

```text
provider API accepts two currencies
```

Certification and activation remain required.

---

# 119. Production Readiness

A cross-currency route SHALL require at least:

```text
Legal Entity context
Market context
source currency eligibility
destination currency eligibility
currency-pair eligibility
provider certification
provider activation
FX capability
quote/rate semantics
idempotency
reconciliation
ERP projection
audit
observability
```

---

# 120. Collection Example — No FX

```text
ZuriBeans ZA
     │
     ▼
Customer pays
ZAR 10,000
     │
     ▼
Provider processes
ZAR 10,000
     │
     ▼
Settlement
ZAR 9,700

Difference:
ZAR 300 provider fees

No FX occurred.
```

---

# 121. Collection Example — FX at Settlement

```text
Customer
     │
     ▼
Payment
UGX 1,000,000
     │
     ▼
Provider
     │
     ▼
FX Conversion
UGX → USD
     │
     ▼
Settlement
USD X
```

Canonical Payment remains:

```text
UGX 1,000,000
```

---

# 122. Cross-Border Without FX

```text
Payer jurisdiction A
        │
        │ USD
        ▼
Merchant jurisdiction B
        │
        ▼
USD settlement
```

This may be cross-border despite no currency conversion.

---

# 123. FX Without Cross-Border

```text
Domestic market
     │
     ▼
Customer chooses supported Currency A
     │
     ▼
Domestic merchant settles Currency B
```

FX may occur without necessarily making the transaction cross-border.

---

# 124. Payout Example — Fixed Destination

```text
Supplier must receive:
UGX 5,000,000
        │
        ▼
Obtain FX quote
        │
        ▼
Required funding:
USD X
        │
        ▼
Execute Payout
        │
        ▼
Supplier receives:
UGX 5,000,000
```

The beneficiary amount is authoritative.

---

# 125. Payout Example — Fixed Source

```text
Approved spend:
USD 1,000
      │
      ▼
Obtain FX quote
      │
      ▼
Beneficiary receives:
UGX X
```

The source amount is authoritative.

---

# 126. Refund Example

```text
Original Payment
USD 100
   │
   ▼
Provider settlement conversion
ZAR
   │
   ▼
Later Refund
USD 100
   │
   ▼
Provider applies current refund FX
ZAR Y
```

The original and refund FX facts remain distinct.

---

# 127. Settlement Example

```text
Payments
   │
   ├── UGX Payment P1
   ├── UGX Payment P2
   └── UGX Refund R1
           │
           ▼
       Provider FX
           │
           ▼
      USD Settlement
           │
           ▼
       Reconciliation
           │
           ▼
           ERP
```

ERP receives operational FX evidence but owns accounting treatment.

---

# 128. Cross-Border Context Flow

```text
Canonical Tenant
      │
      ▼
Organisation
      │
      ▼
Legal Entity
      │
      ▼
Market
      │
      ▼
Transaction Currency
      │
      ▼
Cross-Border Context
      │
      ▼
FX Required?
   ┌──┴───┐
   │      │
  NO     YES
   │      │
   │      ▼
   │   FX Eligibility
   │      │
   │      ▼
   │    Quote / Rate
   │      │
   └──────┤
          ▼
Provider Eligibility
          │
          ▼
Routing
          │
          ▼
Execution
```

---

# 129. FX Execution Flow

```text
Authorised Monetary Obligation
            │
            ▼
      Source Currency
            │
            ▼
      FX Required?
            │
           YES
            │
            ▼
   Resolve Eligible FX Routes
            │
            ▼
       Obtain Quote
            │
            ▼
       Validate Quote
            │
            ▼
    Authorise Economic Terms
            │
            ▼
      Execute Conversion
            │
            ▼
     Destination Currency
            │
            ▼
Payment / Settlement / Payout
            │
            ▼
       Reconciliation
            │
            ▼
            ERP
```

---

# 130. Currency Model

```text
                    MONEY
                      │
       ┌──────────────┼───────────────┐
       ▼              ▼               ▼
 Transaction      Settlement        Payout
   Money             Money           Money
       │              │               │
       │              │               ▼
       │              │         Beneficiary Money
       │              │
       └───────┬──────┘
               ▼
        FX Conversion
               │
       ┌───────┴────────┐
       ▼                ▼
 Source Money     Destination Money
```

---

# 131. Cross-Border Model

```text
                    TRANSACTION
                         │
       ┌─────────────────┼─────────────────┐
       ▼                 ▼                 ▼
     Payer          Legal Entity         Market
 Jurisdiction       Jurisdiction
       │                 │                 │
       └─────────────────┼─────────────────┘
                         ▼
                 Cross-Border Context
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
      Provider       Settlement      Beneficiary
    Jurisdiction    Jurisdiction    Jurisdiction
```

No single country field replaces this model.

---

# 132. Production Readiness Gate

```text
FX & CROSS-BORDER READINESS

Explicit currency roles                         PASS
Canonical Money semantics                       PASS
Currency precision                              PASS
Exact arithmetic                                PASS
FX Conversion model                             PASS
FX Quote model                                  PASS
Rate direction                                  PASS
Rate provenance                                 PASS
Quote expiry                                    PASS
Quote binding                                   PASS
Spread/fee distinction                          PASS
Provider FX mapping                             PASS
Settlement FX                                   PASS
Refund FX                                       PASS
Payout FX                                       PASS
Fixed-source semantics                          PASS
Fixed-destination semantics                     PASS
Cross-border context                            PASS
Market independence                             PASS
Legal Entity independence                       PASS
Provider certification                          PASS
Currency-pair activation                        PASS
Routing eligibility                             PASS
Failover semantics                              PASS
Idempotency                                     PASS
Unknown-outcome recovery                        PASS
Historical integrity                            PASS
Reconciliation                                  PASS
ERP projection                                  PASS
Sensitive-data controls                         PASS
Audit                                           PASS
Observability                                   PASS
Sandbox/prod separation                         PASS
-----------------------------------------------------
FX & Cross-Border Plane                         READY
```

---

# 133. Invariants

The following invariants SHALL hold:

1. Market is not Currency.
2. Currency is not Country.
3. FX is not Cross-Border.
4. Cross-Border is not FX.
5. Same-currency transactions may be cross-border.
6. Multi-currency transactions may be domestic.
7. Transaction Currency is explicit.
8. Settlement Currency is explicit.
9. Payout Currency is explicit.
10. Beneficiary Currency is explicit where distinct.
11. Funding Currency is explicit where relevant.
12. Generic currency fields are avoided where multiple roles exist.
13. Monetary amounts always have currency semantics.
14. Currency precision is not universally two decimals.
15. Binary floating point is not authoritative monetary arithmetic.
16. FX Conversion is explicit.
17. FX Quote is not FX Conversion.
18. Expired quotes are not silently reused.
19. Rate direction is explicit.
20. Rate source is explicit.
21. Rate timestamp is retained where material.
22. Provider rate does not become ERP accounting rate.
23. Payments does not own accounting FX policy.
24. Payments does not perform accounting revaluation.
25. Historical FX facts are immutable.
26. Provider correction does not silently erase original evidence.
27. FX spread is not automatically a provider fee.
28. Unknown spread is not fabricated.
29. Explicit FX fees are represented separately where available.
30. Double counting spread and fees is prohibited.
31. Market does not automatically determine one currency.
32. Country-to-currency hardcoding is not authoritative.
33. Customer IP does not establish financial jurisdiction.
34. Browser locale does not establish financial jurisdiction.
35. Provider country does not redefine canonical Market.
36. Provider merchant location does not redefine Legal Entity.
37. Settlement difference is not automatically FX.
38. Refund FX does not rewrite original Payment FX.
39. Refund FX rate is not assumed equal to original rate.
40. Chargeback FX remains separately reconstructable where evidence exists.
41. Payout FX distinguishes source and beneficiary money.
42. Cross-currency Payout identifies which side is fixed.
43. Payout fee allocation is explicit.
44. Unknown intermediary fees are not disguised as guaranteed beneficiary amounts.
45. Provider support for currencies does not imply FX support.
46. Currency-pair certification is explicit.
47. Currency-pair direction matters.
48. FX activation is Legal-Entity scoped.
49. Subsidiaries do not inherit FX arrangements implicitly.
50. External tenants remain isolated.
51. Routing cannot alter authorised currency.
52. Failover cannot silently change FX economics.
53. Quote-bound failover may require a new quote/approval.
54. FX operations are idempotent.
55. Quote refresh is not a technical retry.
56. UNKNOWN conversion outcome is not failure.
57. Duplicate conversion is prevented.
58. Canonical events do not expose provider-specific FX payloads unnecessarily.
59. FX commercial data receives appropriate access control.
60. Reconciliation uses explicit FX evidence.
61. Unexplained differences are not automatically classified as FX.
62. ERP owns accounting consequences.
63. Cross-border compliance state fails closed where required and unknown.
64. Cross-border money movement does not authorise unrestricted data movement.
65. Multi-region failover preserves canonical context.
66. Sandbox FX is explicitly simulated.
67. Sandbox rates are not production financial rates.
68. Production FX requires provider certification and activation.
69. Current market rates never rewrite historical transactions.
70. No convenience, routing optimisation or provider limitation may silently change the authorised monetary obligation.

---

# 134. Consequences

## Positive

This decision provides:

- explicit multi-currency semantics;
- provider-independent FX modelling;
- cross-border clarity;
- auditable exchange rates;
- settlement reconciliation;
- payout FX safety;
- refund FX correctness;
- multi-market extensibility;
- Legal Entity isolation;
- accounting separation;
- future treasury integration;
- provider portability.

## Negative

It introduces additional complexity in:

- canonical contracts;
- provider adapters;
- quote management;
- rate provenance;
- reconciliation;
- payout processing;
- settlement ingestion;
- accounting integration;
- compliance integration;
- testing.

These costs are accepted because implicit currency conversion creates unacceptable financial ambiguity.

---

# 135. Alternatives Considered

## Use One Currency Field Everywhere

Rejected.

Different financial stages may use different currencies.

---

## Infer Currency from Market

Rejected.

Markets may support multiple currencies.

---

## Let Provider Silently Convert

Rejected.

Conversion changes financial economics and requires explicit evidence.

---

## Treat Settlement Difference as FX

Rejected.

Differences may arise from fees, refunds, reserves, chargebacks or adjustments.

---

## Use Current FX Rate for Historical Transactions

Rejected.

It destroys historical financial truth.

---

## Use Provider Rate as ERP Rate

Rejected.

Operational FX and accounting FX have different authorities.

---

## Treat Cross-Border as Currency Difference

Rejected.

Jurisdiction and currency are separate dimensions.

---

## Let HyperSwitch Define Canonical FX Semantics

Rejected.

HyperSwitch is implementation infrastructure, not Baobab's canonical financial contract.

---

## Hardcode UGX for Uganda and ZAR for South Africa

Rejected.

It prevents legitimate multi-currency configuration and future expansion.

---

# 136. Relationship to PAY-0005

PAY-0005 establishes:

```text
Market
Currency
Payment Method
Transaction Type
```

as separate payment-context dimensions.

PAY-0017 extends Currency into explicit multi-stage and FX semantics without changing Market authority.

---

# 137. Relationship to PAY-0008

PAY-0008 remains authoritative for Payment state.

FX lifecycle SHALL not be disguised as Payment state.

---

# 138. Relationship to PAY-0009

PAY-0009 determines which provider routes are eligible.

PAY-0017 adds:

```text
source currency
destination currency
currency pair
FX capability
quote constraints
```

to the execution envelope where FX occurs.

---

# 139. Relationship to PAY-0010

PAY-0010 ensures FX retries cannot accidentally create duplicate conversions or financial effects.

---

# 140. Relationship to PAY-0011

FX facts propagated through canonical events follow PAY-0011:

```text
validated state
transactional outbox
stable event identity
at-least-once delivery
idempotent consumption
```

---

# 141. Relationship to PAY-0012

FX contracts, logs and events SHALL not leak sensitive payment, banking or provider credential data.

Commercially sensitive rates/spreads also require controlled access.

---

# 142. Relationship to PAY-0013

Refunds preserve their own currency and FX facts.

Original Payment FX is historical evidence, not a mutable rate for later Refunds.

---

# 143. Relationship to PAY-0014

Disputes and chargebacks may create FX effects distinct from the original Payment.

Those effects remain linked to the dispute lifecycle.

---

# 144. Relationship to PAY-0015

PAY-0015 consumes actual FX evidence during settlement reconciliation.

It SHALL not infer FX merely from unmatched settlement differences.

---

# 145. Relationship to PAY-0016

PAY-0016 defines outbound-money authority.

PAY-0017 defines how authorised cross-currency Payouts preserve:

```text
source money
destination money
rate
fees
beneficiary amount
FX provenance
```

---

# 146. Relationship to PAY-0018

PAY-0018 SHALL govern who may:

```text
configure FX providers
activate currency pairs
approve privileged FX overrides
request sensitive rate information
perform administrative correction
```

---

# 147. Relationship to PAY-0019

PAY-0019 SHALL establish operational monitoring for:

```text
FX provider availability
quote latency
conversion failures
rate anomalies
cross-border failures
settlement mismatches
FX reconciliation backlog
```

---

# 148. Relationship to PAY-0020

Disaster recovery SHALL preserve:

```text
FX Quotes where still relevant
FX Conversions
idempotency records
provider references
rate provenance
settlement FX evidence
```

without re-executing already completed conversions.

---

# 149. Relationship to PAY-0021

HyperSwitch upgrades SHALL be assessed for changes affecting:

```text
multi-currency processing
currency representation
FX-capable connectors
settlement currency
payout currency
provider routing
```

before production adoption.

---

# 150. Relationship to PAY-0022

PAY-0022 certification SHALL determine whether a provider is permitted to execute a particular FX/cross-border capability.

Conceptually:

```text
Provider Integrated
       │
       ▼
Currency Support Verified
       │
       ▼
FX Pair Certified
       │
       ▼
Legal Entity Activated
       │
       ▼
Market / Transaction Type Activated
       │
       ▼
FX Route Eligible
```

---

# 151. Complete Financial Currency Chain

```text
COMMERCIAL DOMAIN
      │
      ▼
Authoritative Obligation
      │
      ▼
Transaction Money
      │
      ▼
BAOBAB PAYMENTS
      │
      ├──── No FX ──────────────┐
      │                         │
      └──── FX Required         │
              │                 │
              ▼                 │
        FX Eligibility          │
              │                 │
              ▼                 │
           Quote               │
              │                 │
              ▼                 │
         Conversion             │
              │                 │
              ▼                 │
      Destination Money         │
              │                 │
              └─────────┬───────┘
                        ▼
                HYPERSWITCH /
                  PROVIDER
                        │
                        ▼
               External Execution
                        │
                        ▼
                  Settlement
                        │
                        ▼
                 Reconciliation
                        │
                        ▼
                       ERP
                        │
                        ▼
               Accounting FX Policy
```

---

# 152. Final Decision

Baobab SHALL permanently distinguish:

```text
MONEY
    from
FX

FX
    from
CROSS-BORDER

TRANSACTION CURRENCY
    from
SETTLEMENT CURRENCY

SETTLEMENT CURRENCY
    from
ACCOUNTING CURRENCY

PAYOUT CURRENCY
    from
BENEFICIARY CURRENCY

FX QUOTE
    from
FX CONVERSION

PROVIDER FX RATE
    from
ACCOUNTING FX RATE

FX SPREAD
    from
EXPLICIT FEE

CANONICAL MARKET
    from
COUNTRY/CURRENCY ASSUMPTIONS
```

The authoritative model is:

```text
BUSINESS DOMAIN
determines the monetary obligation
        │
        ▼
BAOBAB CONTEXT
determines Legal Entity + Market
        │
        ▼
PAYMENTS
preserves transaction money
        │
        ▼
FX REQUIRED?
        │
        ├── NO ─────────────────────┐
        │                           │
        └── YES                     │
             │                      │
             ▼                      │
      authorised FX terms           │
             │                      │
             ▼                      │
      eligible FX provider          │
             │                      │
             ▼                      │
         conversion                 │
             │                      │
             └──────────┬───────────┘
                        ▼
              payment / payout
                        │
                        ▼
                    settlement
                        │
                        ▼
                 reconciliation
                        │
                        ▼
                       ERP
                        │
                        ▼
              accounting treatment
```

**The commercial domain determines what money is owed.  
The Control Plane determines the canonical business context.  
Payments preserves the monetary denomination and determines whether an authorised conversion is required.  
FX execution transforms one explicit monetary value into another under traceable terms.  
HyperSwitch and providers may execute that conversion only inside the authorised envelope.  
Settlement records what the provider ultimately delivered.  
ERP determines the accounting meaning of those financial facts.**

**A market is not a currency. A currency difference is not proof of a cross-border transaction. A cross-border transaction does not necessarily require FX. A provider-supported currency does not imply an authorised currency pair. And no provider, routing decision, settlement convention, failover mechanism, or accounting convenience may silently change the monetary obligation that Baobab was authorised to execute.**