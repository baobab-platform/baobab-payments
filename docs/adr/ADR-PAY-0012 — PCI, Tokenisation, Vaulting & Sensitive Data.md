# ADR-PAY-0012 — PCI, Tokenisation, Vaulting & Sensitive Data

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Repository** | `baobab-platform/baobab-payments` |
| **Decision Type** | Architecture / Security / Payment Data Protection |
| **Scope** | Baobab Platform |
| **Depends on** | ADR-PAY-0001 through ADR-PAY-0011; ADR-PAY-0022; ADR-SHARED-011; applicable Control Plane, IAM, Shared and Infrastructure decisions |
| **Related** | ADR-PAY-0013; ADR-PAY-0014; ADR-PAY-0015; ADR-PAY-0016; ADR-PAY-0017; ADR-PAY-0018; ADR-PAY-0019; ADR-PAY-0020; ADR-PAY-0021 |
| **Primary External Standard** | PCI DSS v4.0.1 |

---

# 1. Context

`baobab-payments` executes payment operations through external payment infrastructure.

The intended architecture is:

```text
Digital Estate
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

Some payment methods require sensitive credentials.

For cards these may include:

```text
Primary Account Number
Cardholder name
Expiration date
Service code
Card verification value
Authentication data
```

Other payment methods introduce different sensitive material:

```text
bank account details
mobile-money identifiers
wallet credentials
mandate references
authentication tokens
provider secrets
beneficiary information
```

The architecture must therefore distinguish:

```text
CANONICAL PAYMENT DATA

PAYMENT METHOD REFERENCE

TOKEN

CARDHOLDER DATA

SENSITIVE AUTHENTICATION DATA

PROVIDER CREDENTIAL

SECRET

PERSONAL DATA
```

They are not equivalent.

---

# 2. Problem

A naïve payment architecture might allow:

```text
Browser
   │
   ▼
Trade
   │
   ▼
Baobab Payments
   │
   ▼
HyperSwitch
```

while raw card credentials pass through every component.

That would unnecessarily increase:

- attack surface;
- PCI DSS scope;
- breach impact;
- logging risk;
- database risk;
- debugging risk;
- operational complexity;
- compliance burden.

The platform instead requires deliberate minimisation of sensitive payment data.

---

# 3. Decision

Baobab SHALL adopt a:

> **TOKEN-FIRST, REFERENCE-FIRST, DATA-MINIMISING PAYMENT ARCHITECTURE.**

Raw payment credentials SHALL NOT traverse or persist in Baobab services unless technically necessary for a specifically approved integration.

Where supported, sensitive payment credentials SHALL move directly from the trusted payment-capture surface to the approved payment/tokenisation infrastructure.

Baobab business engines SHALL primarily operate on:

```text
canonical payment IDs
payment-method references
opaque tokens
safe display metadata
provider references
canonical financial facts
```

rather than raw credentials.

---

# 4. Governing Principle

> **If Baobab does not need sensitive payment data, Baobab must not receive it. If it does not need to retain it, Baobab must not store it.**

The preferred architecture is:

```text
Customer
   │
   ▼
Secure Payment Capture
   │
   ▼
Approved Tokenisation / Payment Infrastructure
   │
   ├── sensitive credential
   │
   ▼
Token / Safe Reference
   │
   ▼
Baobab Payments
```

not:

```text
Customer
   │
   ▼
Trade
   │
   ▼
Baobab Payments DB
   │
   ▼
HyperSwitch
```

with raw credentials persisted along the path.

---

# 5. Compliance Is a System Property

Using HyperSwitch, tokenisation or a PCI-compliant PSP does not automatically make Baobab compliant.

Compliance depends upon the actual:

```text
data flow
system boundary
deployment
configuration
network
storage
access controls
people
processes
integrations
operational practices
```

Therefore this ADR defines architectural controls.

It does not constitute a PCI DSS compliance certification.

---

# 6. PCI DSS Baseline

Production card-processing architecture SHALL be assessed against the then-current applicable PCI DSS requirements.

At acceptance of this ADR, the baseline is:

```text
PCI DSS v4.0.1
```

The applicable compliance obligations SHALL be re-evaluated when:

- architecture changes;
- payment capture changes;
- providers change;
- tokenisation changes;
- hosting changes;
- PCI standards change;
- new payment channels are introduced.

---

# 7. Compliance Scope Minimisation

Baobab SHALL deliberately minimise the number of components that:

```text
store
process
transmit
or can materially affect the security of
```

payment account data.

Scope reduction SHALL be an explicit architecture objective.

Scope reduction SHALL NOT be confused with exemption from applicable compliance obligations.

---

# 8. Data Classification

Payment-related information SHALL be classified.

At minimum:

| Classification | Examples | General Handling |
|---|---|---|
| Canonical payment data | Payment ID, amount, currency, state | Normal protected financial data |
| Payment method metadata | Card brand, masked digits, method category | Protected |
| Payment method reference | Token/reference to external credential | Highly controlled |
| Cardholder data | PAN and associated cardholder data | PCI-controlled |
| Sensitive authentication data | CVV/CVC/CID, track data, PIN-related data | Strictly restricted |
| Provider credential | API keys, signing keys, merchant secrets | Secret-management controls |
| Bank/payment credentials | Bank account/payment access data | Restricted |
| Personal data | Name, contact details, payer data | Privacy controls |
| Audit/security data | Security events, access records | Protected/audited |

Classification SHALL be refined through implementation security specifications.

---

# 9. PAN

The Primary Account Number is sensitive payment account data.

Baobab SHALL avoid receiving or storing PAN wherever technically possible.

---

# 10. PAN Storage

No Baobab engine SHALL persist full PAN by default.

Any future requirement to store PAN SHALL require:

```text
explicit architecture review
PCI scope analysis
security review
documented business necessity
approved encryption/tokenisation design
retention policy
access-control design
audit controls
```

and, where material, a new or amended ADR.

---

# 11. PAN in Canonical Contracts

Canonical Baobab payment contracts SHALL NOT require PAN.

Therefore:

```text
PaymentIntent
Payment
PaymentAttempt
Refund
Settlement
```

SHALL remain provider- and credential-independent.

---

# 12. Sensitive Authentication Data

Sensitive authentication data SHALL receive stricter handling than ordinary payment metadata.

Card verification codes such as:

```text
CVV
CVC
CID
CAV2
```

SHALL NOT be stored after authorization.

Encryption does not convert prohibited retention into acceptable retention.

---

# 13. CVV

Baobab SHALL NOT persist CVV/CVC/CID.

This includes:

```text
application database
cache
session store
event payload
webhook record
message broker
DLQ
trace
log
analytics store
backup
data lake
debug dump
```

---

# 14. Logging CVV

Logging card verification data is prohibited.

Application instrumentation SHALL be designed so such fields cannot accidentally enter structured logs.

---

# 15. Track Data

Full magnetic-stripe or equivalent track data SHALL NOT be retained after authorization except where a specifically applicable PCI rule lawfully permits an entity with the appropriate role.

Baobab's normal merchant/payment-orchestration architecture SHALL assume it is prohibited.

---

# 16. PIN Data

PIN and PIN-block processing SHALL not be introduced into ordinary Baobab application services.

If a future payment capability requires PIN processing, that capability requires separate security architecture and applicable payment-security standards.

---

# 17. Payment Credential Capture

The preferred browser payment flow is:

```text
Customer Browser
       │
       ▼
Approved Secure Payment Component
       │
       ▼
Payment / Tokenisation Infrastructure
       │
       ▼
Opaque Reference
       │
       ▼
Baobab Payments
```

The objective is to prevent raw credentials from traversing:

```text
ZuriBeans application server
Thamani application server
baobab-trade
baobab-cp
baobab-cms
baobab-pulse
baobab-erp
baobab-subscriptions
```

where technically feasible.

---

# 18. Leaf Digital Estates

Leaf estates SHALL NOT become payment credential vaults.

For example:

```text
ZuriBeans
Thamani
future customer estates
```

SHALL consume payment UX capabilities without becoming repositories of raw card credentials.

---

# 19. Trade Boundary

`baobab-trade` SHALL NOT store raw payment credentials merely because it owns checkout.

Trade owns:

```text
cart
checkout
order
commercial payment projection
```

Payments owns:

```text
payment execution
payment-method orchestration
payment credential reference handling
```

subject to the external vault/tokenisation boundary.

---

# 20. Control Plane Boundary

`baobab-cp` SHALL NOT receive or store:

```text
PAN
CVV
bank credentials
payment tokens requiring no CP function
provider API secrets
```

Control Plane governs business context and capability resolution.

It is not a payment credential store.

---

# 21. ERP Boundary

ERP SHALL receive accounting-relevant payment facts.

It SHALL NOT receive raw card credentials.

Example:

```text
Payment ID
Provider Reference
Amount
Currency
Settlement Reference
Fee
Accounting Context
```

may be legitimate.

```text
PAN
CVV
```

are not.

---

# 22. Pulse Boundary

`baobab-pulse` SHALL NOT ingest raw payment credentials for analytics or AI.

Payment analytics SHALL operate on minimised canonical data.

---

# 23. CMS Boundary

CMS SHALL never be used to store payment credentials.

Content fields SHALL not provide an accidental payment-data storage path.

---

# 24. IAM Boundary

IAM SHALL authenticate and authorise actors.

IAM SHALL NOT become a payment-method vault.

---

# 25. Subscriptions Boundary

Subscriptions MAY retain:

```text
payment method reference
billing account reference
mandate reference
```

where required.

It SHALL NOT retain raw card credentials merely to support recurring billing.

---

# 26. Tokenisation

Tokenisation replaces sensitive payment data with a surrogate reference.

Conceptually:

```text
PAN
 │
 ▼
TOKENISATION SYSTEM
 │
 ├────► protected underlying credential
 │
 ▼
TOKEN
```

Baobab SHOULD operate on the token/reference wherever possible.

---

# 27. Token Is Not PAN

A token SHALL remain semantically distinct from the credential it represents.

Canonical contracts SHALL not pretend that:

```text
token == PAN
```

---

# 28. Token Types

Baobab SHALL recognise that different token types have different security and portability characteristics.

Possible categories include:

```text
provider/acquirer tokens
vault tokens
network/payment tokens
wallet tokens
merchant-scoped tokens
mandate references
bank-account tokens
```

The architecture SHALL NOT assume all tokens are interchangeable.

---

# 29. Token Provenance

A payment-method token/reference SHOULD preserve controlled provenance.

Conceptually:

```text
PaymentMethodReference
│
├── reference_id
├── method_type
├── token_type
├── issuer/provider
├── vault/provider reference
├── tenant scope
├── legal entity scope
├── merchant scope
├── environment
├── portability classification
├── status
└── safe display metadata
```

Exact canonical contracts SHALL be defined separately.

---

# 30. Token Scope

A token may be valid only within a particular:

```text
provider
merchant
Legal Entity
tenant
environment
region
payment method
customer
```

Baobab SHALL NOT assume universal token portability.

---

# 31. Provider Token

A token created by Provider A SHALL not automatically be submitted to Provider B.

Routing SHALL account for token portability.

---

# 32. Routing and Token Portability

PAY-0009 routing eligibility SHALL consider whether the selected Payment Method Reference can actually be used by a candidate provider.

Therefore:

```text
Provider Eligible
      +
Token Compatible
      =
Potential Route
```

Provider certification alone is insufficient.

---

# 33. Failover and Token Portability

Cross-provider failover SHALL NOT assume that a provider-specific token can move between providers.

If:

```text
Provider A token
```

cannot be used by:

```text
Provider B
```

then Provider B is not a valid failover route for that credential unless another approved mechanism exists.

---

# 34. Vault

A vault is the security boundary responsible for protecting underlying sensitive payment credentials and exposing controlled references.

Conceptually:

```text
                BAOBAB
                  │
                  │ opaque reference
                  ▼
            VAULT INTERFACE
                  │
          ┌───────┴────────┐
          │                │
       TOKEN          CONTROLLED USE
          │                │
          └───────┬────────┘
                  ▼
          SENSITIVE CREDENTIAL
```

---

# 35. Vault Authority

The vault owns credential protection.

It does not own:

```text
Tenant
Organisation
Legal Entity
Market
Payment
Order
Subscription
Accounting
```

---

# 36. HyperSwitch Vaulting

Where HyperSwitch or its supported vault architecture provides credential tokenisation/storage, Baobab MAY use it behind the Payments adapter boundary.

The implementation SHALL be evaluated for:

```text
PCI implications
security model
deployment model
token portability
tenant isolation
merchant isolation
data residency
backup
key management
operational recovery
upgrade compatibility
```

before production activation.

---

# 37. No Assumed HyperSwitch Trust

The architectural statement:

```text
HyperSwitch supports vaulting
```

does not itself establish that the selected Baobab deployment is compliant or sufficiently secure.

Actual deployment and configuration SHALL be assessed.

---

# 38. External Vault

Baobab MAY use a provider-managed or specialised external vault where this better reduces sensitive-data exposure.

The canonical architecture SHALL not depend on Baobab implementing its own vault.

---

# 39. Building a Baobab Vault

Baobab SHALL NOT implement a custom raw-card credential vault merely for convenience.

A custom vault would materially increase:

```text
security responsibility
PCI scope
cryptographic responsibility
operational burden
breach consequence
```

and requires a separate architecture decision.

---

# 40. Payment Method Reference

Baobab SHALL introduce or maintain the concept of:

```text
PaymentMethodReference
```

as the canonical pointer to reusable or transaction-specific payment credentials.

It is not itself the credential.

---

# 41. Payment Method Reference Example

Conceptually:

```text
PaymentMethodReference
{
    id
    method_type
    token_type
    external_reference
    provider_or_vault
    tenant_id
    legal_entity_id
    customer_reference?
    environment
    status
    reusable
    portability
    display_metadata
    created_at
}
```

This is conceptual and does not freeze the Shared contract.

---

# 42. Safe Display Metadata

Baobab MAY retain non-sensitive display information where appropriate, such as:

```text
card brand
masked PAN
last four digits
expiration metadata where permitted/needed
bank name
wallet type
mobile-money provider
```

subject to applicable security/privacy requirements.

---

# 43. Masked PAN

Masked PAN MAY be used for customer recognition and operational support where permitted.

It SHALL not become a substitute storage mechanism for full PAN.

---

# 44. Payment Method Ownership

A stored payment method SHALL have explicit canonical ownership/scope.

For example:

```text
Tenant
  │
  ▼
Legal Entity
  │
  ▼
Customer / Account
  │
  ▼
Payment Method Reference
```

where the business model requires customer ownership.

---

# 45. Cross-Tenant Isolation

A payment-method reference belonging to Tenant A SHALL NOT be usable by Tenant B merely because the underlying provider can technically access it.

---

# 46. Cross-Legal-Entity Isolation

A payment method registered under ZuriBeans SHALL NOT automatically become available to Thamani.

The companies are independent Legal Entities.

Shared group ownership does not establish credential-sharing authority.

---

# 47. External Customer Isolation

The same rules apply to external Baobab customers and their subsidiaries.

No Nabhold-specific exception exists.

---

# 48. Merchant Scope

If the underlying token is merchant-scoped, Baobab SHALL preserve that constraint.

A different HyperSwitch Merchant SHALL not use the token unless the provider/vault explicitly supports the arrangement.

---

# 49. Environment Isolation

Sandbox payment references SHALL NOT be usable in production.

Production references SHALL NOT be copied into sandbox/test environments.

---

# 50. Simulated Payment Methods

SandboxProvider payment methods SHALL be synthetic.

They SHALL contain no real payment credentials.

---

# 51. Test Data

Development, CI and test environments SHALL NOT use live payment credentials.

Test fixtures SHALL use:

```text
provider test values
synthetic values
non-sensitive mock references
```

---

# 52. Source Control

Payment credentials SHALL NEVER be committed to Git.

This includes:

```text
PAN
provider API keys
webhook secrets
vault keys
private keys
production tokens
database credentials
```

---

# 53. Example Files

`.env.example`, fixtures and documentation SHALL use obviously non-production placeholders.

---

# 54. Secrets Management

Provider and vault credentials SHALL be stored in an approved secret-management mechanism.

Examples conceptually include:

```text
cloud secret manager
dedicated secrets platform
Kubernetes-integrated secret delivery
HSM-backed key service
```

according to infrastructure architecture.

---

# 55. Environment Variables

Production secrets SHALL not be treated as ordinary configuration.

Environment-variable injection MAY be an implementation transport where appropriately controlled, but secrets SHALL originate from governed secret management.

---

# 56. Secret Ownership

Secrets SHALL be scoped to the minimum required:

```text
environment
engine instance
provider
merchant
Legal Entity
region
```

as applicable.

---

# 57. Shared Credentials

One provider credential SHALL NOT be shared across unrelated Legal Entities merely to simplify deployment.

Any shared credential arrangement requires the provider contract and Baobab governance model to explicitly permit it.

---

# 58. Secret Rotation

Provider credentials and webhook secrets SHALL support rotation.

Rotation SHALL be:

```text
controlled
auditable
tested
recoverable
```

and, where possible, zero-downtime.

---

# 59. Key Management

Cryptographic keys SHALL have defined:

```text
owner
purpose
algorithm
location
access policy
rotation
revocation
backup/recovery
audit
```

---

# 60. Encryption at Rest

Sensitive payment-related data retained by Payments SHALL use appropriate encryption at rest.

Database/storage encryption does not eliminate the need for:

```text
access control
tokenisation
data minimisation
retention control
application security
```

---

# 61. Field-Level Protection

Highly sensitive values that must be retained MAY require field-level/application-level encryption in addition to storage-level encryption.

The need SHALL be determined by classification and threat model.

---

# 62. Encryption in Transit

Payment traffic SHALL use modern protected transport.

Production payment APIs, provider connections, webhook ingress and secret-management connections SHALL use TLS according to current security policy.

---

# 63. Service-to-Service Security

Internal network location SHALL not itself establish trust.

Requests from:

```text
Trade
Subscriptions
ERP
Control Plane
```

to Payments SHALL still require authenticated and authorised service identity.

---

# 64. Least Privilege

Payment data access SHALL follow least privilege.

A workload permitted to:

```text
payment:execute
```

does not automatically require access to:

```text
raw webhook payloads
vault administration
provider secrets
payment-method export
```

---

# 65. IAM

PAY-0018 SHALL define detailed identity and mutation controls.

PAY-0012 establishes that sensitive payment-data access is a privileged security operation.

---

# 66. Human Access

Human access to sensitive payment infrastructure SHALL be exceptional.

Administrative access SHOULD require:

```text
strong authentication
MFA
least privilege
time-bound access where appropriate
audit
```

---

# 67. No Routine Database Browsing

Production payment databases SHALL not be treated as routinely browsable application data stores.

Operational tooling SHOULD expose safe, masked views instead.

---

# 68. Support Staff

Customer-support interfaces SHALL show only the minimum payment information required to resolve support cases.

For example:

```text
Payment ID
status
amount
currency
method category
masked display
provider reference where authorised
```

not raw credentials.

---

# 69. Developer Access

Developers SHALL not require routine access to production payment credentials.

Debugging SHALL rely on:

```text
canonical IDs
correlation IDs
masked metadata
traces
safe provider references
```

---

# 70. Logs

Logs SHALL be treated as a high-risk exfiltration surface.

Sensitive-data filtering SHALL occur before log emission where possible.

---

# 71. Log Redaction

Logging infrastructure SHALL redact or suppress recognised sensitive fields.

Representative protected names include:

```text
pan
card_number
cvv
cvc
cid
security_code
track_data
account_number
secret
api_key
private_key
```

Exact rules SHALL evolve with integrations.

---

# 72. Structured Logging

Structured logging is preferred because known fields can be systematically classified and redacted.

Developers SHALL NOT serialize entire provider request/response objects into logs by default.

---

# 73. Debug Logging

Production debug mode SHALL NOT be permitted to expose sensitive payment payloads.

Temporary debugging SHALL not justify disabling data-protection controls.

---

# 74. Error Messages

Errors returned to clients SHALL not expose:

```text
raw credentials
provider secrets
vault details
internal keys
sensitive provider responses
```

---

# 75. Tracing

Distributed traces SHALL carry:

```text
payment_id
payment_attempt_id
correlation_id
provider identifier
merchant/profile references where safe
```

but not raw credentials.

---

# 76. Metrics

Metrics SHALL not use sensitive payment values as labels.

For example:

```text
PAN
customer bank account
payment token
```

SHALL NOT become metric dimensions.

---

# 77. Events

Canonical events SHALL not contain raw payment credentials.

PAY-0011's event plane SHALL propagate financial facts, not credential material.

---

# 78. Webhooks

Raw webhook payloads MAY contain sensitive provider data.

They SHALL therefore be:

```text
verified
minimised
protected
retained only as necessary
access-controlled
```

---

# 79. Webhook Persistence

If raw webhook persistence is not required for security, dispute or recovery reasons, the system SHOULD persist only the validated/normalised information necessary for processing.

---

# 80. DLQ

Dead-letter queues SHALL not become uncontrolled repositories of raw sensitive payloads.

Payload minimisation/redaction applies before or within quarantine design.

---

# 81. Message Broker

Sensitive payment credentials SHALL not be propagated through the event broker.

Canonical event payloads SHALL remain credential-minimised.

---

# 82. Cache

Raw payment credentials SHALL NOT be cached in general-purpose application caches.

---

# 83. Redis

If Redis is used by HyperSwitch or Baobab Payments, it SHALL NOT be assumed safe for arbitrary credential storage merely because it is internal.

Its use SHALL follow the relevant component's approved security design.

---

# 84. Analytics

Payment analytics SHALL operate on canonical financial facts and appropriately minimised dimensions.

Raw credentials SHALL not enter analytics pipelines.

---

# 85. AI Systems

Sensitive payment credentials SHALL NOT be sent to:

```text
LLMs
embedding services
vector databases
prompt logs
AI observability tools
```

for ordinary Baobab functionality.

---

# 86. Qdrant

Control Plane Qdrant storage SHALL NOT contain raw payment credentials.

---

# 87. Backups

Backups containing sensitive payment data inherit the protection requirements of the source data.

Backup architecture SHALL therefore consider:

```text
encryption
access control
retention
restoration
deletion
geographic location
```

---

# 88. Tokenised Backups

Tokenisation reduces the value of exposed backup data where the underlying credential remains separately protected.

It does not justify weak backup controls.

---

# 89. Data Retention

Baobab SHALL retain sensitive payment information only for documented:

```text
business
legal
regulatory
security
reconciliation
dispute
```

purposes.

---

# 90. No Arbitrary Retention

The following policy is prohibited:

```text
keep payment data forever
because it might be useful later
```

Retention periods SHALL be intentional.

---

# 91. Secure Disposal

When protected data is no longer required, it SHALL be securely deleted or rendered unrecoverable according to applicable policy and storage technology.

---

# 92. Token Retention

Token/reference retention MAY differ from underlying credential retention.

Baobab SHALL retain only references still required for:

```text
recurring payment
refund
dispute
customer payment method
reconciliation
audit
```

as applicable.

---

# 93. Customer Deletion

Customer deletion or privacy requests SHALL not blindly destroy financial records that must lawfully be retained.

The architecture SHALL distinguish:

```text
personal profile
payment credential
financial record
audit record
```

and apply appropriate retention/legal rules.

---

# 94. POPIA and Privacy

Payment-data processing for South African operations SHALL also consider applicable POPIA obligations.

PCI DSS and privacy obligations address overlapping but different concerns.

PCI compliance does not imply privacy compliance.

---

# 95. Cross-Border Data

Provider selection and vault deployment SHALL consider:

```text
data residency
cross-border transfers
Legal Entity jurisdiction
provider processing location
backup location
subprocessor location
```

where legally or contractually relevant.

---

# 96. Market Does Not Equal Data Residency

A Baobab Market does not automatically define where payment data must physically reside.

Residency policy SHALL be explicitly resolved.

---

# 97. Provider Certification

PAY-0022 provider certification SHALL include security and sensitive-data handling assessment.

A provider SHALL not become production eligible merely because it processes payments successfully.

---

# 98. Certification Evidence

Relevant evidence MAY include:

```text
PCI attestation
security documentation
penetration-testing evidence
data-flow documentation
subprocessor information
incident procedures
tokenisation design
vault architecture
key-management controls
data-residency commitments
```

according to risk and contractual requirements.

---

# 99. Certification Expiry

Time-bound security/compliance evidence SHALL be monitored for expiry.

Expired evidence MAY cause:

```text
certification suspension
activation suspension
routing ineligibility
```

according to PAY-0022 policy.

---

# 100. Provider Compliance Is Not Baobab Compliance

A provider's PCI certification does not automatically certify:

```text
Baobab Payments
Trade
digital estates
network architecture
operational processes
```

Baobab's own scope must be assessed.

---

# 101. Tokenisation Does Not Automatically Remove Scope

The security architecture SHALL evaluate whether tokenised systems can still affect the security of payment processing or remain connected to in-scope systems.

Tokenisation is a scope-reduction and risk-reduction technique, not a universal compliance escape hatch.

---

# 102. Network Segmentation

Where PCI scope exists, relevant payment components SHOULD be segmented from unrelated platform workloads where architecture and assessment require it.

Conceptually:

```text
GENERAL BAOBAB NETWORK
        │
        │ controlled boundary
        ▼
PAYMENT SECURITY ZONE
        │
        ├── Payments
        ├── approved vault path
        ├── HyperSwitch
        └── controlled payment infrastructure
```

Actual deployment topology belongs to Infrastructure architecture.

---

# 103. Segmentation Validation

Network segmentation used to reduce PCI scope SHALL be technically validated.

A diagram alone does not establish effective segmentation.

---

# 104. Database Separation

Baobab Payments SHALL continue to use its own database separate from HyperSwitch.

Neither database SHALL be directly accessed by unrelated engines.

---

# 105. HyperSwitch Database

No Baobab engine SHALL query HyperSwitch tables to obtain credential material.

Integration SHALL use approved APIs/interfaces.

---

# 106. Vault Database

No application SHALL bypass the vault interface to retrieve raw stored credentials.

---

# 107. Detokenisation

Detokenisation is a privileged operation.

Baobab SHOULD avoid requiring detokenisation.

---

# 108. Detokenisation Authority

Where detokenisation exists, access SHALL be:

```text
explicitly authorised
purpose-limited
audited
network-restricted
```

and unavailable to ordinary business engines.

---

# 109. Application Detokenisation

Trade, CMS, ERP, Pulse and Control Plane SHALL not detokenise card credentials.

---

# 110. Payments Detokenisation

Even `baobab-payments` SHOULD avoid detokenisation where the vault/provider can execute payment operations directly using a token/reference.

---

# 111. Token Exchange

Where routing requires translating one token representation into another, such exchange SHALL occur only through approved secure payment infrastructure.

Baobab SHALL not reconstruct the original PAN merely to achieve portability unless explicitly required and approved.

---

# 112. Stored Payment Methods

A reusable payment method SHALL be created only through an explicit customer/business workflow.

A one-time payment credential SHALL not automatically become a stored reusable credential.

---

# 113. Consent and Business Authority

Where recurring or future use requires customer consent, mandate or contractual authority, the originating business domain SHALL preserve that authority.

Payments owning the token does not establish permission to charge it.

---

# 114. Payment Method Status

Reusable payment references SHOULD support states such as:

```text
ACTIVE
SUSPENDED
EXPIRED
REVOKED
DELETED
```

or equivalent.

---

# 115. Expired Card

Card expiry SHOULD not require Baobab to retrieve PAN.

Provider/vault lifecycle capabilities SHOULD be used where available.

---

# 116. Network Token Lifecycle

If network payment tokens are later adopted, their lifecycle and cryptogram requirements SHALL remain behind the payment-method abstraction.

---

# 117. Customer Portability

Payment method portability between providers is not guaranteed.

Migration SHALL explicitly assess whether:

```text
token export
token exchange
provider migration
customer re-entry
network token
```

is required.

---

# 118. Provider Exit

Provider exit planning SHALL include the fate of stored payment methods.

A provider contract SHOULD address, where applicable:

```text
token portability
secure migration
data return
data destruction
evidence of destruction
```

---

# 119. Provider Failure

Provider outage does not justify exporting sensitive credentials into Baobab.

Failover remains constrained by secure credential portability.

---

# 120. Compromise Response

Suspected compromise of:

```text
payment credentials
vault
provider credential
webhook secret
encryption key
```

SHALL trigger security incident procedures.

---

# 121. Credential Revocation

Compromised provider credentials SHALL be revocable without requiring code changes where architecture permits.

---

# 122. Token Revocation

Compromised or invalid payment method references SHALL support revocation/suspension.

---

# 123. Incident Scope

Incident investigation SHALL determine:

```text
what data was exposed
which tenant
which Legal Entity
which provider
which market
which payment methods
which time range
```

without exposing further sensitive data during investigation.

---

# 124. Audit

Security-sensitive operations SHALL generate audit evidence.

Representative operations:

```text
payment method created
payment method revoked
provider secret changed
vault configuration changed
detokenisation attempted
sensitive-data access attempted
security policy changed
```

---

# 125. Audit Logs

Audit logs themselves SHALL not contain prohibited credential data.

---

# 126. Separation of Duties

Where practical, no single operator SHOULD be able to:

```text
change provider credentials
change routing
access sensitive data
erase audit evidence
```

without appropriate controls.

---

# 127. Production Administrative UI

Any future payment administration interface SHALL default to masked and minimised data.

There SHALL be no general-purpose “show full card” capability.

---

# 128. HyperSwitch Control Center

HyperSwitch Control Center remains an operational/admin surface.

Its access SHALL be privileged.

It SHALL NOT become a substitute for Baobab IAM, Control Plane or canonical governance.

---

# 129. Screenshots and Exports

Administrative tools SHALL avoid exposing sensitive payment data through:

```text
CSV exports
screenshots
browser caching
clipboard
support attachments
```

where possible.

---

# 130. Browser Security

Payment pages SHALL apply appropriate web security controls.

These SHOULD include, according to architecture:

```text
Content Security Policy
secure cookies
TLS
anti-clickjacking controls
dependency integrity
script governance
XSS prevention
CSRF protection where relevant
```

---

# 131. Payment Page Scripts

Third-party scripts on payment surfaces SHALL be minimised and governed.

Marketing or analytics convenience SHALL not take precedence over payment-page security.

---

# 132. Digital Estate Analytics

Payment credential fields SHALL not be captured by:

```text
session replay
heatmaps
frontend telemetry
analytics SDKs
error reporting
```

---

# 133. Browser Error Reporting

Client-side error tools SHALL redact payment input and payment tokens where necessary.

---

# 134. Content Security Policy

Payment capture surfaces SHOULD use restrictive CSP appropriate to the selected provider/tokenisation model.

---

# 135. Supply-Chain Security

Dependencies capable of affecting payment capture SHALL receive elevated supply-chain scrutiny.

---

# 136. Dependency Vulnerabilities

Critical vulnerabilities in payment-path components SHALL be prioritised according to the platform vulnerability-management policy and applicable PCI requirements.

---

# 137. CI/CD

CI SHALL prevent known patterns of:

```text
committed secrets
private keys
credential fixtures
unsafe debug configuration
```

from reaching production where tooling permits.

---

# 138. Secret Scanning

Repositories participating in the payment path SHOULD enable secret scanning or equivalent controls.

---

# 139. Dependency Scanning

Payment-path repositories SHOULD perform dependency and container vulnerability analysis.

---

# 140. Container Images

Payment production images SHALL:

```text
use minimal required components
avoid embedded secrets
run with least privilege
use pinned/controlled dependencies
be reproducibly built where practical
```

---

# 141. Production Debug Tools

Development-only debugging utilities capable of exposing request bodies SHALL not be enabled in production.

---

# 142. Database Dumps

Production database dumps SHALL be treated as sensitive assets.

They SHALL not be copied into developer environments.

---

# 143. Support Bundles

Automated support bundles SHALL redact or exclude sensitive payment information.

---

# 144. Observability Architecture

Preferred observability:

```text
Payment
   │
   ├── payment_id
   ├── attempt_id
   ├── correlation_id
   ├── provider
   ├── method_type
   ├── market
   └── result
```

not:

```text
Payment
   │
   ├── PAN
   ├── CVV
   └── bank credentials
```

---

# 145. Payment Credential Data Flow

Preferred:

```text
Customer
   │
   │ credential
   ▼
SECURE CAPTURE COMPONENT
   │
   │ credential
   ▼
VAULT / PAYMENT INFRASTRUCTURE
   │
   │ opaque token
   ▼
BAOBAB PAYMENTS
   │
   │ PaymentMethodReference
   ▼
TRADE / SUBSCRIPTIONS
```

---

# 146. Canonical Data Flow

```text
CONTROL PLANE
Tenant / Legal Entity / Market
             │
             ▼
          PAYMENTS
             │
     ┌───────┴────────┐
     │                │
Canonical         Payment Method
Payment Data       Reference
     │                │
     │                ▼
     │           Secure Vault /
     │           Payment Infra
     │
     ▼
Canonical Events
     │
 ┌───┼────────────┐
 ▼   ▼            ▼
Trade ERP    Subscriptions
```

Raw credentials do not propagate with canonical events.

---

# 147. Tokenised Recurring Payment Flow

```text
Customer
   │
   ▼
Secure Capture
   │
   ▼
Vault / Provider
   │
   ├── stores/protects credential
   │
   ▼
Payment Method Token
   │
   ▼
Baobab Payments
   │
   ▼
PaymentMethodReference
   │
   ▼
Subscriptions
stores reference only

... later ...

Subscriptions
   │
   ▼
Payment Obligation
   │
   ▼
Baobab Payments
   │
   ▼
PaymentMethodReference
   │
   ▼
Vault / HyperSwitch / Provider
   │
   ▼
Payment Execution
```

Subscriptions never needs the underlying PAN.

---

# 148. Cross-Provider Routing with Tokens

```text
PaymentMethodReference
        │
        ▼
Determine Token Type
        │
        ▼
Determine Portability
        │
   ┌────┴─────────────┐
   │                  │
Provider-specific   Portable
   │                  │
   ▼                  ▼
Provider A only    Eligible providers
```

PAY-0009 routing occurs only after this constraint is applied.

---

# 149. Secret Flow

```text
SECRET MANAGER
      │
      ▼
Authorised Payments Runtime
      │
      ▼
HyperSwitch / Provider

Trade ─────────X
ERP ───────────X
CMS ───────────X
Pulse ─────────X
Browser ───────X
```

Provider credentials do not propagate through business engines.

---

# 150. Data-Zone Model

Conceptually:

```text
ZONE 1
GENERAL BUSINESS DATA

Trade
CMS
Subscriptions
ERP
Control Plane

          │
          │ canonical references only
          ▼

ZONE 2
PAYMENT ORCHESTRATION

Baobab Payments

          │
          │ controlled payment interface
          ▼

ZONE 3
SENSITIVE PAYMENT INFRASTRUCTURE

HyperSwitch
Vault
Provider
Tokenisation Infrastructure
```

Physical deployment need not map one-to-one to these conceptual zones.

The security boundary SHALL, however, preserve their intent.

---

# 151. Production Readiness Gate

```text
PCI / PAYMENT DATA SECURITY

Payment data-flow documented                 PASS
PCI scope assessed                           PASS
Credential capture architecture reviewed     PASS
PAN minimised                                PASS
CVV persistence prohibited                   PASS
Sensitive authentication data controls       PASS
Tokenisation operational                     PASS
Vault architecture reviewed                  PASS
PaymentMethodReference implemented           PASS
Token provenance recorded                    PASS
Token portability classified                 PASS
Cross-tenant isolation                       PASS
Cross-Legal-Entity isolation                 PASS
Sandbox/prod isolation                       PASS
Provider secrets managed                     PASS
Secret rotation tested                       PASS
Encryption in transit                        PASS
Encryption at rest                           PASS
Least privilege                              PASS
Human privileged access                      PASS
Log redaction                                PASS
Trace redaction                              PASS
Event minimisation                           PASS
Webhook minimisation                         PASS
DLQ protection                               PASS
No real credentials in tests                 PASS
Secret scanning                              PASS
Dependency scanning                          PASS
Retention policy                             PASS
Secure deletion                              PASS
Backup protection                            PASS
Incident response                            PASS
Provider security certification              PASS
Payment-page security                        PASS
Observability                                PASS
Audit                                        PASS
-------------------------------------------------
Sensitive Payment Data Plane                 READY
```

---

# 152. Invariants

The following invariants SHALL hold:

1. Raw payment credentials are excluded from Baobab wherever technically possible.
2. Canonical Payment contracts do not require PAN.
3. Full PAN is not stored by default.
4. CVV/CVC/CID is never persisted after authorization.
5. Prohibited sensitive authentication data is not retained merely because it can be encrypted.
6. Trade is not a credential vault.
7. Control Plane is not a credential vault.
8. ERP is not a credential vault.
9. CMS is not a credential vault.
10. Pulse is not a credential vault.
11. IAM is not a credential vault.
12. Subscriptions stores references rather than raw credentials.
13. Canonical events never transport raw card credentials.
14. Logs never intentionally contain sensitive authentication data.
15. Traces never intentionally contain raw payment credentials.
16. Metrics never use raw credentials as dimensions.
17. DLQs do not become uncontrolled credential stores.
18. Analytics pipelines do not receive raw payment credentials.
19. AI systems do not receive raw payment credentials.
20. Real payment credentials are never used as ordinary test fixtures.
21. Production credentials never enter sandbox.
22. Sandbox credentials never establish production authority.
23. Payment tokens are not assumed universally portable.
24. Provider-specific tokens constrain routing.
25. Cross-provider failover cannot bypass token compatibility.
26. Payment method references retain provenance.
27. Payment method references retain applicable tenant/Legal Entity/environment scope.
28. Group ownership does not imply payment-method sharing.
29. External customer tenants receive the same isolation guarantees.
30. Provider credentials remain in governed secret storage.
31. Provider credentials are not embedded in source code.
32. Sensitive credentials are not committed to Git.
33. Secret rotation is supported.
34. Detokenisation is exceptional.
35. Ordinary business engines cannot detokenise credentials.
36. Payments itself avoids detokenisation where token execution is possible.
37. Tokenisation reduces exposure but does not automatically prove PCI scope reduction.
38. Provider compliance does not automatically establish Baobab compliance.
39. Encryption does not replace tokenisation or minimisation.
40. Storage encryption does not replace access control.
41. Network location does not replace workload authentication.
42. Payment-data access follows least privilege.
43. Administrative tools expose masked/minimised data by default.
44. Provider certification includes sensitive-data security assessment.
45. Expired security certification can affect routing eligibility.
46. Data retention is intentional and documented.
47. Sensitive data no longer required is securely disposed of.
48. Backups receive equivalent protection.
49. Payment credential compromise is a security incident.
50. Payment-page scripts are security-sensitive dependencies.
51. Session replay and analytics tooling cannot capture credential fields.
52. Provider/vault changes trigger PCI/security-scope review.
53. New payment methods trigger data-flow review.
54. New payment channels trigger data-flow review.
55. A custom Baobab card vault requires a separate architecture decision.
56. Security controls apply uniformly to Nabhold subsidiaries and external tenants.
57. No implementation may weaken these controls for convenience.

---

# 153. Consequences

## Positive

This architecture:

- materially reduces sensitive-data exposure;
- reduces breach blast radius;
- supports PCI scope minimisation;
- isolates provider secrets;
- enables safer recurring payments;
- supports multi-provider architecture;
- prevents credential leakage into business engines;
- strengthens tenant isolation;
- improves provider portability analysis;
- protects event and observability systems;
- establishes clear vault/token boundaries;
- improves auditability.

## Negative

It requires:

- secure payment-capture integration;
- tokenisation infrastructure;
- vault integration;
- secret management;
- token-provenance tracking;
- portability modelling;
- additional provider certification;
- data-flow documentation;
- compliance review;
- payment-page hardening;
- operational security procedures.

These costs are accepted because payment credential exposure is materially more dangerous than the complexity required to avoid it.

---

# 154. Alternatives Considered

## Store PAN in Baobab Payments

Rejected as the default.

It unnecessarily expands sensitive-data exposure and compliance responsibility.

---

## Store credentials in Trade

Rejected.

Commerce authority does not require credential-vault authority.

---

## Store credentials in ERP

Rejected.

Accounting does not require payment credentials.

---

## Let every digital estate integrate directly with PSP tokenisation

Rejected as the platform architecture.

This would fragment payment security and provider integration.

Secure client components may communicate with approved payment infrastructure, but canonical orchestration remains under Payments.

---

## Build a custom Baobab card vault immediately

Rejected.

Existing specialised payment/vault infrastructure should be preferred unless a compelling requirement justifies the additional security responsibility.

---

## Assume tokenisation eliminates PCI scope

Rejected.

Actual scope depends on the complete architecture and applicable standard.

---

## Encrypt everything and keep it indefinitely

Rejected.

Encryption does not replace data minimisation or retention controls.

---

## Permit raw credentials in logs only during debugging

Rejected.

Debug convenience does not justify credential leakage.

---

## Share stored payment methods between subsidiaries

Rejected by default.

Legal-entity independence remains authoritative.

---

# 155. Relationship to PAY-0009

Routing SHALL consider:

```text
PROVIDER ELIGIBILITY
        +
PAYMENT METHOD ELIGIBILITY
        +
TOKEN PORTABILITY
        +
MERCHANT COMPATIBILITY
        =
VALID ROUTE
```

A certified provider is not a valid route if it cannot securely consume the available Payment Method Reference.

---

# 156. Relationship to PAY-0010

Idempotency records SHALL contain:

```text
canonical identifiers
request fingerprints
safe external references
```

not raw payment credentials.

Duplicate safety does not require credential duplication.

---

# 157. Relationship to PAY-0011

Webhook/event infrastructure SHALL propagate:

```text
financial facts
canonical IDs
safe provider references
```

not payment credentials.

Raw external payload retention is exceptional, minimised and controlled.

---

# 158. Relationship to PAY-0013

Refunds SHALL execute using the existing canonical Payment and provider/vault references.

A refund workflow SHALL NOT require recovering a customer's raw card credentials.

Conceptually:

```text
Refund
   │
   ▼
Original Payment
   │
   ▼
Provider Payment Reference
   │
   ▼
Provider Refund
```

---

# 159. Relationship to PAY-0014

Dispute evidence SHALL be minimised.

Dispute handling may require payment-related evidence, but this does not justify exposing raw credentials to ordinary dispute workflows.

---

# 160. Relationship to PAY-0015

Settlement and reconciliation SHALL use:

```text
payment references
processor references
merchant references
settlement references
amount
currency
fees
```

rather than payment credentials.

---

# 161. Relationship to PAY-0016

Payout credentials such as beneficiary bank details require similarly strict treatment.

PAY-0016 SHALL define payout-specific beneficiary security without weakening PAY-0012's minimisation principles.

---

# 162. Relationship to PAY-0017

Cross-border payment processing SHALL consider where sensitive payment data is:

```text
captured
tokenised
processed
stored
backed up
```

independently from the commercial Market model.

---

# 163. Relationship to PAY-0018

PAY-0018 SHALL define:

```text
who
under which workload identity
with which scope
under which approval
```

may mutate payment/security configuration.

PAY-0012 defines the data that those controls protect.

---

# 164. Relationship to PAY-0020

Backup and disaster recovery SHALL preserve both:

```text
availability
```

and:

```text
payment-data security
```

A disaster-recovery copy is not permitted to become a weaker copy of sensitive production data.

---

# 165. Relationship to PAY-0021

HyperSwitch upgrades, extensions or forks SHALL be assessed for impact on:

```text
vaulting
tokenisation
credential flow
PCI scope
encryption
secret handling
webhook content
logging
```

before adoption.

---

# 166. Relationship to PAY-0022

Provider certification SHALL answer:

```text
Can this provider process the payment?
```

and:

```text
Can Baobab safely entrust the relevant payment data to this provider in this business context?
```

Both must be true.

---

# 167. Complete Security Boundary

```text
                    CUSTOMER
                       │
                       │ payment credential
                       ▼
              SECURE CAPTURE SURFACE
                       │
                       ▼
             TOKENISATION / VAULT
                       │
                 opaque reference
                       │
                       ▼
               BAOBAB PAYMENTS
                       │
          ┌────────────┼────────────┐
          │            │            │
          ▼            ▼            ▼
     PaymentIntent   Payment      Refund
          │            │            │
          └────────────┼────────────┘
                       │
                       ▼
                  HyperSwitch
                       │
                       ▼
               Certified Provider
                       │
                       ▼
                 Payment Rail

Canonical facts
      │
      ▼
   Events
      │
 ┌────┼───────────────┐
 ▼    ▼               ▼
Trade ERP       Subscriptions

NO RAW CREDENTIAL PROPAGATION
```

---

# 168. Final Decision

Baobab's permanent payment-data model SHALL be:

```text
RAW CREDENTIAL
      │
      ▼
SECURE CAPTURE
      │
      ▼
TOKENISATION / VAULT
      │
      ▼
OPAQUE PAYMENT METHOD REFERENCE
      │
      ▼
BAOBAB PAYMENT EXECUTION
      │
      ▼
CANONICAL FINANCIAL FACTS
```

The architectural distinctions are:

```text
PAYMENT METHOD
      ≠
PAYMENT CREDENTIAL

PAYMENT METHOD REFERENCE
      ≠
RAW CREDENTIAL

TOKEN
      ≠
CANONICAL PAYMENT ID

TOKEN
      ≠
UNIVERSALLY PORTABLE CREDENTIAL

MASKED PAN
      ≠
FULL PAN

PROVIDER SECRET
      ≠
APPLICATION CONFIGURATION

PCI-COMPLIANT PROVIDER
      ≠
PCI-COMPLIANT BAOBAB DEPLOYMENT

TOKENISATION
      ≠
AUTOMATIC REMOVAL FROM PCI SCOPE

ENCRYPTION
      ≠
PERMISSION TO RETAIN UNNECESSARY DATA

CVV ENCRYPTION
      ≠
PERMISSION TO STORE CVV AFTER AUTHORIZATION
```

Therefore:

**Baobab shall not become a repository of payment credentials merely because it orchestrates payments.  
Digital estates shall not become payment vaults merely because they own checkout.  
ERP shall not receive credentials merely because it owns accounting.  
Subscriptions shall not receive credentials merely because it initiates recurring collection.  
Control Plane shall not receive credentials merely because it resolves payment context.**

**Sensitive payment data shall remain as close as possible to specialised, approved payment-security infrastructure. Baobab shall move references and financial facts across the platform—not raw credentials.**