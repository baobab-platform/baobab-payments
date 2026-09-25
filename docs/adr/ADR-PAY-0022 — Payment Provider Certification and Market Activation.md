# ADR-PAY-0022 — Payment Provider Certification and Market Activation

**Status:** Accepted  
**Decision Type:** Architecture / Payments Governance / Production Readiness  
**Repository:** `baobab-platform/baobab-payments`  
**Scope:** Baobab Platform  
**Owners:** Nabhold / Baobab Platform Architecture  
**Depends On:** ADR-PAY-0001; ADR-SHARED-007; ADR-SHARED-011; applicable Control Plane, IAM, Infrastructure and ERP ADRs  
**Related:** ADR-PAY-0002 through ADR-PAY-0021  
**Date:** 2026-09-25

---

# 1. Context

ADR-PAY-0001 establishes `baobab-payments` as Baobab's independently deployable headless Payment Orchestration Engine and adopts HyperSwitch as its foundational orchestration implementation.

HyperSwitch can support multiple payment connectors and processor integrations. However, the existence of:

- a HyperSwitch connector;
- a configured Merchant Connector Account;
- valid credentials;
- a technically successful API call;
- a supported currency;
- or a successful sandbox payment

does **not** establish that a payment provider is suitable, lawful, commercially approved, operationally supportable, financially reconcilable or production-ready for a particular Baobab legal entity and market.

Baobab is inherently:

```text
multi-tenant
multi-organisation
multi-legal-entity
multi-market
multi-currency
multi-digital-estate
multi-provider
multi-engine
```

and must support both internal Nabhold Group entities and future external customers.

Consequently, payment-provider availability cannot be treated as a global boolean:

```text
provider.enabled = true
```

because a provider may be appropriate for one combination of:

```text
Legal Entity
Market
Currency
Payment Method
Commercial Model
Transaction Type
```

while being prohibited or unsuitable for another.

For example, a hypothetical provider may be:

```text
technically integrated        = YES
South Africa approved         = YES
Uganda approved               = NO
ZAR                           = YES
UGX                           = NO
cards                         = YES
bank transfer                 = YES
payouts                       = NO
refunds                       = YES
recurring payments            = NO
```

The provider therefore cannot simply be described as "enabled".

Baobab requires an explicit **certification and market-activation lifecycle** between technical integration and production payment routing.

---

# 2. Problem Statement

Without a governed certification model, the following dangerous inference could arise:

```text
HyperSwitch supports Provider X
              │
              ▼
Provider X configured
              │
              ▼
Provider X becomes routable
              │
              ▼
Production money movement
```

That inference is prohibited.

Technical availability is only one component of production readiness.

The required model is:

```text
Provider / Connector exists
            │
            ▼
Technical Integration
            │
            ▼
Certification
            │
            ▼
Legal-Entity Activation
            │
            ▼
Market Activation
            │
            ▼
Currency / Method Eligibility
            │
            ▼
Production Routing Eligibility
```

Only after all applicable gates have passed may a provider participate in production routing.

---

# 3. Decision

Baobab SHALL establish a formal **Payment Provider Certification and Market Activation** model.

No payment provider, processor, acquirer, connector or payment-method integration SHALL become eligible for production money movement merely because:

```text
HyperSwitch supports it;
credentials exist;
configuration exists;
sandbox tests pass;
the connector responds successfully;
or an administrator enables it directly in HyperSwitch.
```

Production eligibility SHALL require an explicit Baobab certification and activation decision.

The governing rule is:

> **Connector availability is a technical fact. Provider certification is a Baobab governance decision. Market activation is an explicitly authorised operational decision. Routing eligibility is derived from both.**

---

# 4. Architectural Principle

The architecture SHALL distinguish four concepts:

```text
CONNECTOR
    │
    │ technical implementation
    ▼
PROVIDER
    │
    │ commercial / operational counterparty
    ▼
CERTIFICATION
    │
    │ approved capability envelope
    ▼
ACTIVATION
    │
    │ authorised use within Baobab context
    ▼
ROUTING ELIGIBILITY
```

These concepts SHALL NOT be collapsed into a single HyperSwitch configuration object.

---

# 5. Definitions

## 5.1 Connector

A **Connector** is the technical integration implementation used to communicate with a provider.

Examples may include:

```text
HyperSwitch connector
Baobab adapter
bank API adapter
mobile-money adapter
future payment rail adapter
```

A connector answers:

> Can Baobab technically communicate with this payment service?

It does not answer:

> May this legal entity use this provider in this market?

---

## 5.2 Payment Provider

A **Payment Provider** is the external payment-processing, acquiring, banking, mobile-money or other financial service counterparty through which payment operations may be executed.

The provider is conceptually independent of its connector implementation.

Therefore:

```text
Provider ≠ Connector
```

A provider could eventually be reachable through more than one connector implementation, and a connector implementation could support several provider configurations.

---

## 5.3 Provider Certification

A **Provider Certification** is Baobab's governed evidence that a provider integration has satisfied the requirements necessary for a defined operational scope.

Certification SHALL be scoped.

It SHALL NOT mean:

```text
Provider X is universally approved.
```

Instead it means approximately:

```text
Provider X

is certified for:

capabilities       = [AUTHORIZE, CAPTURE, REFUND]
markets            = [ZA]
currencies         = [ZAR]
payment_methods    = [CARD]
commercial_models  = [B2C]
environment        = PRODUCTION
subject to         = certification controls
```

---

## 5.4 Market Activation

A **Market Activation** authorises a certified provider for actual use within a resolved Baobab business context.

At minimum, activation SHALL reference:

```text
tenant
organisation where applicable
legal entity
market
provider
provider certification
environment
effective period
```

Additional restrictions MAY include:

```text
currency
payment method
transaction type
digital estate
business profile
commercial model
amount range
customer type
cross-border status
```

---

## 5.5 Routing Eligibility

**Routing Eligibility** is the derived result determining whether a provider may participate in payment routing for a specific transaction context.

Routing eligibility is not itself provider certification.

It is derived from authoritative context and valid certification/activation records.

---

# 6. Authority Model

Authority SHALL be divided as follows.

| Concern | Authority |
|---|---|
| Tenant | `baobab-cp` |
| Organisation | `baobab-cp` |
| Legal Entity | `baobab-cp` |
| Market | `baobab-cp` |
| Digital Estate | `baobab-cp` |
| Capability Binding | `baobab-cp` |
| Workload Identity | `baobab-iam` / platform identity |
| Payment Domain | `baobab-payments` |
| Provider Certification | `baobab-payments` governed payment administration |
| Provider Activation | governed Payments + Control Plane context |
| Connector Configuration | `baobab-payments` / HyperSwitch |
| Payment Routing | HyperSwitch within Baobab eligibility constraints |
| Commercial Obligation | Trade / Subscriptions / other source engine |
| Accounting | `baobab-erp` |
| Secrets | approved platform secrets infrastructure |

HyperSwitch SHALL NOT become authoritative for:

```text
tenant
organisation
legal entity
market
digital estate
provider certification
Baobab production admission
```

---

# 7. Certification Scope

Certification SHALL be explicit about what has actually been tested and approved.

A certification SHOULD be representable conceptually as:

```text
ProviderCertification
│
├── certification_id
├── provider_id
├── connector_type
├── connector_version
├── environment
│
├── supported_capabilities[]
├── approved_markets[]
├── approved_currencies[]
├── approved_payment_methods[]
├── approved_transaction_types[]
│
├── technical_status
├── security_status
├── compliance_status
├── commercial_status
├── reconciliation_status
├── operational_status
├── resilience_status
│
├── evidence[]
├── limitations[]
├── approved_at
├── approved_by
├── effective_from
├── expires_at
└── status
```

The exact canonical schema SHALL be defined in Shared payment contracts where cross-engine consumption is required.

---

# 8. Certification Dimensions

A provider SHALL NOT reach `CERTIFIED` production status until all mandatory applicable dimensions have passed.

## 8.1 Technical Certification

Technical certification SHALL verify applicable operations including:

```text
connectivity
authentication
payment intent creation
authorisation
capture
cancellation / void
refund
webhook delivery
idempotency
duplicate handling
timeout behaviour
retry behaviour
error normalisation
correlation
provider-reference persistence
```

Where supported, certification SHOULD also test:

```text
partial capture
partial refund
3DS / customer action
asynchronous payment methods
recurring credentials
tokenised methods
disputes
payouts
```

A capability that has not been certified SHALL NOT be inferred from another capability.

For example:

```text
CARD_PAYMENT certified

does not imply

REFUND certified
PAYOUT certified
RECURRING certified
```

---

# 9. Security Certification

Security certification SHALL establish that the provider integration conforms to applicable Baobab security architecture.

At minimum it SHALL examine:

```text
credential storage
credential rotation
workload authentication
TLS requirements
webhook verification
secret exposure
logging hygiene
sensitive-data handling
tokenisation
least privilege
administrative access
auditability
incident revocation
```

Where cardholder or similarly sensitive payment data is involved, certification SHALL additionally conform to ADR-PAY-0012.

Production credentials SHALL NOT be stored in:

```text
source code
Git repositories
digital-estate configuration
frontend applications
ordinary environment templates
logs
events
canonical payment contracts
```

---

# 10. Regulatory and Compliance Certification

Technical support SHALL NOT be interpreted as regulatory permission.

For every production market, certification SHALL record evidence that the proposed provider relationship and payment capability have undergone the applicable legal, regulatory, contractual and compliance review required by the responsible organisation.

Baobab itself SHALL NOT attempt to infer regulatory legality solely from provider documentation or HyperSwitch connector availability.

The certification record SHALL therefore distinguish:

```text
TECHNICALLY_SUPPORTED

from

APPROVED_FOR_PRODUCTION_USE
```

Where regulatory or contractual status is unknown, the provider SHALL remain non-production-routable.

---

# 11. Commercial Certification

Production activation SHALL verify applicable commercial arrangements.

These MAY include:

```text
merchant agreement
pricing
transaction fees
refund fees
chargeback fees
settlement fees
FX fees
minimum commitments
reserve requirements
settlement periods
payout schedules
service levels
support channels
termination conditions
```

Commercial certification SHALL NOT be encoded inside HyperSwitch routing rules.

Routing MAY consume resulting eligibility or cost metadata where permitted by ADR-PAY-0009.

---

# 12. Settlement and Reconciliation Certification

A provider SHALL NOT be considered production-ready merely because it can successfully capture funds.

Before production activation, Baobab SHALL establish how financial outcomes can be reconciled.

Certification SHALL therefore test, where applicable:

```text
payment reference
processor reference
merchant reference
settlement reference
refund reference
fee information
settlement amount
settlement currency
settlement date
reconciliation reports
webhook / API / file availability
ERP mapping
exception handling
```

The governing principle is:

> **If Baobab can move money through a provider but cannot reliably establish what happened to that money, the provider is not production-ready.**

Accounting authority remains with `baobab-erp`.

ADR-PAY-0015 SHALL define the canonical settlement and reconciliation architecture.

---

# 13. Operational Certification

Operational readiness SHALL include:

```text
provider support contacts
incident escalation path
credential recovery procedure
provider status visibility
known failure modes
runbooks
manual reconciliation procedure
refund procedure
dispute procedure
provider suspension procedure
routing-disable procedure
```

At least one documented emergency mechanism SHALL exist for preventing new transactions from reaching a compromised or malfunctioning provider.

---

# 14. Resilience Certification

Where routing failover or multiple providers are expected, certification SHALL test provider behaviour under:

```text
timeouts
5xx responses
connection failures
rate limits
provider degradation
webhook delay
duplicate webhook delivery
out-of-order webhook delivery
partial provider outage
```

Failover SHALL comply with ADR-PAY-0009.

A retry or fallback SHALL never create duplicate financial execution.

---

# 15. Certification States

The canonical lifecycle SHALL include at least:

```text
DISCOVERED
    │
    ▼
INTEGRATION_PENDING
    │
    ▼
INTEGRATED
    │
    ▼
CERTIFICATION_PENDING
    │
    ├────────────► CERTIFICATION_FAILED
    │
    ▼
CERTIFIED
    │
    ├────────────► SUSPENDED
    │
    ├────────────► EXPIRED
    │
    └────────────► REVOKED
```

Meaning:

| State | Meaning |
|---|---|
| `DISCOVERED` | Provider/connector is known |
| `INTEGRATION_PENDING` | Integration work has begun |
| `INTEGRATED` | Technical connection exists |
| `CERTIFICATION_PENDING` | Production certification is underway |
| `CERTIFICATION_FAILED` | Mandatory certification requirement failed |
| `CERTIFIED` | Approved for the explicitly recorded scope |
| `SUSPENDED` | Temporarily unavailable for production admission |
| `EXPIRED` | Certification validity ended |
| `REVOKED` | Certification withdrawn |

Only:

```text
CERTIFIED
```

may support a new production activation.

---

# 16. Activation Lifecycle

Provider certification and provider activation SHALL remain separate lifecycles.

A certified provider is not automatically active.

The activation lifecycle SHALL include at least:

```text
PROPOSED
    │
    ▼
VALIDATING
    │
    ├────────► BLOCKED
    │
    ▼
APPROVED
    │
    ▼
ACTIVE
    │
    ├────────► SUSPENDED
    │
    ├────────► DEACTIVATED
    │
    └────────► EXPIRED
```

An activation SHALL become `ACTIVE` only when its referenced certification is valid.

---

# 17. Certification Versus Activation

The relationship is:

```text
                    Provider
                       │
                       ▼
                  Certification
                       │
            ┌──────────┼──────────┐
            │          │          │
            ▼          ▼          ▼
         ZA scope   UG scope   future scope
            │
            ▼
        Activation
            │
     ┌──────┼─────────┐
     │      │         │
     ▼      ▼         ▼
 Legal   Market   Currency /
 Entity           Method scope
```

A certification MAY support multiple activations where its approved scope permits.

An activation SHALL NOT broaden its parent certification.

For example:

```text
Certification:
markets = [ZA]
currencies = [ZAR]
methods = [CARD]
```

cannot produce:

```text
Activation:
market = UG
currency = UGX
method = MOBILE_MONEY
```

Such activation SHALL be rejected.

---

# 18. Legal-Entity Boundary

Provider activation SHALL be legal-entity-aware.

This is mandatory because:

```text
Nabhold Group Africa
ZuriBeans
Thamani Global
Equator & Estate Co.
future Baobab customers
```

may be separate contracting and merchant entities even when they consume the same Baobab Payments capability.

Therefore:

```text
Provider X active for Thamani ZA

≠

Provider X active for ZuriBeans ZA
```

unless both activations have been independently established or an explicitly governed provider arrangement lawfully covers both.

No subsidiary relationship SHALL implicitly inherit payment-provider activation.

---

# 19. Tenant and External-Customer Isolation

Future external Baobab customers may have their own:

```text
organisations
subsidiaries
legal entities
markets
merchant agreements
provider credentials
settlement accounts
```

Provider certification MAY be reusable at the platform technical level where appropriate.

Provider activation SHALL remain isolated to the authorised tenant/legal-entity context.

Therefore:

```text
Baobab has certified Provider X
```

does not mean:

```text
Every Baobab customer may use Provider X.
```

---

# 20. Market Boundary

A provider SHALL be activated explicitly for a Baobab Market.

Market activation SHALL use the canonical Market identity resolved by the Control Plane.

The Payments Engine SHALL NOT create competing market definitions.

Conceptually:

```text
baobab-cp
    │
    ▼
Canonical Market
    │
    ▼
Payment Provider Activation
    │
    ▼
HyperSwitch configuration / profile mapping
```

HyperSwitch profile or merchant configuration SHALL therefore project Baobab market context rather than redefine it.

---

# 21. Currency Boundary

Market activation SHALL NOT imply universal currency support.

Eligibility SHALL evaluate the transaction currency separately.

For example:

```text
market             = ZA
provider active    = YES
currency requested = USD
```

does not imply the transaction is eligible merely because:

```text
ZAR transactions are certified.
```

Currency eligibility SHALL be explicit.

FX behaviour is governed by ADR-PAY-0017.

---

# 22. Payment-Method Boundary

Payment-method support SHALL also be explicit.

Representative methods include:

```text
CARD
BANK_TRANSFER
MOBILE_MONEY
WALLET
ACCOUNT_TO_ACCOUNT
DIRECT_DEBIT
BUY_NOW_PAY_LATER
other future methods
```

The canonical vocabulary SHALL be governed by Baobab contracts rather than copied blindly from provider-specific values.

Provider-specific method identifiers SHALL be translated at the Payments boundary.

---

# 23. Transaction-Type Boundary

Certification SHALL distinguish transaction capabilities.

Representative capabilities include:

```text
AUTHORIZE
CAPTURE
SALE
VOID
REFUND
PARTIAL_REFUND
RECURRING
TOKENIZE
PAYOUT
DISPUTE
SETTLEMENT_REPORTING
```

A provider SHALL participate only in operations for which it is explicitly eligible.

---

# 24. Commercial-Model Boundary

Where material, activation MAY restrict provider use according to commercial model:

```text
B2B
B2C
SUBSCRIPTION
MARKETPLACE
PLATFORM
```

This permits Baobab to support different provider arrangements without encoding digital-estate names directly into payment routing logic.

For example, the fact that ZuriBeans currently operates B2B and Thamani currently operates B2C SHALL NOT result in hard-coded logic such as:

```text
if estate == "zuribeans":
    ...
```

The payment architecture SHALL reason from canonical business context and activation policy.

---

# 25. Environment Separation

Certification SHALL distinguish at minimum:

```text
SANDBOX
PRODUCTION
```

Sandbox success SHALL never imply production certification.

Sandbox credentials and production credentials SHALL be isolated.

The existing Baobab `SandboxProvider` remains explicitly non-production.

Records and events generated by simulation SHALL remain marked:

```text
simulated = true
```

as established by ADR-SHARED-011 and ADR-PAY-0001.

A simulated transaction SHALL never satisfy production settlement evidence.

---

# 26. Provider Activation Resolution

Before initiating real money movement, `baobab-payments` SHALL resolve an eligible provider set.

Conceptually:

```text
Payment Request
      │
      ▼
Resolved Baobab Context
      │
      ├── tenant
      ├── legal entity
      ├── market
      ├── digital estate
      └── currency
      │
      ▼
Required Payment Capability
      │
      ▼
Provider Certifications
      │
      ▼
Legal-Entity Activations
      │
      ▼
Market Eligibility
      │
      ▼
Currency Eligibility
      │
      ▼
Payment-Method Eligibility
      │
      ▼
Transaction-Type Eligibility
      │
      ▼
Eligible Provider Set
      │
      ▼
Routing Policy
      │
      ▼
HyperSwitch
```

Routing SHALL occur only after eligibility has been established.

---

# 27. Fail-Closed Behaviour

Provider eligibility SHALL fail closed.

If Baobab cannot establish a valid production activation, the payment SHALL NOT be sent to that provider.

Examples include:

```text
certification missing
certification expired
certification suspended
activation missing
activation expired
market mismatch
currency mismatch
method mismatch
legal-entity mismatch
environment mismatch
capability not certified
credentials unavailable
provider administratively disabled
```

The system SHALL prefer:

```text
NO PAYMENT
```

over:

```text
UNCERTAIN OR UNAUTHORISED PAYMENT
```

where provider eligibility cannot be proven.

---

# 28. No Silent Fallback Across Legal Boundaries

Routing failover SHALL NOT bypass certification.

If:

```text
Provider A
```

fails, HyperSwitch SHALL NOT automatically route to:

```text
Provider B
```

unless Provider B is independently eligible for the exact transaction context.

Therefore:

```text
technical connector availability
```

is never sufficient fallback authority.

---

# 29. Activation and HyperSwitch Configuration

Baobab activation SHALL drive or constrain HyperSwitch configuration.

HyperSwitch configuration SHALL NOT become the authoritative source of Baobab activation state.

The direction of authority is:

```text
Baobab governed state
        │
        ▼
Provider Activation
        │
        ▼
HyperSwitch configuration
```

not:

```text
HyperSwitch configuration
        │
        ▼
infer Baobab governance
```

Configuration drift SHALL be detectable.

---

# 30. Drift Detection

Baobab SHALL detect discrepancies such as:

```text
Baobab activation = INACTIVE
HyperSwitch connector = ENABLED
```

or:

```text
Baobab permits ZAR only
HyperSwitch profile permits ZAR + USD
```

or:

```text
Baobab certification revoked
HyperSwitch routing still references provider
```

Such discrepancies SHALL create operational drift.

Production readiness SHOULD fail where material payment-configuration drift exists.

---

# 31. Controlled Mutation

Provider certification and production activation are privileged operations.

They SHALL NOT be mutable by ordinary:

```text
digital-estate users
commerce administrators
customer-service users
supplier users
buyer users
application workloads
```

Mutations SHALL require explicit privileged scopes governed by Baobab IAM and controlled-mutation architecture.

At minimum, the future authorisation contract SHOULD distinguish permissions comparable to:

```text
payment-provider:read
payment-provider:certify
payment-provider:activate
payment-provider:suspend
payment-provider:revoke
```

Exact scope names SHALL be defined through the canonical authorisation registry.

---

# 32. Separation of Duties

Where practical, Baobab SHOULD prevent the same actor from unilaterally:

```text
configure production credentials
+
certify provider
+
activate provider
+
modify routing
```

Payment-provider admission affects real money movement and therefore warrants stronger governance than ordinary application configuration.

High-risk production changes SHOULD support maker-checker or equivalent approval semantics.

---

# 33. Evidence

Certification SHALL be evidence-backed.

Evidence MAY include:

```text
automated test results
integration-test reports
provider agreements
security review
compliance approval
market review
credential verification
webhook certification
refund test
reconciliation test
settlement test
incident runbook
DR test
provider documentation reference
approval record
```

Evidence SHALL be referenced rather than unnecessarily duplicated.

Sensitive evidence SHALL remain in its authoritative protected system.

---

# 34. Expiry and Recertification

Certification SHALL support expiration.

Recertification SHOULD be required following material changes including:

```text
provider contract change
connector major-version change
HyperSwitch major behavioural change
credential model change
payment-method addition
new market
new currency
new transaction capability
PCI-boundary change
settlement model change
major provider API migration
significant security incident
material regulatory change
```

Not every software patch requires full recertification.

The change classification and resulting certification depth SHALL be risk-based and auditable.

---

# 35. Emergency Suspension

Baobab SHALL support rapid provider suspension.

Suspension SHALL:

```text
prevent new eligible routing
preserve historical records
preserve reconciliation capability
preserve refund/dispute handling where safe
emit an auditable state change
```

Suspending new payments SHALL NOT erase existing payment obligations or financial history.

Emergency suspension SHOULD be possible without deploying new application code.

---

# 36. Existing Transaction Handling

Provider deactivation SHALL distinguish:

```text
NEW TRANSACTIONS
```

from:

```text
IN-FLIGHT TRANSACTIONS
REFUNDS
DISPUTES
SETTLEMENTS
RECONCILIATION
```

A provider may be prohibited from accepting new transactions while Baobab must continue processing asynchronous outcomes for historical transactions.

Therefore:

```text
routing disabled
```

does not mean:

```text
integration deleted
```

Provider mappings and credentials required for lawful completion or reconciliation of historical transactions SHALL be handled according to retention and security policy.

---

# 37. Events

Provider-governance state changes SHOULD emit canonical events.

Representative events include:

```text
payment.provider.discovered.v1
payment.provider.integrated.v1
payment.provider.certified.v1
payment.provider.certification_failed.v1
payment.provider.certification_suspended.v1
payment.provider.certification_revoked.v1

payment.provider.activation_proposed.v1
payment.provider.activated.v1
payment.provider.suspended.v1
payment.provider.deactivated.v1
payment.provider.activation_expired.v1
```

Events SHALL carry canonical identifiers and governance state.

They SHALL NOT expose:

```text
API keys
secrets
cardholder data
provider credentials
unnecessary commercially sensitive data
```

---

# 38. Audit

Every certification and activation mutation SHALL record sufficient audit information to answer:

```text
Who changed it?
What changed?
When?
Under which tenant/legal-entity context?
Which provider?
Which market?
Which certification?
Why?
Which evidence supported it?
Which approval authorised it?
What was the previous state?
What is the resulting state?
```

Payment-provider governance SHALL be reconstructable after the fact.

---

# 39. Readiness

Baobab production readiness SHALL treat provider certification as an explicit gate.

Representative readiness result:

```text
PAYMENT READINESS
│
├── Payment API                   PASS
├── HyperSwitch Adapter           PASS
├── Provider Integration          PASS
├── Security Certification        PASS
├── Market Certification          PASS
├── Legal-Entity Activation       PASS
├── Currency Eligibility          PASS
├── Payment Method                PASS
├── Refund Path                   PASS
├── Webhooks                      PASS
├── Reconciliation               PASS
├── Operational Runbook           PASS
└── Production Routing            READY
```

A more realistic incomplete example:

```text
PAYMENT READINESS
│
├── Payment API                   PASS
├── HyperSwitch Adapter           PASS
├── Sandbox Provider              PASS
├── Production Provider           NOT CERTIFIED
└── Production Routing            BLOCKED
```

The latter SHALL NOT be reported as production payment readiness.

---

# 40. Initial ZuriBeans and Thamani Implication

Initial activation SHALL be evaluated independently for each relevant legal entity and market.

Conceptually:

```text
                    Provider X
                        │
                  CERTIFICATION
                        │
             ┌──────────┴──────────┐
             │                     │
             ▼                     ▼
         South Africa            Uganda
             │                     │
       ┌─────┴─────┐         ┌─────┴─────┐
       │           │         │           │
       ▼           ▼         ▼           ▼
 ZuriBeans     Thamani   ZuriBeans    Thamani
 activation   activation activation   activation
```

The existence of one branch SHALL NOT imply any other branch.

This supports independent subsidiary operations even where subsidiaries consume common Baobab infrastructure.

---

# 41. Future Market Expansion

Expansion into markets such as:

```text
Kenya
Tanzania
Rwanda
other African markets
global markets
```

SHALL NOT require redesign of the payment architecture.

Instead, expansion SHALL consist of governed additions to:

```text
canonical Market
provider certification
legal-entity activation
currency eligibility
payment-method eligibility
routing policy
```

The architecture therefore scales by adding governed configuration and evidence rather than hard-coded market branches.

---

# 42. External Tenant Expansion

When Baobab onboards an external customer, provider activation SHALL operate through the same model.

For example:

```text
External Client
    │
    ├── Subsidiary A
    │      ├── ZA
    │      └── UG
    │
    └── Subsidiary B
           └── KE
```

Each relevant legal-entity/market combination can establish its own authorised provider activations without confusing the external customer's corporate hierarchy with Nabhold Group's hierarchy.

---

# 43. Relationship to PAY-0009

ADR-PAY-0009 shall govern:

```text
routing
connector eligibility
failover
retry selection
```

PAY-0022 establishes the admission boundary upon which PAY-0009 operates.

Therefore:

```text
PAY-0022
determines WHO MAY PARTICIPATE

PAY-0009
determines WHICH ELIGIBLE PROVIDER IS SELECTED
```

Routing SHALL never widen certification.

---

# 44. Relationship to PAY-0012

ADR-PAY-0012 shall govern:

```text
PCI boundary
tokenisation
vaulting
sensitive payment data
```

Provider certification SHALL consume the resulting security requirements.

PAY-0022 SHALL NOT redefine the PCI architecture.

---

# 45. Relationship to PAY-0015

ADR-PAY-0015 shall govern:

```text
settlement
reconciliation
ERP projection
accounting handoff
```

PAY-0022 requires successful reconciliation certification before a provider may be considered fully production-ready where settlement is applicable.

---

# 46. Relationship to PAY-0017

ADR-PAY-0017 shall govern:

```text
FX
cross-border payment semantics
currency conversion
source/destination currency
rate provenance
```

PAY-0022 merely requires that cross-border and FX capabilities be explicitly certified rather than inferred.

---

# 47. Relationship to PAY-0018

ADR-PAY-0018 shall govern:

```text
IAM
privileged payment administration
controlled mutation
workload identity
```

PAY-0022 establishes that provider certification, activation, suspension and revocation are privileged payment-governance operations.

---

# 48. Relationship to PAY-0021

ADR-PAY-0021 shall govern HyperSwitch:

```text
extensions
upgrades
forking
version changes
```

Material HyperSwitch changes SHALL trigger certification impact assessment under this ADR.

---

# 49. Implementation Requirements

Implementation SHALL eventually provide at least:

```text
Provider Registry
Certification Registry
Activation Registry
Eligibility Resolver
Certification Evidence References
Administrative Mutation API
Read API
Audit Trail
Drift Detection
Readiness Projection
Canonical Events
```

The implementation MAY evolve incrementally.

The architectural authority boundaries defined by this ADR apply from acceptance.

---

# 50. Conceptual Data Model

```text
PaymentProvider
      │
      ├───────────────┐
      │               │
      ▼               ▼
Connector       ProviderCertification
                      │
                      │ 1..n
                      ▼
               ProviderActivation
                      │
          ┌───────────┼───────────┐
          │           │           │
          ▼           ▼           ▼
     LegalEntity    Market     Environment
                                  │
                                  ▼
                         Eligibility Rules
                                  │
                   ┌──────────────┼──────────────┐
                   ▼              ▼              ▼
               Currency     Payment Method   Capability
                                  │
                                  ▼
                          Routing Eligibility
                                  │
                                  ▼
                             HyperSwitch
```

---

# 51. Certification Decision Flow

```text
Provider identified
       │
       ▼
Connector available?
       │
   NO ─┴─ YES
   │       │
 BLOCK     ▼
      Integrate connector
             │
             ▼
      Technical tests pass?
             │
        NO ──┴── YES
        │         │
       BLOCK      ▼
             Security approved?
                    │
               NO ──┴── YES
               │         │
              BLOCK      ▼
                   Market/compliance approved?
                           │
                      NO ──┴── YES
                      │         │
                     BLOCK      ▼
                    Commercial approved?
                                  │
                             NO ──┴── YES
                             │         │
                            BLOCK      ▼
                       Reconciliation proven?
                                      │
                                 NO ──┴── YES
                                 │         │
                                BLOCK      ▼
                              CERTIFIED
                                      │
                                      ▼
                         Legal-entity activation
                                      │
                                      ▼
                         Market activation valid?
                                      │
                                 NO ──┴── YES
                                 │         │
                                BLOCK      ▼
                              ROUTABLE
```

---

# 52. Invariants

The following invariants SHALL hold:

1. **No connector implies certification.**
2. **No certification implies activation.**
3. **No activation implies routing eligibility.**
4. **Certification scope cannot be widened by activation.**
5. **Activation cannot override canonical Control Plane context.**
6. **A provider active for one legal entity is not implicitly active for another.**
7. **A provider active in one market is not implicitly active in another.**
8. **Currency support is explicit.**
9. **Payment-method support is explicit.**
10. **Transaction capability support is explicit.**
11. **Sandbox success is not production certification.**
12. **Routing cannot bypass certification.**
13. **Failover cannot bypass certification.**
14. **HyperSwitch configuration is not canonical Baobab governance state.**
15. **Revocation prevents new routing.**
16. **Historical financial records survive deactivation.**
17. **Accounting authority remains with ERP.**
18. **Payment obligations remain with their originating commercial engine.**
19. **Secrets never become canonical contract data.**
20. **Unknown eligibility fails closed.**

---

# 53. Consequences

## Positive

This decision provides:

- controlled production admission of payment providers;
- explicit legal-entity isolation;
- explicit market isolation;
- explicit currency and payment-method eligibility;
- safer multi-provider routing;
- safer failover;
- support for independent Nabhold subsidiaries;
- support for external Baobab customers;
- auditable provider governance;
- stronger regulatory and operational controls;
- clear separation between HyperSwitch configuration and Baobab authority;
- repeatable expansion into additional markets;
- honest production-readiness reporting.

## Negative

The decision introduces:

- additional governance records;
- certification workflows;
- evidence management;
- additional Control Plane/Payments integration;
- provider lifecycle administration;
- recertification obligations;
- drift detection requirements;
- more operational discipline before a provider can be used.

These costs are accepted because uncontrolled activation of payment providers creates unacceptable financial, regulatory, security and reconciliation risk.

---

# 54. Alternatives Considered

## HyperSwitch configuration as the source of truth

Rejected.

HyperSwitch configuration represents payment-engine implementation state, not Baobab tenant, legal-entity, market or governance authority.

---

## Any configured connector is automatically routable

Rejected.

Technical connectivity does not establish legal, commercial, security, settlement or operational readiness.

---

## Certification by provider only

Rejected.

A provider's statement that it supports a country, currency or payment method does not establish that a particular Baobab legal entity is authorised and operationally ready to use it.

---

## Digital estates choose their own providers

Rejected.

This would recreate direct provider coupling, fragment credentials and routing logic, and bypass platform governance.

---

## One global provider configuration for all Baobab tenants

Rejected.

Baobab supports independent legal entities and future external customers with different contracts, markets, credentials, settlement arrangements and compliance obligations.

---

# 55. Final Decision

Baobab SHALL treat payment-provider production admission as a governed lifecycle:

```text
CONNECTOR
    │
    ▼
INTEGRATION
    │
    ▼
CERTIFICATION
    │
    ▼
LEGAL-ENTITY ACTIVATION
    │
    ▼
MARKET ACTIVATION
    │
    ▼
CURRENCY / METHOD / CAPABILITY ELIGIBILITY
    │
    ▼
ROUTING ELIGIBILITY
    │
    ▼
HYPERSWITCH
    │
    ▼
PAYMENT PROVIDER
```

HyperSwitch connector availability SHALL never constitute production authorisation.

A provider SHALL participate in real Baobab money movement only when Baobab can prove that:

```text
the provider is certified
AND
the certification is current
AND
the legal entity is authorised
AND
the market is authorised
AND
the environment is production
AND
the currency is eligible
AND
the payment method is eligible
AND
the requested capability is certified
AND
the provider is operationally enabled
AND
the transaction satisfies applicable routing constraints.
```

Where any required fact cannot be established:

```text
ROUTING ELIGIBILITY = DENIED
```

**Baobab certifies the provider.  
Baobab activates the business context.  
HyperSwitch routes only among providers Baobab has already authorised.**