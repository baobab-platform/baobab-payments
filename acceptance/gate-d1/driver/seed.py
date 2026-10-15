"""Gate D1 seeding: HyperSwitch's merchant, CP's tenant, market and mandate,
and Payments' provider activation and merchant binding.

CP is seeded through its own admin API wherever one exists (tenant bootstrap
and activation, market create and four-eyes activation, mandate propose,
decide and activate by three distinct humans). Where CP has no admin API the
row is inserted directly and recorded as SYNTHETIC in the evidence:
  - the human identities (IAM user provisioning is Gate D2),
  - the legal-entity verification (an independent legal registry),
  - the tenant's market assignment (tenant provisioning),
  - the baobab-payments engine instance and its capability binding.
"""
from __future__ import annotations

import json
import time
import uuid

from d1lib import CP, HYPERSWITCH, ISSUER, Compose, call

ISS = "https://iam.d1.test/realms/baobab"
LEGAL_ENTITY = "LE-D1ZA"
MARKET = "ZA"
CURRENCY = "ZAR"
CUSTOMER = "pacct_d1buyer"
PAYMENTS_CLIENT = "baobab-payments-legal-actor-assessor-staging"
ADMIN_SCOPES = ["tenant:bootstrap", "tenant:write", "market:write", "market:approve",
                "legal-actor-mandate:propose", "legal-actor-mandate:decide",
                "legal-actor-mandate:activate", "legal-actor-mandate:terminate"]
CAPABILITIES = ["payment.intent.create", "payment.payment.authorize", "payment.payment.capture",
                "payment.refund.create", "payment.intent.cancel"]
ACTIVITIES = ["PAYMENT_COLLECTION", "PAYMENT_REFUND", "PAYMENT_VOID"]
CARDS = {"approve": "4242424242424242", "decline": "4000000000000002"}


def must(response, *ok: int):
    if response.status not in ok:
        raise RuntimeError(f"unexpected answer {response!r}")
    return response.body


def human_token(subject: str) -> str:
    return must(call("POST", ISSUER + "/_mint", {
        "subject": subject, "audience": "baobab-control-plane-admin", "scopes": ADMIN_SCOPES,
        "actor_type": "human", "roles": ["cp:platform-admin"]}), 200)["access_token"]


def workload_token(client: str, audience: str, scopes: list[str]) -> str:
    return must(call("POST", ISSUER + "/_mint", {
        "subject": client, "client": client, "audience": audience, "scopes": scopes,
        "actor_type": "workload"}), 200)["access_token"]


class HyperSwitch:
    """The merchant account, its API key and profile, the Stripe connector
    (pointed at the processor), the buyer and their saved cards."""

    def __init__(self, evidence):
        self.admin = {"api-key": "test_admin"}
        self.merchant = "merchant_d1"
        self.evidence = evidence

    def seed(self) -> dict:
        # A kept stack (GATE_D_KEEP=1) already has the merchant: reuse it.
        must(call("POST", HYPERSWITCH + "/accounts", {
            "merchant_id": self.merchant, "merchant_name": "Baobab Gate D1 synthetic merchant",
            "locker_id": "m0010", "primary_business_details": [{"country": MARKET, "business": "default"}]},
            self.admin), 200, 400)
        self.key = must(call("POST", f"{HYPERSWITCH}/api_keys/{self.merchant}",
                             {"name": "gate-d1", "expiration": "never"}, self.admin), 200)["api_key"]
        self.profile = must(call("GET", f"{HYPERSWITCH}/accounts/{self.merchant}", headers=self.admin),
                            200)["default_profile"]
        connector = must(call("POST", f"{HYPERSWITCH}/account/{self.merchant}/connectors", {
            "connector_type": "payment_processor", "connector_name": "stripe", "connector_label": "gate_d1_processor",
            "profile_id": self.profile,
            "connector_account_details": {"auth_type": "HeaderKey", "api_key": "sk_test_gate_d1_synthetic"},
            "test_mode": True, "disabled": False,
            "payment_methods_enabled": [{"payment_method": "card", "payment_method_types": [
                {"payment_method_type": "credit", "card_networks": ["Visa"], "minimum_amount": 1,
                 "maximum_amount": 68607706, "recurring_enabled": True, "installment_payment_enabled": True}]}]},
            {"api-key": self.key}), 200, 400)
        self.connector = connector.get("merchant_connector_id") or "existing"
        merchant = {"api-key": self.key}
        must(call("POST", HYPERSWITCH + "/customers", {"customer_id": CUSTOMER, "name": "Synthetic Buyer"},
                  merchant), 200, 400)
        for card in CARDS.values():
            must(call("POST", HYPERSWITCH + "/payment_methods", {
                "payment_method": "card", "payment_method_type": "credit", "customer_id": CUSTOMER,
                "card": {"card_number": card, "card_exp_month": "10", "card_exp_year": "30",
                         "card_holder_name": "Synthetic Buyer"}}, merchant), 200)
        self.evidence.note("hyperswitch", {"merchant_id": self.merchant, "profile_id": self.profile,
                                           "connector": "stripe (unmodified)", "merchant_connector_id": self.connector,
                                           "customer_id": CUSTOMER})
        return {"api_key": self.key, "profile_id": self.profile}

    def token(self, which: str = "approve") -> str:
        """A fresh, single-use payment token for one of the buyer's saved cards."""
        listed = must(call("GET", f"{HYPERSWITCH}/customers/{CUSTOMER}/payment_methods",
                           headers={"api-key": self.key}), 200)
        last4 = CARDS[which][-4:]
        for method in listed["customer_payment_methods"]:
            if (method.get("card") or {}).get("last4_digits") == last4:
                return method["payment_token"]
        raise RuntimeError(f"no saved {which} card")

    def payment(self, payment_id: str) -> dict:
        return must(call("GET", f"{HYPERSWITCH}/payments/{payment_id}?force_sync=true",
                         headers={"api-key": self.key}), 200)

    def refund(self, refund_id: str):
        return call("GET", f"{HYPERSWITCH}/refunds/{refund_id}", headers={"api-key": self.key})


class ControlPlane:
    def __init__(self, compose: Compose, evidence):
        self.c, self.evidence = compose, evidence
        self.maker, self.checker, self.operator = (str(uuid.uuid4()) for _ in range(3))

    def sql(self, statement: str, label: str):
        self.evidence.note("synthetic-cp-seed", {"label": label})
        return self.c.sql("cpdb", "controlplane", "controlplane", statement)

    def scalar(self, statement: str) -> str:
        return self.c.scalar("cpdb", "controlplane", "controlplane", statement)

    def admin(self, method: str, path: str, who: str, body=None, headers=None, ok=(200, 201, 202)):
        h = {"Authorization": "Bearer " + human_token(who), **(headers or {})}
        return must(call(method, CP + path, body, h), *ok)

    def seed(self) -> dict:
        humans = ", ".join(f"('{p}','human','ACTIVE')" for p in (self.maker, self.checker, self.operator))
        links = ", ".join(f"('{p}','{ISS}','{p}','ACTIVE')" for p in (self.maker, self.checker, self.operator))
        self.sql(f"INSERT INTO identity.principal (principal_id, actor_type, status) VALUES {humans};"
                 f"INSERT INTO identity.external_identity (principal_id, issuer, subject, status) VALUES {links}",
                 "three human administrators (IAM user provisioning is Gate D2)")

        registration = self.admin("POST", "/v1/tenants/bootstrap-registrations", self.maker, {
            "legal_entity_id": LEGAL_ENTITY, "display_name": "Gate D1 Synthetic Merchant (Pty) Ltd",
            "isolation_strategy": "row_level_security", "residency_region": "af-south-1",
            "bootstrap_reason": "P0-04B Gate D1 synthetic merchant for money-movement acceptance",
            "evidence_reference": "gate-d1/synthetic-registration"},
            {"Idempotency-Key": str(uuid.uuid4())})
        self.tenant = registration["tenant_id"]
        for _ in range(60):
            self.organisation = self.scalar(
                "SELECT organisation_id FROM registry.tenant_organisation_mapping "
                f"WHERE tenant_id='{self.tenant}' AND mapping_role='PRIMARY_ORGANISATION' AND status='ACTIVE'")
            if self.organisation:
                break
            time.sleep(1)
        if not self.organisation:
            raise RuntimeError(f"tenant registration did not complete: {registration}")
        self.admin("POST", f"/v1/tenants/{self.tenant}/activate", self.maker)

        now = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(time.time() - 3600))
        created = call("POST", CP + "/v1/markets", {
            "canonical_key": "za.gate.d1", "name": "South Africa (Gate D1)", "owner_tenant_id": self.tenant,
            "market_type": "MARKETPLACE", "default_country": MARKET, "default_currency": CURRENCY,
            "default_locale": "en-ZA", "timezone": "Africa/Johannesburg", "effective_from": now},
            {"Authorization": "Bearer " + human_token(self.maker), "Idempotency-Key": str(uuid.uuid4())})
        must(created, 201)
        market_id = created.body["market_id"]
        self.admin("POST", f"/v1/markets/{market_id}/activate", self.checker, {},
                   {"If-Match": created.headers.get("etag", '"1"')})
        self.sql(f"""INSERT INTO market.market_assignment (tenant_id, market_id, status, source, policy_version,
                       effective_from)
                     SELECT '{self.tenant}', m.market_id, 'ACTIVE', 'PROVISIONING', '1', now() - interval '1 hour'
                       FROM market.market m WHERE m.code = '{MARKET}' AND m.is_active;
                     INSERT INTO market.market_participation_capability (market_assignment_id, capability)
                     SELECT market_assignment_id, 'SELLING' FROM market.market_assignment WHERE tenant_id='{self.tenant}'""",
                 "tenant market assignment (tenant provisioning has no admin API)")

        self.sql(f"""UPDATE registry.legal_entity_profile
                        SET verification_state='VERIFIED', legal_status='ACTIVE',
                            source_authority='synthetic-independent-legal-registry',
                            evidence_references='["gate-d1/synthetic-legal-registry"]'::jsonb,
                            verified_by='{self.checker}', verified_at=clock_timestamp() - interval '1 minute',
                            updated_at=clock_timestamp()
                      WHERE legal_entity_id='{LEGAL_ENTITY}'""",
                 "legal-entity verification (an independent legal registry)")

        self.engine_instance = self.scalar(f"""
            WITH e AS (
              INSERT INTO topology.engine (code, name) VALUES ('baobab-payments', 'Baobab Payments')
              ON CONFLICT (code) DO UPDATE SET name = EXCLUDED.name RETURNING engine_id),
            i AS (
              INSERT INTO topology.engine_instance (engine_id, region, environment, status)
              SELECT engine_id, 'af-south-1', 'integration', 'ACTIVE' FROM e
              RETURNING engine_instance_id, engine_id, engine_instance_key),
            c AS (
              INSERT INTO capability.capability (code, name) VALUES ('payments.gate-d1', 'Gate D1 payments')
              RETURNING capability_id),
            s AS (INSERT INTO capability.capability_scope (tenant_id) VALUES ('{self.tenant}') RETURNING scope_id),
            p AS (
              INSERT INTO capability.capability_provider (provider_key, name, provider_type, engine_id, status)
              SELECT 'gate-d1.payments', 'Gate D1 Payments', 'BAOBAB_ENGINE', engine_id, 'ACTIVE' FROM e
              RETURNING provider_id),
            b AS (
              INSERT INTO capability.capability_binding (capability_id, engine_instance_id, scope_id, binding_mode,
                  status, contract_version, effective_from, provider_id)
              SELECT c.capability_id, i.engine_instance_id, s.scope_id, 'PRIMARY', 'ACTIVE', '1',
                     now() - interval '1 hour', p.provider_id FROM c, i, s, p)
            SELECT engine_instance_key FROM i""")
        self.evidence.note("synthetic-cp-seed", {"label": "baobab-payments engine instance and binding",
                                                 "engine_instance_key": self.engine_instance})

        self.mandate = self.propose(effective_minutes_ago=5)
        self.evidence.note("cp", {"tenant_id": self.tenant, "organisation_id": self.organisation,
                                  "legal_entity_id": LEGAL_ENTITY, "market_id": market_id,
                                  "mandate_id": self.mandate, "engine_instance_key": self.engine_instance})
        return {"tenant": self.tenant, "organisation": self.organisation, "engine_instance": self.engine_instance}

    def propose(self, effective_minutes_ago: int) -> str:
        frm = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(time.time() - effective_minutes_ago * 60))
        proposed = self.admin("POST", "/v2/legal-actor-mandates", self.maker, {
            "tenant_id": self.tenant, "operating_organisation_id": self.organisation,
            "responsible_legal_entity_id": LEGAL_ENTITY, "roles": ["PAYMENT_BENEFICIARY"],
            "activity_scope": ACTIVITIES, "market_scope": [MARKET], "capability_scope": CAPABILITIES,
            "authority_basis_reference": "gate-d1/board-resolution", "evidence_references": ["gate-d1/mandate-evidence"],
            "effective_from": frm}, {"Idempotency-Key": str(uuid.uuid4())})
        mandate = proposed["mandate_id"]
        self.admin("POST", f"/v2/legal-actor-mandates/{mandate}/decision", self.checker, {
            "decision": "APPROVE", "decision_basis_reference": "gate-d1/four-eyes-review",
            "evidence_references": ["gate-d1/mandate-evidence"],
            "legal_actor_verification_reference": "gate-d1/synthetic-legal-registry"},
            {"Idempotency-Key": str(uuid.uuid4())})
        self.admin("POST", f"/v2/legal-actor-mandates/{mandate}/activate", self.operator, {
            "action": "ACTIVATE", "authority_basis_reference": "gate-d1/activation",
            "evidence_references": ["gate-d1/mandate-evidence"]}, {"Idempotency-Key": str(uuid.uuid4())})
        return mandate

    def revoke(self, mandate: str) -> None:
        self.admin("POST", f"/v2/legal-actor-mandates/{mandate}/terminate", self.operator, {
            "action": "REVOKE", "authority_basis_reference": "gate-d1/revocation",
            "evidence_references": ["gate-d1/revocation-evidence"]}, {"Idempotency-Key": str(uuid.uuid4())})


def payments_activation(compose: Compose, cp: dict, profile: str, evidence, kinds: str = "AUTHORIZE,CAPTURE,CANCEL,REFUND") -> str:
    """Provider readiness, recorded by the operator (the database owner): the
    runtime role can read it and can never grant it to itself."""
    activation = compose.scalar("paydb", "baobab", "baobab_payments", f"""
        INSERT INTO payment_ops.provider_activation (tenant_id, responsible_legal_entity_id,
            merchant_profile_reference, market_code, currency_code, operation_kinds, provider_environment,
            provider_certification_reference, merchant_activation_reference, valid_from, valid_until)
        VALUES ('{cp["tenant"]}', '{LEGAL_ENTITY}', '{profile}', '{MARKET}', '{CURRENCY}',
                string_to_array('{kinds}', ','), 'SANDBOX', 'gate-d1/test-processor-certification',
                'gate-d1/merchant-activation', now() - interval '1 minute', now() + interval '1 day')
        RETURNING activation_id""")
    evidence.note("synthetic-payments-seed", {"label": "provider activation (test processor, SANDBOX)",
                                              "activation_id": activation, "kinds": kinds})
    return activation


def merchant_bindings(cp: dict, profile: str) -> str:
    return json.dumps({"bindings": [{
        "tenant_id": cp["tenant"], "responsible_legal_entity_id": LEGAL_ENTITY,
        "operating_organisation_id": cp["organisation"], "market": MARKET, "currency": CURRENCY,
        "merchant_profile_reference": profile, "provider_merchant_id": "merchant_d1",
        "provider_engine_instance_id": "hyperswitch-gate-d1",
        "certification_reference": "gate-d1/test-processor-certification",
        "merchant_activation_reference": "gate-d1/merchant-activation",
        "platform_account_ids": [CUSTOMER]}]})
