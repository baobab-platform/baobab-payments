# PAY-0001 — Adopt HyperSwitch as the Headless Baobab Payment Orchestration Engine

**Status:** Accepted
**Decision Type:** Architecture / Platform Engine
**Repository:** `baobab-platform/baobab-payments`
**Scope:** Baobab Platform
**Engine:** HyperSwitch
**Owners:** Nabhold / Baobab Platform Architecture
**Supersedes:** None
**Related:** Baobab Control Plane, Trade, ERP, IAM, Subscriptions (ADR-SUB-0001), Shared Contracts (ADR-SHARED-011), Infrastructure
**Refreshed:** 2026-09-25: repository namespace (`baobab-platform`), upstream runtime verification (§15.1), and the Baobab façade and sandbox boundary (§15.2)

---

## 1. Context

Baobab requires a payment capability that can support multiple digital estates, legal entities, markets, currencies, payment methods, payment service providers, and commercial models without coupling any individual estate or business engine directly to a specific payment processor.

Initial Baobab digital estates include different commercial models:

* ZuriBeans operates as a B2B trading business.
* Thamani operates as a B2C commerce business.
* Future Baobab tenants may operate B2B, B2C, marketplace, SaaS, subscription, or mixed commercial models.

Payment capabilities will consequently be required by multiple upstream engines, including:

```text
baobab-trade
baobab-subscriptions
baobab-erp
future Baobab engines
future external Baobab customers
```

Direct integrations between every consuming engine and every payment service provider would lead to an undesirable topology:

```text
                 PayU
                ↗
Medusa ────────→ Flutterwave
                ↘
                 Stripe

Subscriptions ─→ Paystack
             ──→ DPO

ERP ───────────→ Bank APIs
```

This creates duplicated connector code, inconsistent retry behaviour, duplicated credentials, different payment states, divergent observability, and tight coupling between business engines and payment processors.

Baobab therefore requires a dedicated payment orchestration capability.

HyperSwitch provides an orchestration layer between applications and payment processors. Its documented responsibilities include routing, retry and failover logic, response normalisation and centralised provider configuration, while actual authorisation, clearing and settlement remain responsibilities of payment processors and acquirers.

HyperSwitch is therefore suitable as the foundational implementation of a dedicated Baobab Payments Engine.

---

# 2. Decision

Baobab SHALL adopt **HyperSwitch as the foundational headless payment orchestration engine** implemented and operated through:

```text
baobab-platform/baobab-payments
```

The repository SHALL expose HyperSwitch capabilities through Baobab-defined contracts rather than allowing consuming applications to depend directly upon HyperSwitch implementation details.

The logical architecture SHALL be:

```text
                    BAOBAB PLATFORM

                         baobab-cp
                             │
                context / engine resolution
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
          ▼                  ▼                  ▼
   baobab-trade     baobab-subscriptions   future engine
      Medusa
          │                  │                  │
          └──────────────────┼──────────────────┘
                             │
                             ▼
                     baobab-payments
                             │
                    Baobab Payment API
                             │
                             ▼
                        HyperSwitch
                             │
             ┌───────────────┼────────────────┐
             │               │                │
             ▼               ▼                ▼
           PayU         Flutterwave         DPO
             │
             ├──────────── Paystack
             ├──────────── Pesapal
             ├──────────── Adyen
             └──────────── future PSPs
```

HyperSwitch SHALL therefore be treated as an **engine implementation**, not as the Baobab payment-domain contract itself.

---

# 3. Architectural Principle

Baobab follows the principle:

> Standardise contracts, not implementations.

Existing Baobab architecture deliberately permits specialised engines to retain their native implementations while Baobab standardises identity, tenancy, APIs, events, audit, configuration, observability and security.

Accordingly:

```text
HyperSwitch ≠ Baobab Payments Contract

HyperSwitch = implementation of Baobab Payments capability
```

Consumers SHALL integrate with the Baobab payment capability rather than acquiring ownership of HyperSwitch-specific concepts.

This permits HyperSwitch to be upgraded, extended, isolated, or eventually replaced without requiring wholesale changes to MedusaJS, subscription billing, ERP, or digital estates.

---

# 4. Engine Responsibilities

`baobab-payments` SHALL own payment orchestration capabilities including:

```text
Payment execution
Payment processor orchestration
Connector invocation
Payment routing
Fallback routing
Retry orchestration
Payment authorisation coordination
Capture coordination
Void/cancellation coordination
Refund orchestration
Payment-method orchestration
Token/reference handling
Processor response normalisation
Payment operational status
Processor references
Payment webhooks
Payment routing telemetry
Payment-level reconciliation inputs
```

HyperSwitch currently provides rule-based routing based on attributes including payment method, transaction amount, currency, customer country, card network and business profile, and supports idempotency keys for duplicate-request protection.

These capabilities MAY be exposed through Baobab where appropriate but SHALL remain subordinate to Baobab platform boundaries.

---

# 5. Explicit Non-Responsibilities

`baobab-payments` SHALL NOT own:

```text
Tenant authority
Legal entity authority
Digital Estate authority
Market authority
Canonical customer identity
Product catalogue
Orders
Commerce pricing
Inventory
Subscription plan definitions
Subscription lifecycle
Accounting ledger
Revenue recognition
General ledger posting
Enterprise identity
Platform authorisation policy
```

These responsibilities belong respectively to:

```text
baobab-cp
baobab-trade
baobab-subscriptions
baobab-erp
baobab-iam
and other authoritative Baobab engines
```

The payment engine SHALL consume resolved context rather than redefine it.

---

# 6. Control Plane Relationship

`baobab-cp` remains responsible for resolving:

```text
Tenant
Legal Entity
Digital Estate
Market
Engine
Engine Instance
Capability
Capability Binding
Isolation Profile
```

The conceptual request path SHALL therefore be:

```text
Request
   │
   ▼
Baobab context resolution
   │
   ▼
Tenant
Legal Entity
Market
Digital Estate
   │
   ▼
Payment capability resolution
   │
   ▼
baobab-payments
   │
   ▼
HyperSwitch merchant/profile context
```

HyperSwitch SHALL NOT independently decide which Baobab tenant or legal entity owns a transaction.

---

# 7. HyperSwitch Account Hierarchy

HyperSwitch currently provides an internal hierarchy broadly consisting of:

```text
Tenant
  │
  └── Organisation
        │
        └── Merchant
              │
              └── Business Profile
```

Its current platform model additionally supports a Platform Organisation containing a Platform Merchant together with Connected or Standard merchants. Connected merchants may share customers and payment methods, while Standard merchants retain isolation.

This hierarchy SHALL NOT automatically become Baobab's canonical organisational hierarchy.

Specifically:

```text
HyperSwitch Organisation ≠ Baobab Tenant
HyperSwitch Merchant     ≠ automatically LegalEntity
HyperSwitch Profile      ≠ automatically Market
```

The exact mappings SHALL be governed by PAY-0003 and PAY-0004.

This distinction is mandatory because HyperSwitch permits merchants to model business units, brands, geographies and channels, while Baobab already has canonical concepts for legal entities, markets and digital estates.

---

# 8. Headless Operation

HyperSwitch SHALL operate as a **headless Baobab engine**.

Digital estates SHALL NOT be required to use HyperSwitch Control Center as their business-facing payment interface.

The normal application path SHALL be:

```text
Digital Estate
      │
      ▼
Business Engine
      │
      ▼
Baobab Payments API
      │
      ▼
HyperSwitch
```

HyperSwitch Control Center MAY be retained as an engine-native operational interface for authorised payment administrators, troubleshooting and specialised configuration.

It SHALL NOT become the canonical Baobab Control Plane UI.

---

# 9. Commerce Integration

MedusaJS SHALL delegate external payment execution to `baobab-payments`.

The target architecture is:

```text
ZuriBeans / Thamani
        │
        ▼
    baobab-trade
      MedusaJS
        │
        ▼
Baobab Payment Provider
        │
        ▼
   baobab-payments
        │
        ▼
     HyperSwitch
        │
        ▼
       PSP
```

Medusa SHALL retain commerce-domain concepts including:

```text
Cart
Order
PaymentCollection
PaymentSession
commerce-level Payment
```

HyperSwitch SHALL retain payment-orchestration concepts.

The detailed Medusa integration SHALL be governed by PAY-0006.

---

# 10. Subscription Integration

`baobab-subscriptions` SHALL use the same payment orchestration engine.

The expected relationship is:

```text
Subscription lifecycle
        │
        ▼
baobab-subscriptions
        │
   payment required
        │
        ▼
baobab-payments
        │
        ▼
HyperSwitch
        │
        ▼
Payment Processor
```

`baobab-subscriptions` SHALL decide:

```text
what is owed
when it is owed
subscription status
billing period
credits
entitlements
usage charges
```

`baobab-payments` SHALL decide:

```text
how payment is executed
through which eligible processor
how processor responses are normalised
how payment execution is retried
```

---

# 11. ERP Relationship

`baobab-erp` SHALL remain authoritative for accounting consequences.

The architecture SHALL separate:

```text
Payment execution
        │
        ▼
baobab-payments
        │
 payment.captured
 payment.refunded
 settlement information
        │
        ▼
baobab-erp
        │
        ▼
Accounting
Reconciliation
Receivables
Ledger
```

HyperSwitch SHALL NOT become Baobab's financial ledger.

Its reconciliation capabilities MAY assist operational reconciliation, but accounting authority remains with ERP.

HyperSwitch's current reconciliation model uses its Organisation/Merchant/Profile hierarchy as configuration and access boundaries.

The mapping between operational reconciliation and ERP accounting SHALL therefore be explicitly defined in PAY-0015.

---

# 12. Event-Driven Integration

Payment state changes SHALL be translated into Baobab canonical events.

Representative events include:

```text
payment.created.v1
payment.requires_action.v1
payment.authorized.v1
payment.capture_requested.v1
payment.captured.v1
payment.failed.v1
payment.cancelled.v1
payment.refund_requested.v1
payment.refunded.v1
payment.dispute.opened.v1
payment.dispute.updated.v1
payment.settlement.received.v1
```

HyperSwitch-native payloads SHALL NOT become cross-platform canonical event schemas.

Translation SHALL occur at the `baobab-payments` boundary.

---

# 13. Webhook Reliability

Incoming HyperSwitch webhooks SHALL be treated as untrusted external asynchronous messages until:

```text
signature verified
event identified
merchant context resolved
Baobab mapping resolved
idempotency verified
schema validated
authorisation boundary validated
```

HyperSwitch currently signs webhooks and retries failed deliveries for approximately 24 hours. Its documentation also warns that duplicate and out-of-order deliveries may occur and recommends deduplication using `event_id`.

Baobab SHALL therefore assume:

```text
at-least-once delivery
possible duplicates
possible reordering
possible delayed delivery
```

PAY-0011 SHALL define the exact event-ingestion design.

---

# 14. Idempotency

Every payment-creating or money-moving operation SHALL support Baobab-level idempotency.

Idempotency identifiers SHALL propagate through the integration chain where supported:

```text
Digital Estate
      │
correlation / idempotency
      ▼
Trade / Subscription
      │
      ▼
baobab-payments
      │
      ▼
HyperSwitch
      │
      ▼
Processor
```

HyperSwitch supports API idempotency keys and returns the original result when the same idempotent request is replayed.

Baobab SHALL nonetheless maintain its own idempotency and canonical transaction references rather than treating HyperSwitch's implementation as the sole protection.

---

# 15. Runtime Architecture

The initial engine deployment SHALL use HyperSwitch's native architecture rather than rewriting payment orchestration.

HyperSwitch currently consists primarily of:

```text
Router
Scheduler
  ├── Producer
  └── Consumer

PostgreSQL
Redis
Locker / secure PII storage
```

The Router handles core payment flows. Scheduler Producer identifies due jobs and places batches onto Redis, while Scheduler Consumer executes those jobs. PostgreSQL holds durable merchant/customer/payment data and Redis serves caching and scheduler queue functions.

The current upstream Compose configuration likewise provides PostgreSQL, Redis, the HyperSwitch server and optional scheduler services.

Baobab SHALL initially preserve those native runtime responsibilities.

### 15.1 Runtime verification (refreshed 2026-09-25)

The upstream runtime has changed since this ADR was accepted:

- **Superposition is mandatory.** From HyperSwitch **v1.124.0**, Superposition, Juspay's configuration service, is a required dependency: "Deployments must have an active Superposition service it can connect to before upgrading to v1.124.0". The upstream Compose file runs `ghcr.io/juspay/superposition-demo:0.113.0` for it.
- **Required runtime components** for a current HyperSwitch deployment:
  - PostgreSQL;
  - Redis (`redis:7` upstream; standalone or cluster);
  - Superposition;
  - the Router;
  - the Scheduler Producer and Consumer;
  - the Drainer.
- **Optional components:**
  - ClickHouse and Kafka (analytics and event streaming);
  - the Locker (card vaulting, governed by PAY-0012).
- **Latest release observed:** `v1.126.0` (August 2026). v1.125.0, v1.124.0 and v1.123.x precede it.
- **PostgreSQL version:** upstream Compose runs `postgres:latest`, which states no supported major version. Baobab SHALL NOT pin `latest` for any HyperSwitch component, and SHALL NOT assume the devcontainer's `postgres:16` image proves production support. The HyperSwitch integration gate SHALL pin an exact HyperSwitch release plus PostgreSQL, Redis and Superposition versions verified against it.

### 15.2 The Baobab payment façade

HyperSwitch is reached only through the Baobab-owned façade in this repository, written in Rust, the repository's declared runtime:

```text
Consuming engine ──► Baobab Payment API (contracts/payments/v1)
                            │
                     PaymentProvider port
                   ┌────────┴─────────┐
                   ▼                  ▼
            SandboxProvider     HyperSwitch adapter
          (NON-PRODUCTION ONLY)  (NOT YET IMPLEMENTED)
```

The façade:

- owns Baobab idempotency, correlation and context validation;
- persists Baobab payment records in its own PostgreSQL database (PostgreSQL 17, the Baobab service standard), separate from HyperSwitch's schema.

The **SandboxProvider** simulates processor outcomes for integration testing only:

- it runs only with explicit non-production configuration (`PAYMENT_PROVIDER=sandbox` in a non-production environment);
- the service refuses to start with it in production;
- every record and event it produces is marked `simulated`;
- nothing it produces is ever a real-world settlement.

The **HyperSwitch adapter** is NOT YET IMPLEMENTED. No component may be labelled as a HyperSwitch integration until it calls HyperSwitch.

---

# 16. Database Isolation

HyperSwitch SHALL own its operational persistence.

No other Baobab engine SHALL directly access HyperSwitch database tables.

Prohibited:

```text
baobab-trade ───────► HyperSwitch PostgreSQL

baobab-erp ─────────► HyperSwitch PostgreSQL

baobab-cp ──────────► HyperSwitch PostgreSQL

baobab-subscriptions► HyperSwitch PostgreSQL
```

Required:

```text
Engine
   │
   ├── API
   └── canonical events
          │
          ▼
   baobab-payments
```

This maintains Baobab's established engine-independence principle.

---

# 17. Processor Independence

Baobab SHALL NOT make any individual payment processor a platform dependency.

Processors SHALL remain connector implementations beneath HyperSwitch.

Conceptually:

```text
                 HyperSwitch
                     │
       ┌─────────────┼─────────────┐
       │             │             │
      PSP A         PSP B         PSP C
       │
       └──────── future connectors
```

Digital estates SHALL not contain business logic such as:

```text
if market == "ZA":
    use PayU
```

or:

```text
if market == "UG":
    use Flutterwave
```

Routing policies belong to payment orchestration, subject to Baobab-resolved legal-entity and market constraints.

---

# 18. Routing Authority

Baobab Control Plane SHALL determine:

```text
which payment engine
which tenant
which legal entity
which market
which engine instance
which permitted payment configuration
```

HyperSwitch SHALL determine, within that authorised scope:

```text
which eligible connector
which configured routing rule
whether permitted retry/fallback applies
```

Thus:

```text
Baobab decides the BUSINESS CONTEXT.

HyperSwitch decides the PAYMENT ROUTE.
```

This boundary SHALL not be inverted.

---

# 19. Security Boundary

Payment credentials, processor API keys, payment tokens and sensitive payment material SHALL NOT be propagated unnecessarily into:

```text
ZuriBeans frontend
Thamani frontend
baobab-trade
baobab-subscriptions
baobab-cp
baobab-erp
```

The payment engine SHALL minimise the PCI and sensitive-data surface of the remainder of the Baobab platform.

HyperSwitch's architecture includes a dedicated locker for secure handling of payment-related PII/card information.

The exact tokenisation, vaulting and PCI boundary SHALL be governed by PAY-0012.

---

# 20. Observability

Every money-moving operation SHALL be traceable across Baobab.

The minimum correlation chain SHALL support:

```text
Baobab correlation_id
Baobab payment_id
source_engine
source_reference
tenant_id
legal_entity_id
market_id
digital_estate_id
HyperSwitch payment_id
merchant_id
profile_id
connector
processor transaction reference
```

Sensitive card/payment credentials SHALL never be placed in logs or traces.

HyperSwitch supports OpenTelemetry-based metrics and traces in its documented monitoring architecture.

Baobab SHALL integrate this telemetry into platform observability rather than establishing an isolated monitoring island.

---

# 21. Extension Policy

Baobab SHOULD prefer:

```text
configuration
official APIs
connector interfaces
adapter modules
Baobab façade services
event translation
```

over deep modifications to HyperSwitch core.

Where Baobab-specific functionality is required, it SHOULD be implemented outside upstream core whenever practical.

A permanent fork SHALL require a separate architectural decision or explicit justification under PAY-0021.

---

# 22. Consequences

## Positive

The decision provides:

* one payment orchestration capability for all Baobab engines;
* payment-provider independence;
* common routing and failover;
* centralised processor integration;
* a reduced payment-security surface;
* reusable payment capability for B2B, B2C and subscriptions;
* improved observability;
* clearer commercial-engine boundaries;
* easier addition of African and global payment providers;
* reduced duplication across digital estates.

## Negative

The decision introduces:

* another critical production engine;
* Rust into the Baobab technology estate;
* PostgreSQL and Redis runtime requirements;
* additional operational complexity;
* payment-specific security responsibilities;
* cross-repository contract requirements;
* dependency on HyperSwitch upstream evolution;
* additional certification/testing requirements for connectors.

These costs are accepted because implementing payment orchestration independently inside every consuming engine would create substantially greater architectural and operational risk.

---

# 23. Alternatives Considered

### Direct Medusa payment integrations

Rejected as Baobab's platform strategy because subscriptions, ERP and future engines would still require independent payment integrations.

### Direct payment-provider integrations from every engine

Rejected because it creates duplicated connectors, credentials, error handling, routing and reconciliation.

### Build a custom Baobab payment router

Rejected because payment orchestration, routing, retries, vaulting, processor integrations, disputes and reconciliation represent mature specialist domains which Baobab should adopt rather than rebuild.

### Make HyperSwitch part of `baobab-trade`

Rejected because payments are required outside commerce.

### Make HyperSwitch part of `baobab-subscriptions`

Rejected because commerce and other engines also require payment execution.

---

# 24. Final Decision

Baobab SHALL establish:

```text
baobab-platform/baobab-payments
```

as an independently deployable **Headless Payment Orchestration Engine**, with HyperSwitch serving as its foundational implementation.

The canonical architectural relationship is:

```text
                    BAOBAB PLATFORM

               ┌───────────┴───────────┐
               │                       │
          Control Plane            Digital Estates
               │                       │
               └───────────┬───────────┘
                           │
                  Business Engines
               ┌───────────┼───────────┐
               │           │           │
              Trade   Subscriptions   ERP
               │           │           │
               └───────────┼───────────┘
                           ▼
                    BAOBAB PAYMENTS
                           │
                     HyperSwitch
                           │
               ┌───────────┼───────────┐
               ▼           ▼           ▼
             PSP A       PSP B       PSP C
```

HyperSwitch SHALL orchestrate payments.

It SHALL NOT become the authority for Baobab tenancy, legal entities, markets, commerce, subscriptions, accounting or identity.

**Baobab owns the platform context and contracts. HyperSwitch owns payment orchestration within that authorised context.**
