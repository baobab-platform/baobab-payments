# ADR-PAY-0005 — Market, Currency & Payment-Method Context

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Payment Context / Market & Currency |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001; ADR-PAY-0002; ADR-PAY-0003; ADR-PAY-0004; ADR-SHARED-011; applicable Control Plane Market, Context, Capability and Mapping ADRs |
| **Related** | ADR-PAY-0006; ADR-PAY-0007; ADR-PAY-0008; ADR-PAY-0009; ADR-PAY-0012; ADR-PAY-0015; ADR-PAY-0017; ADR-PAY-0018; ADR-PAY-0022 |

---

# 1. Context

PAY-0003 and PAY-0004 establish the identity side of payment execution:

```text
Tenant
  │
  ▼
Organisation
  │
  ▼
Legal Entity
  │
  ▼
Merchant
  │
  ▼
Business Profile
```

That hierarchy does not yet establish whether a particular payment may actually be executed.

Payment eligibility additionally depends on contextual dimensions such as:

- Market;
- transaction currency;
- settlement arrangements;
- payment method;
- transaction type;
- legal-entity activation;
- provider capability;
- provider certification;
- merchant/profile configuration;
- environment;
- potentially cross-border rules.

For example, the fact that ZuriBeans has a valid production Merchant does not establish that it may process:

```text
UGX
ZAR
USD
```

through every configured provider.

Likewise, a provider technically supporting cards or mobile money does not mean those methods are authorised for every:

```text
Tenant × Legal Entity × Market × Currency
```

combination.

Baobab therefore requires a canonical payment-context model that determines the permitted execution envelope before provider routing occurs.

---

# 2. Decision

Every payment operation SHALL execute within an explicitly resolved canonical payment context.

At minimum, production execution SHALL establish:

```text
Tenant
Organisation
Legal Entity
Market
Currency
Transaction Type
Payment Method / Payment Method Category
Payment Engine Instance
Environment
```

and, where applicable:

```text
Digital Estate
Merchant
Business Profile
Provider Eligibility
Settlement Currency
Cross-Border Context
```

These dimensions SHALL be resolved and validated before external payment execution.

No individual dimension SHALL independently imply payment eligibility.

---

# 3. Governing Principle

> **Market establishes where the business context applies. Currency establishes the monetary denomination. Payment Method establishes how payment may be performed. Legal Entity establishes whose business is being conducted. Provider eligibility determines which certified execution paths may satisfy that context.**

The complete context determines execution eligibility.

---

# 4. Payment Context

Conceptually:

```text
PaymentContext
│
├── tenant_id
├── organisation_id
├── legal_entity_id
├── market_id
├── digital_estate_id?
│
├── transaction_currency
├── settlement_currency?
│
├── payment_method_type
├── transaction_type
│
├── engine_instance_id
├── environment
│
├── merchant_mapping
├── business_profile_mapping
│
├── source_engine
├── source_reference
│
└── correlation_id
```

This is a conceptual model.

Canonical contracts SHALL remain governed by Shared and Control Plane contracts.

---

# 5. Market Authority

`Market` remains a canonical Baobab concept.

Payments SHALL consume Market identity from authoritative platform context.

Payments SHALL NOT create an independent competing market registry.

Therefore:

```text
Baobab Market
      ≠
HyperSwitch Business Profile
```

and:

```text
Baobab Market
      ≠
Medusa Region
```

and:

```text
Baobab Market
      ≠
Provider Country Configuration
```

These external concepts may map to or constrain Market.

They do not define it.

---

# 6. Market Versus Geography

A Market is not merely a country code.

For example:

```text
ZA
```

may participate in identifying the South African market, but a canonical Market may additionally carry policy concerning:

```text
legal entity
currency
tax context
commercial availability
payment eligibility
engine bindings
```

Payments SHALL therefore consume canonical `market_id` rather than infer Market solely from country.

---

# 7. Market Resolution

Payment execution SHALL use server-authoritative Market resolution.

Conceptually:

```text
Incoming Commercial Request
        │
        ▼
Resolved Baobab Context
        │
        ▼
Canonical Market
        │
        ▼
Payment Context
```

The browser SHALL NOT establish payment authority merely by submitting:

```text
market = ZA
```

or:

```text
market = UG
```

Client values may participate as requests or hints but require authoritative resolution.

---

# 8. Market Activation

A canonical Market existing in Baobab does not imply that Payments is active in that Market.

Therefore:

```text
Market exists
      ≠
Payments enabled
```

and:

```text
Payments enabled
      ≠
Provider available
```

and:

```text
Provider available
      ≠
Provider certified and activated
```

PAY-0022 governs provider certification and activation.

---

# 9. Legal Entity × Market

Payment eligibility SHALL be evaluated for the Legal Entity and Market together.

For example:

```text
ZuriBeans
   │
   ├── ZA
   └── UG
```

does not imply that both markets have identical:

```text
merchant configuration
provider contracts
payment methods
currencies
settlement accounts
routing
```

Each legal-entity/market combination SHALL be independently valid.

---

# 10. Market Inheritance Is Prohibited

The following inference SHALL NOT occur:

```text
Parent Organisation active in ZA
          │
          ▼
all subsidiaries active in ZA
```

Similarly:

```text
ZuriBeans active in UG
          │
          ▼
Thamani automatically active in UG
```

is prohibited.

Market/payment activation is scoped to the applicable Legal Entity.

---

# 11. Currency

Every authoritative monetary payment amount SHALL carry explicit currency.

Conceptually:

```text
Money
│
├── amount
└── currency
```

Currency SHALL NOT be inferred from:

```text
tenant
country
market
provider
merchant
browser locale
IP address
```

once the authoritative monetary obligation has been established.

---

# 12. Transaction Currency

The **Transaction Currency** is the currency in which the payment obligation is presented for payment.

For example:

```text
amount   = 150000
currency = UGX
```

or:

```text
amount   = 1250.00
currency = ZAR
```

according to the canonical monetary representation adopted by Shared contracts.

Payments SHALL preserve the authoritative currency supplied by the originating obligation.

---

# 13. Currency Mutation

Payments SHALL NOT silently convert:

```text
UGX → USD
```

or:

```text
ZAR → USD
```

merely because a provider prefers another currency.

Any conversion requires explicit cross-border/FX semantics.

PAY-0017 governs that architecture.

---

# 14. Market Currency

A Market MAY define one or more permitted transaction currencies.

Conceptually:

```text
Market ZA
   │
   ├── ZAR
   └── other explicitly permitted currencies

Market UG
   │
   ├── UGX
   └── other explicitly permitted currencies
```

The exact currencies permitted are configuration/policy rather than hard-coded architectural assumptions.

---

# 15. Market Currency Validation

Before payment execution:

```text
transaction_currency
```

SHALL be checked against:

```text
Market
Legal Entity
Provider activation
Payment Method
Transaction Type
```

as applicable.

A currency supported by a provider is not automatically valid for the business context.

---

# 16. Provider Currency Support

Provider capability is a necessary but insufficient condition.

Conceptually:

```text
Provider supports USD
       │
       ▼
Technical Capability
```

does not establish:

```text
Legal Entity A
may process USD
through Provider
in Market ZA
```

That requires governed eligibility.

---

# 17. Settlement Currency

Transaction Currency and Settlement Currency SHALL remain distinct concepts.

For example:

```text
Customer pays
UGX
   │
   ▼
Provider processing
   │
   ▼
Merchant settles
USD
```

may be possible under a specific provider arrangement.

Therefore:

```text
Transaction Currency
       ≠ necessarily
Settlement Currency
```

Settlement currency SHALL NOT overwrite transaction currency.

---

# 18. Settlement Currency Authority

Settlement currency SHALL be determined by explicit provider/merchant/legal-entity arrangements.

It SHALL not be guessed from transaction currency.

Settlement consequences are governed further by:

- PAY-0015 — Settlement & ERP Reconciliation;
- PAY-0017 — FX & Cross-Border Payment Semantics.

---

# 19. Payment Method

A Payment Method describes how the payer satisfies a payment obligation.

Canonical high-level categories MAY include:

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

Provider-specific method names SHALL be translated.

---

# 20. Payment Method Category Versus Provider Method

Baobab SHALL distinguish:

```text
Canonical Payment Method
```

from:

```text
Provider Payment Method
```

For example:

```text
Canonical:
MOBILE_MONEY

Provider:
provider-specific mobile-money connector/method
```

The adapter layer SHALL translate between them.

---

# 21. Payment Method Eligibility

Payment-method eligibility SHALL be contextual.

Conceptually:

```text
EligiblePaymentMethod =
    Tenant
  × LegalEntity
  × Market
  × Currency
  × TransactionType
  × ProviderCertification
  × ProviderActivation
  × Merchant/ProfileConfiguration
```

This does not require implementing eligibility as a literal Cartesian-product table.

It defines the semantic intersection that must be valid.

---

# 22. Payment Method Availability

The frontend SHALL NOT treat provider-advertised payment methods as authoritative customer options.

The correct direction is:

```text
Canonical Context
      │
      ▼
Payments Eligibility Resolution
      │
      ▼
Eligible Payment Methods
      │
      ▼
Trade / Digital Estate
      │
      ▼
Customer
```

not:

```text
Frontend
   │
   ▼
show every provider method
   │
   ▼
hope payment works
```

---

# 23. Server-Authoritative Eligibility

Eligible methods SHALL be calculated or retrieved server-side.

The client may request:

```text
CARD
```

but Payments SHALL verify whether CARD is valid for the resolved context.

Client-provided payment method selection is a request, not authority.

---

# 24. Transaction Type

Eligibility may vary by transaction type.

Representative transaction types include:

```text
AUTHORIZE
CAPTURE
SALE
REFUND
VOID
PAYOUT
```

A provider being eligible for:

```text
CARD / CAPTURE
```

does not necessarily mean it is eligible for:

```text
REFUND
```

or:

```text
PAYOUT
```

in the same context.

---

# 25. Capability-Level Eligibility

Provider eligibility SHOULD therefore be capability-aware.

Conceptually:

```text
Provider A

ZA
├── ZAR
│   ├── CARD
│   │   ├── AUTHORIZE  ✓
│   │   ├── CAPTURE    ✓
│   │   ├── REFUND     ✓
│   │   └── PAYOUT     ✗
│   │
│   └── BANK_TRANSFER
│       ├── PAYMENT    ✓
│       └── REFUND     ?
```

Eligibility SHALL not be reduced to:

```text
provider_enabled = true
```

---

# 26. Business Profile

A HyperSwitch Business Profile MAY represent some portion of the resolved payment context.

For example:

```text
Legal Entity A
      │
      ▼
Merchant A
      │
      ├── Profile ZA
      └── Profile UG
```

may be an appropriate operational mapping.

However:

```text
Profile ZA
```

does not become the canonical Market ZA.

The mapping remains explicit.

---

# 27. Profile Resolution

Business Profile selection SHALL be server-authoritative.

Conceptually:

```text
Tenant
  │
Organisation
  │
Legal Entity
  │
Market
  │
Currency
  │
Payment Method
  │
  ▼
Resolve Merchant
  │
  ▼
Resolve Business Profile
```

A client SHALL NOT arbitrarily select another Business Profile.

---

# 28. Profile Reuse

One Business Profile MAY serve multiple compatible contexts only where explicitly governed.

Profile reuse SHALL NOT erase distinctions required for:

```text
legal entity
market
currency
settlement
provider activation
reconciliation
```

---

# 29. Profile Splitting

Conversely, one canonical Market MAY map to multiple Business Profiles where necessary.

For example:

```text
ZA Market
   │
   ├── Profile: B2B
   └── Profile: B2C
```

or:

```text
ZA Market
   │
   ├── Profile: ZAR Domestic
   └── Profile: Cross-Border
```

if justified by payment architecture.

The canonical Market remains one canonical entity.

---

# 30. Digital Estate

Digital Estate MAY contribute contextual information but SHALL NOT determine payment eligibility independently.

For example:

```text
ZuriBeans Digital Estate
        │
        ▼
resolved tenant/legal entity/market
        │
        ▼
Payments Context
```

is valid.

The following is not:

```text
hostname = zuribeans.example
        │
        ▼
assume Merchant X
```

without authoritative context resolution.

---

# 31. Trade Boundary

Trade determines commercial context such as:

```text
order
amount due
currency
customer selection
```

within its authority.

Payments validates whether that payment request can be executed in the resolved payment context.

Therefore:

```text
Trade:
"What must this customer pay?"

Payments:
"How may that payment be executed?"
```

PAY-0006 defines the detailed integration.

---

# 32. Subscription Boundary

Subscriptions determines:

```text
whether payment is required
amount due
billing obligation
billing currency
```

within its authority.

Payments determines whether and how that obligation can be executed through the available payment infrastructure.

PAY-0007 defines the detailed integration.

---

# 33. INTERNAL Subscription Rule

An INTERNAL subscription with no monetary obligation SHALL NOT acquire a payment method merely because Payments can resolve one.

The correct flow remains:

```text
Payment required?
      │
      ├── NO ───► Do not invoke payment execution
      │
      └── YES ──► Resolve payment context
```

---

# 34. Payment Method Reference

Payment-method eligibility and Payment Method Reference are separate.

For example:

```text
CARD
```

may be eligible.

A particular:

```text
token_abc
```

may still be invalid because it:

```text
belongs to another customer
belongs to another tenant
is expired
is provider-incompatible
is environment-incompatible
```

PAY-0012 defines sensitive payment references and tokenisation.

---

# 35. Stored Payment Methods

A stored payment method SHALL preserve sufficient provenance to determine where it may be reused.

Relevant scope may include:

```text
tenant
payer/customer
legal entity
provider
merchant
environment
tokenisation domain
payment method
```

A stored token SHALL NOT be assumed portable across:

```text
providers
merchants
legal entities
environments
```

unless the tokenisation architecture explicitly supports such portability.

---

# 36. Market Expansion

Adding a new canonical Market SHALL NOT automatically enable payments.

For example:

```text
Add Market KE
```

does not imply:

```text
Payments KE = ACTIVE
```

Payment activation requires the applicable sequence:

```text
Canonical Market
      │
      ▼
Legal Entity eligibility
      │
      ▼
Merchant/Profile configuration
      │
      ▼
Currency configuration
      │
      ▼
Payment Method configuration
      │
      ▼
Provider certification
      │
      ▼
Provider activation
      │
      ▼
Routing eligibility
      │
      ▼
Production readiness
```

---

# 37. Currency Expansion

Likewise:

```text
Provider supports currency X
```

does not automatically activate currency X.

Currency activation requires explicit governed context.

---

# 38. Payment-Method Expansion

Adding a new payment method SHALL follow controlled activation.

Conceptually:

```text
Method discovered
      │
      ▼
Integration supported
      │
      ▼
Provider capability confirmed
      │
      ▼
Security/compliance assessed
      │
      ▼
Legal Entity approved
      │
      ▼
Market approved
      │
      ▼
Currency compatibility confirmed
      │
      ▼
Production activated
```

---

# 39. No Hard-Coded Market Routing

Digital estates and originating engines SHALL NOT contain logic such as:

```text
if market == "ZA":
    provider = ProviderA

if market == "UG":
    provider = ProviderB
```

Provider routing belongs to Payments.

The originating engine supplies canonical context.

---

# 40. No Hard-Coded Currency Routing

Likewise:

```text
if currency == "USD":
    use ProviderX
```

SHALL NOT exist in leaf digital estates or commerce code.

Currency contributes to provider eligibility.

Payments owns routing decisions within authorised context.

---

# 41. No Hard-Coded Payment-Method Routing

Similarly:

```text
if payment_method == "MOBILE_MONEY":
    call ProviderY directly
```

is prohibited outside the payment orchestration boundary.

The correct flow is:

```text
Payment Method
      │
      ▼
Eligibility
      │
      ▼
Routing
      │
      ▼
Eligible Provider
```

---

# 42. Provider Eligibility

A provider SHALL participate in routing only when all required conditions hold.

Conceptually:

```text
ProviderEligible =
    Certified
AND Activated
AND LegalEntityEligible
AND MarketEligible
AND CurrencyEligible
AND PaymentMethodEligible
AND TransactionTypeEligible
AND MerchantConfigured
AND ProfileConfigured
AND EnvironmentValid
```

Additional constraints MAY apply.

---

# 43. Routing Comes After Eligibility

The architecture SHALL distinguish:

```text
ELIGIBILITY
```

from:

```text
ROUTING
```

Eligibility asks:

> Which providers are legally, commercially, technically and operationally allowed to execute this payment?

Routing asks:

> Among those eligible providers, which should execute this payment?

Therefore:

```text
Canonical Context
       │
       ▼
Eligibility
       │
       ▼
Eligible Provider Set
       │
       ▼
Routing
       │
       ▼
Selected Provider
```

PAY-0009 governs routing.

---

# 44. Routing Cannot Expand Eligibility

A routing algorithm SHALL NOT select a provider excluded by canonical eligibility.

Therefore:

```text
Routing candidates
        ⊆
Eligible providers
```

always.

Retry and failover SHALL obey the same invariant.

---

# 45. Cross-Border Context

A transaction may involve several jurisdictions or currencies.

Conceptually:

```text
Legal Entity jurisdiction
          │
Customer / payer location
          │
Canonical Market
          │
Transaction Currency
          │
Provider processing location
          │
Settlement Currency
```

These dimensions SHALL not be collapsed into one `country` field.

PAY-0017 defines cross-border semantics.

---

# 46. Customer Location

Customer location MAY affect:

```text
method availability
regulatory requirements
risk
authentication
provider capability
```

but customer location SHALL NOT independently determine canonical Market.

The canonical Market remains authoritative.

---

# 47. IP Geolocation

IP geolocation MAY be used as:

```text
signal
default suggestion
risk input
```

where appropriate.

It SHALL NOT independently establish:

```text
Legal Entity
Market
Currency authority
Merchant
Provider
```

for financial execution.

---

# 48. Browser Locale

Browser locale SHALL have no authority over payment context.

For example:

```text
browser locale = en-US
```

does not mean:

```text
currency = USD
market = US
```

---

# 49. FX

Where:

```text
Transaction Currency
        ≠
Settlement Currency
```

or another currency conversion occurs, the conversion SHALL be explicit.

The system SHALL preserve:

```text
source amount
source currency
target amount
target currency
rate
rate source
rate timestamp
fees where applicable
```

according to PAY-0017 and accounting requirements.

---

# 50. Amount Integrity

Provider routing SHALL NOT silently change the authoritative payment amount.

If a provider requires:

```text
minor-unit transformation
rounding representation
format conversion
```

the adapter MAY perform representation translation.

It SHALL preserve the canonical monetary meaning.

---

# 51. Minor Units

Provider adapters SHALL handle provider-specific monetary representation.

For example, a provider requiring integer minor units SHALL receive the correctly transformed value.

That representation SHALL NOT change the canonical currency semantics.

Currency metadata SHALL govern the conversion rather than hard-coded assumptions that every currency has two decimal places.

---

# 52. Currency Precision

Baobab SHALL use exact monetary arithmetic.

Binary floating-point SHALL NOT be authoritative for payment values.

The implementation SHALL respect canonical currency precision and contract representation.

---

# 53. Environment

Eligibility SHALL always be environment-aware.

Therefore:

```text
SANDBOX provider activation
        ≠
PRODUCTION provider activation
```

and:

```text
SANDBOX payment method
        ≠
PRODUCTION readiness
```

---

# 54. Simulated Execution

Where the sandbox provider is used:

```text
simulated = true
```

SHALL propagate through relevant payment records and events.

A simulated payment SHALL never satisfy production financial evidence.

---

# 55. Context Immutability

Core payment context SHOULD become immutable once external payment execution begins.

In particular, an existing Payment SHALL NOT silently switch:

```text
tenant
legal entity
market
transaction currency
```

after execution has begun.

Where correction requires a different business context, the system SHOULD create an appropriate new payment operation rather than mutate financial history.

---

# 56. Merchant/Profile Immutability

The Merchant and Business Profile actually used for execution SHALL be preserved as historical execution context.

Later configuration changes SHALL not rewrite previous payments.

---

# 57. Provider Immutability

Likewise, the provider actually used for a Payment Attempt is a historical fact.

Retry through another provider SHALL create or preserve a distinct attempt rather than rewriting the previous provider.

---

# 58. Payment Context Snapshot

Payments SHOULD preserve sufficient immutable execution context to reconstruct why a payment was considered eligible.

The snapshot MAY include references to:

```text
tenant
organisation
legal entity
market
currency
payment method
merchant
business profile
provider
engine instance
activation/configuration revision
```

The snapshot SHALL not duplicate unnecessary master data.

---

# 59. Policy Versioning

Where payment eligibility depends on mutable policy, the system SHOULD retain enough revision information to answer:

> Why was this provider/payment method considered eligible at the time of execution?

This is important for:

```text
audit
incident investigation
disputes
regulatory review
reconciliation
```

---

# 60. Fail-Closed Behaviour

If required context is unknown or contradictory, production payment execution SHALL fail closed.

Examples:

```text
Market unknown
Currency unsupported
Legal Entity inactive
Merchant mapping missing
Profile mapping ambiguous
Payment Method not activated
Provider not certified
```

all result in:

```text
PAYMENT CONTEXT NOT ELIGIBLE
```

rather than best-effort provider execution.

---

# 61. No Silent Fallback Across Market

A failed provider in Market ZA SHALL NOT cause routing to a provider configuration intended only for Market UG.

---

# 62. No Silent Fallback Across Currency

A failed ZAR payment SHALL NOT be retried as USD merely to find another provider.

---

# 63. No Silent Fallback Across Legal Entity

A payment for ZuriBeans SHALL NOT be retried through Thamani's Merchant because its provider is available.

---

# 64. No Silent Fallback Across Method

If the payer selected an authorised payment method:

```text
BANK_TRANSFER
```

Payments SHALL NOT silently transform the transaction into:

```text
CARD
```

to achieve success.

A method change requires an explicit new payer/business decision where appropriate.

---

# 65. Context Resolution Flow

```text
Payment Requested
      │
      ▼
Authenticate Caller
      │
      ▼
Resolve Tenant
      │
      ▼
Resolve Organisation
      │
      ▼
Resolve Legal Entity
      │
      ▼
Resolve Market
      │
      ▼
Validate Currency
      │
      ▼
Resolve Payment Method
      │
      ▼
Resolve Engine Instance
      │
      ▼
Resolve Merchant
      │
      ▼
Resolve Business Profile
      │
      ▼
Determine Provider Eligibility
      │
      ├── none ─────────► BLOCK
      │
      ▼
Eligible Provider Set
      │
      ▼
Routing
      │
      ▼
Execute
```

---

# 66. Eligibility Matrix

A conceptual eligibility matrix may resemble:

| Legal Entity | Market | Currency | Method | Transaction | Provider | Status |
|---|---|---|---|---|---|---|
| ZuriBeans | ZA | ZAR | CARD | PAYMENT | Provider A | Eligible |
| ZuriBeans | ZA | ZAR | REFUND-capable route | REFUND | Provider A | Eligible |
| ZuriBeans | UG | UGX | MOBILE_MONEY | PAYMENT | Provider B | Eligible |
| Thamani | ZA | ZAR | CARD | PAYMENT | Provider A | Independently governed |
| Thamani | UG | UGX | MOBILE_MONEY | PAYMENT | Provider B | Independently governed |

This table is illustrative.

Actual provider activation requires PAY-0022 certification evidence.

---

# 67. Capability Binding

Payment capability availability SHOULD align with canonical Capability and CapabilityBinding architecture.

For example:

```text
payment.intent.create
payment.payment.authorize
payment.payment.capture
payment.refund.create
```

shall remain subject to appropriate canonical context.

Capability availability does not automatically imply that a provider route exists.

---

# 68. API Behaviour

Payment APIs SHOULD accept canonical business context rather than expose provider configuration as primary caller-controlled parameters.

A caller SHOULD NOT need to know:

```text
HyperSwitch Organisation ID
Merchant ID
Business Profile ID
Connector Account ID
Provider Credential
```

to create a normal Baobab payment.

Payments resolves those operational details.

---

# 69. Frontend Behaviour

Digital estates SHOULD receive an eligibility projection suitable for presentation.

For example:

```text
Available Payment Methods

CARD
BANK_TRANSFER
MOBILE_MONEY
```

The frontend SHOULD NOT receive sensitive provider configuration merely to display those methods.

---

# 70. Provider Abstraction

Where multiple providers can satisfy:

```text
CARD
```

the customer experience need not expose which processor will ultimately execute the transaction unless business or regulatory requirements require it.

Provider selection remains an orchestration concern.

---

# 71. Security

Payment context SHALL be treated as security-sensitive because manipulating it could redirect money movement.

Particularly sensitive fields include:

```text
tenant
legal entity
market
merchant
business profile
provider
settlement configuration
```

Caller-supplied values SHALL never override trusted server-authoritative resolution.

---

# 72. Controlled Mutation

Changes to production eligibility SHALL be privileged.

Examples include:

```text
enable currency
disable currency
enable payment method
disable payment method
activate market
suspend market
change profile mapping
change settlement currency
change provider eligibility
```

PAY-0018 defines detailed authorisation.

---

# 73. Audit

Production context configuration SHALL permit reconstruction of:

```text
Who enabled the market?

Who enabled the currency?

Who enabled the payment method?

Which legal entity was affected?

Which provider was eligible?

Which Merchant/Profile was used?

When did the configuration become effective?

Which policy/configuration revision applied?
```

---

# 74. Drift Detection

Baobab SHOULD detect divergence between canonical eligibility and operational HyperSwitch configuration.

Examples:

```text
Baobab:
CARD disabled

HyperSwitch:
CARD still enabled
```

or:

```text
Baobab:
Provider A suspended in ZA

HyperSwitch:
Provider A still routable
```

or:

```text
Baobab:
UGX not activated

HyperSwitch:
connector technically supports UGX
```

Operational capability SHALL NOT override canonical policy.

---

# 75. Readiness

Representative readiness evaluation:

```text
PAYMENT CONTEXT READINESS

Tenant                           PASS
Organisation                     PASS
Legal Entity                     PASS
Market                           PASS
Currency                         PASS
Payment Method                   PASS
Engine Instance                  PASS
Merchant Mapping                 PASS
Business Profile                 PASS
Provider Certification           PASS
Provider Activation              PASS
Transaction Capability           PASS
Settlement Configuration         PASS
Reconciliation Path              PASS
-------------------------------------
Payment Context                  READY
```

Any mandatory failure results in:

```text
Payment Context = BLOCKED
```

---

# 76. Observability

Operational telemetry SHOULD support correlation by:

```text
tenant_id
organisation_id
legal_entity_id
market_id
transaction_currency
settlement_currency where relevant
payment_method
transaction_type
merchant_id
business_profile_id
provider
payment_intent_id
payment_id
correlation_id
```

Sensitive payment data SHALL not be logged.

---

# 77. Metrics

Useful aggregate metrics MAY include:

```text
payments by market
payments by currency
payments by method
payments by provider
provider success by context
provider latency by context
eligibility rejection count
routing failure count
unsupported method count
unsupported currency count
```

Metrics SHALL preserve tenant isolation and appropriate data-access controls.

---

# 78. Events

Where canonical configuration events are required, representative events MAY include:

```text
payment.market.activated.v1
payment.market.suspended.v1

payment.currency.activated.v1
payment.currency.suspended.v1

payment.method.activated.v1
payment.method.suspended.v1

payment.context.eligibility_changed.v1
```

Where generic Control Plane configuration/capability events already express the required fact, Payments SHOULD consume those rather than duplicate authority.

---

# 79. Initial Market Architecture

Initial platform payment architecture SHALL support at least the canonical distinction between:

```text
UG
ZA
```

without embedding either market into core routing code.

Future markets such as:

```text
KE
TZ
RW
```

SHALL be introduced through canonical configuration, mappings, certification and activation rather than architectural forks.

---

# 80. Multi-Market Legal Entity

A Legal Entity operating in multiple markets SHALL preserve market-specific payment context.

Conceptually:

```text
Legal Entity
      │
      ├── Market ZA
      │     ├── currencies
      │     ├── methods
      │     ├── profiles
      │     └── eligible providers
      │
      └── Market UG
            ├── currencies
            ├── methods
            ├── profiles
            └── eligible providers
```

Configuration in one branch SHALL not silently alter another.

---

# 81. Multi-Currency Market

Where a market legitimately supports multiple currencies:

```text
Market
  │
  ├── Currency A
  ├── Currency B
  └── Currency C
```

each currency MAY have distinct:

```text
payment methods
providers
fees
FX behaviour
settlement behaviour
```

Eligibility SHALL therefore remain currency-aware.

---

# 82. Same Currency Across Markets

The same currency used across several markets SHALL NOT collapse those markets.

For example:

```text
Market A ── USD
Market B ── USD
```

does not imply:

```text
Market A = Market B
```

Provider eligibility remains market-aware.

---

# 83. Same Method Across Markets

Likewise:

```text
CARD in ZA
```

and:

```text
CARD in UG
```

may have different:

```text
providers
authentication requirements
merchant accounts
routing policies
fees
```

The canonical method may be the same while execution context differs.

---

# 84. Payment Method Presentation

Provider-specific implementation detail SHOULD be separated from customer-facing payment method presentation.

Conceptually:

```text
Customer sees:
CARD

Payments may resolve:
Provider A / connector X
```

This permits provider migration without unnecessarily changing estate UX.

---

# 85. Configuration Hierarchy

Payment configuration SHOULD support controlled inheritance where useful, but explicit narrower policy SHALL take precedence.

Conceptually:

```text
Platform Defaults
      │
      ▼
Tenant Policy
      │
      ▼
Legal Entity Policy
      │
      ▼
Market Policy
      │
      ▼
Currency / Method Policy
```

However, inheritance SHALL never broaden a narrower prohibition.

---

# 86. Deny Precedence

Where conflicting policies exist:

```text
broader scope = ALLOW
narrower scope = DENY
```

the effective result SHALL be:

```text
DENY
```

unless an explicitly governed policy model specifies otherwise.

Financial execution SHALL favour fail-closed semantics.

---

# 87. Context Evaluation

Conceptually:

```text
EffectivePaymentContext =
    CanonicalContext
  ∩ LegalEntityPolicy
  ∩ MarketPolicy
  ∩ CurrencyPolicy
  ∩ PaymentMethodPolicy
  ∩ ProviderCertification
  ∩ ProviderActivation
  ∩ Merchant/ProfileConfiguration
  ∩ TransactionCapability
```

Only the resulting permitted set participates in routing.

---

# 88. Invariants

The following invariants SHALL hold:

1. Market is canonical Baobab context.
2. Market is not a HyperSwitch Business Profile.
3. Market is not a Medusa Region.
4. Currency is explicit for every authoritative monetary amount.
5. Currency SHALL not be inferred from browser locale.
6. Currency SHALL not be silently changed for provider convenience.
7. Transaction Currency and Settlement Currency remain distinct.
8. Payment Method is canonical and provider-independent.
9. Provider-specific method codes SHALL not leak into canonical contracts.
10. Payment-method availability is context-dependent.
11. Provider technical support does not establish business eligibility.
12. Legal Entity activation is independent by Market.
13. Subsidiary payment activation is not inherited from a parent.
14. One subsidiary's activation does not activate another.
15. Business Profile selection is server-authoritative.
16. Merchant selection is server-authoritative.
17. Provider selection is server-authoritative.
18. Routing occurs only after eligibility.
19. Routing cannot expand eligibility.
20. Retry cannot bypass eligibility.
21. Failover cannot cross Legal Entity silently.
22. Failover cannot cross Market silently.
23. Failover cannot change Currency silently.
24. Failover cannot change Payment Method silently.
25. Sandbox eligibility does not establish production eligibility.
26. Historical payment context remains attributable to the context actually used.
27. A Market can support multiple currencies.
28. A currency can exist in multiple Markets without collapsing them.
29. A payment method can have different provider routes across Markets.
30. Unknown or ambiguous production context fails closed.

---

# 89. Consequences

## Positive

This decision:

- establishes deterministic payment context;
- preserves canonical Market authority;
- enables multi-market operation;
- enables multi-currency operation;
- supports local payment methods;
- prevents frontend/provider routing leakage;
- separates eligibility from routing;
- protects legal-entity boundaries;
- supports future African market expansion;
- enables cross-border payment architecture;
- provides strong production readiness checks;
- reduces provider lock-in.

## Negative

It requires:

- context resolution;
- eligibility configuration;
- currency validation;
- payment-method registries/vocabularies;
- provider capability modelling;
- profile mapping;
- activation workflows;
- policy evaluation;
- drift detection.

These costs are accepted because implicit payment context creates unacceptable financial and operational ambiguity.

---

# 90. Alternatives Considered

## Infer Market from country

Rejected.

Market is a richer canonical business context.

---

## Infer currency from Market

Rejected.

Markets may support multiple currencies.

---

## Let HyperSwitch Business Profile define Market

Rejected.

Business Profile is an operational payment representation.

---

## Let Medusa Region define Market

Rejected.

Trade is a consumer of canonical Market context, not platform Market authority.

---

## Let providers advertise available payment methods directly to frontends

Rejected.

Technical provider capability is not equivalent to governed Baobab eligibility.

---

## Hard-code provider per market

Rejected.

Routing belongs to Payments and must remain configurable.

---

## Automatically convert unsupported currencies

Rejected.

Currency conversion has financial, commercial and accounting consequences requiring explicit FX semantics.

---

# 91. Relationship to Subsequent ADRs

PAY-0005 completes the canonical context necessary for the integration ADRs.

```text
PAY-0002
Payment Domain
     │
     ▼
PAY-0003
Tenant / Organisation
     │
     ▼
PAY-0004
Legal Entity / Merchant / Profile
     │
     ▼
PAY-0005
Market / Currency / Payment Method
     │
     ├───────────────┐
     ▼               ▼
PAY-0006          PAY-0007
Trade             Subscriptions
Integration       Integration
     │               │
     └───────┬───────┘
             ▼
         PAY-0008
      Payment Lifecycle
             │
             ▼
         PAY-0009
      Routing / Failover
```

---

# 92. Complete Pre-Routing Context

Following PAY-0003 through PAY-0005, the canonical pre-routing context becomes:

```text
WHO IS THE PLATFORM CUSTOMER?
        │
        ▼
Tenant

WHO IS THE ORGANISATION?
        │
        ▼
Organisation

WHO IS LEGALLY CONDUCTING BUSINESS?
        │
        ▼
Legal Entity

WHERE IS THE BUSINESS CONTEXT?
        │
        ▼
Market

WHAT MONEY IS BEING REQUESTED?
        │
        ▼
Currency

HOW DOES THE PAYER WANT TO PAY?
        │
        ▼
Payment Method

WHAT OPERATION IS REQUIRED?
        │
        ▼
Transaction Type

HOW IS THE LEGAL ENTITY REPRESENTED
IN THE PAYMENT ENGINE?
        │
        ▼
Merchant + Business Profile

WHICH PROVIDERS ARE AUTHORISED?
        │
        ▼
Eligibility Set

WHICH AUTHORISED PROVIDER SHOULD EXECUTE?
        │
        ▼
Routing
```

---

# 93. Final Decision

Baobab SHALL treat payment execution as a function of explicit canonical context:

```text
Payment Execution Context
        =
Tenant
+
Organisation
+
Legal Entity
+
Market
+
Currency
+
Payment Method
+
Transaction Type
+
Engine Instance
+
Merchant/Profile Mapping
+
Provider Eligibility
```

No provider, commerce engine, billing engine, frontend, Business Profile or external identifier may independently redefine that context.

The architecture SHALL therefore preserve the distinctions:

```text
Market
    ≠
Business Profile

Market
    ≠
Medusa Region

Transaction Currency
    ≠ necessarily
Settlement Currency

Payment Method
    ≠
Provider Connector

Provider Support
    ≠
Provider Eligibility

Provider Eligibility
    ≠
Provider Routing
```

The execution sequence is:

```text
CANONICAL BUSINESS CONTEXT
          │
          ▼
PAYMENT CONTEXT VALIDATION
          │
          ▼
PROVIDER ELIGIBILITY
          │
          ▼
ROUTING
          │
          ▼
HYPERSWITCH
          │
          ▼
PAYMENT PROVIDER
```

**Baobab determines the market.  
The commercial engine determines what is owed and in which transaction currency.  
Payments determines which payment methods and providers are eligible.  
HyperSwitch routes only inside that authorised execution envelope.**