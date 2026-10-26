#!/usr/bin/env python3
"""DEV-15 Payments owner adapter for the private simulated Rust service.

Legacy health probing remains separate from the connected container phases.
The latter consume genuine IAM/CP references and exercise actual Rust HTTP
effects. No phase issues identity, verifies a legal mandate or certifies a PSP.
"""
from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request

PHASES = ("readiness", "migrate", "seed", "exercise", "cleanup")
CONNECTED_PHASES = ("prepare", "test", "observe", "unavailable", "denied", "restored")
BLOCKERS = {
    "readiness": "PAYMENTS_NATIVE_SERVICE_UNAVAILABLE",
    "migrate": "PAYMENTS_NATIVE_STORE_MIGRATION_NOT_WIRED",
    "seed": "CP_GOVERNED_PAYMENTS_SEED_NOT_WIRED",
    "exercise": "REAL_IAM_CP_PAYMENTS_QUALIFIER_NOT_WIRED",
    "cleanup": "INDEPENDENT_TEARDOWN_WITNESS_REQUIRED",
}
LOOPBACK = "http://127.0.0.1:8080/health/ready"


def probe(endpoint: str) -> bool:
    # Avoid host-supplied endpoint overrides that can probe secrets or
    # protected services. Infrastructure owns network and lease boundaries.
    if endpoint != LOOPBACK:
        return False
    try:
        with urllib.request.urlopen(endpoint, timeout=3) as response:  # noqa: S310
            if response.status != 200:
                return False
            body = response.read(4097)
            if len(body) > 4096:
                return False
            payload = json.loads(body)
            return (
                isinstance(payload, dict)
                and payload.get("status") == "READY"
                and isinstance(payload.get("checks"), dict)
                and payload["checks"].get("payment_provider") == "UP"
                and payload.get("payment_provider") == {"kind": "SANDBOX", "simulated": True}
                and payload.get("real_payments") == "NOT_CONFIGURED"
                and payload.get("store") == "in-memory"
                and payload.get("environment") == "development"
            )
    except (OSError, ValueError, urllib.error.URLError):
        return False


def run(phase: str) -> tuple[int, dict]:
    if phase not in PHASES:
        raise ValueError("UNKNOWN_OWNER_PHASE")
    healthy = phase == "readiness" and probe(LOOPBACK)
    result = {
        "schema": "baobab.engine-development-adapter-result.v1",
        "engine": "baobab-payments",
        "phase": phase,
        "result": "PASS" if healthy else "BLOCKED",
        "reason": "NATIVE_HEALTH_OBSERVED" if healthy else BLOCKERS[phase],
        "evidence_class": "REAL_HEALTH_ONLY" if healthy else "SOURCE",
        "simulated_money_only": True,
        "native_money_effect_observed": False,
        "eng_onb_runtime_qualified": False,
        "protected_acceptance": False,
        "production_authorized": False,
    }
    return (0 if healthy else 2), result


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=PHASES+CONNECTED_PHASES)
    args = parser.parse_args(argv)
    if args.phase in CONNECTED_PHASES:
        try:
            print(json.dumps(connected(args.phase), sort_keys=True))
            return 0
        except Exception:
            print("PAYMENTS_CONNECTED_ADAPTER_REFUSED", file=sys.stderr)
            return 2
    status, receipt = run(args.phase)
    print(json.dumps(receipt, sort_keys=True))
    return status




def connected(phase: str) -> dict:
    import uuid
    from engine_sdk import Driver

    driver = Driver()
    custody = driver.context
    if custody.get("schema") != "baobab.engine-adapter-context.v1":
        raise ValueError("PRIVATE_ENGINE_CONTEXT_REQUIRED")
    tenant = custody["tenant"]["id"]
    cases = {name: {"result": "BLOCKED", "reason": "NOT_EXECUTED"} for name in (
        "PAYMENTS-INTENT-CONFIRM-AUTHORIZE-CAPTURE", "PAYMENTS-CANCEL", "PAYMENTS-REFUND", "PAYMENTS-OVER-REFUND",
        "PAYMENTS-IDEMPOTENCY-CONFLICT", "PAYMENTS-LOST-RESPONSE-RETRY", "PAYMENTS-CROSS-TENANT",
        "PAYMENTS-ACTOR-MARKET-CURRENCY", "PAYMENTS-REVOKED-GRANT", "PAYMENTS-EVENT-REPLAY",
        "PAYMENTS-UNTRUSTED-WEBHOOK", "PAYMENTS-MISSING-MANDATE", "PAYMENTS-PSP-NOT-CONFIGURED", "PAYMENTS-SIMULATION-TRUTH",
    )}
    refs = {}

    def check(value):
        if not value:
            raise ValueError("NATIVE_ASSERTION_FAILED")

    def request_body(capture="MANUAL"):
        return {"context": {"tenant_id": tenant, "legal_entity_id": custody["tenant"]["legal_entity_id"],
            "market_id": custody["tenant"]["market_id"], "currency": "ZAR" if custody["tenant"]["country"] == "ZA" else "UGX",
            "source_engine": "baobab-payments", "source_reference": "dev15:"+custody["run"]["id"], "correlation_id": str(uuid.uuid4())},
            "amount": {"currency": "ZAR" if custody["tenant"]["country"] == "ZA" else "UGX", "amount_minor": 1000}, "capture_method": capture}

    def send(path, body=None, key=None, token=None, context=None, method="POST"):
        witness = custody["witness"]
        return driver.native(path, body, token or witness["caller_token"], context or witness["context"]["context_id"], key or "dev15-"+uuid.uuid4().hex, method)

    def observe(name, work):
        try:
            work()
            cases[name] = {"result": "PASS", "reason": "OBSERVED"}
        except Exception:
            cases[name] = {"result": "FAIL", "reason": "ASSERTION_FAILED"}

    if phase == "prepare":
        return {"prepared": True}  # Native sandbox is explicitly process-local.
    if phase == "observe":
        refs = custody.get("native_refs", {})
        if refs.get("intent_id"):
            code, intent = send("/v1/tenants/"+tenant+"/payment-intents/"+refs["intent_id"], method="GET")
            check(code == 200 and intent["provider"] == {"kind": "SANDBOX", "simulated": True})
        return {"effects": {}, "resolutions": [], "checks": {"native_reads": True, "simulated_only": True}}
    if phase in {"denied", "unavailable", "restored"}:
        key = "dev15-outage-"+custody["run"]["id"]+"-"+custody.get("fault_phase", "cp")
        body = request_body()
        # Correlation is stable across lost-response and recovered retries.
        body["context"]["correlation_id"] = custody["witness"]["context"]["context_id"]
        code, _ = send("/v1/payment-intents", body, key)
        if phase == "restored":
            check(code == 201 and driver.last_headers.get("idempotent-replayed") != "true")
        elif phase == "unavailable":
            check(code == 503)
        else:
            check(code in (403, 404))
        return {"observed": True}
    check(phase == "test")
    created_body = request_body()
    key = "dev15-intent-"+uuid.uuid4().hex

    def lifecycle():
        code, intent = send("/v1/payment-intents", created_body, key)
        check(code == 201 and intent["provider"] == {"kind": "SANDBOX", "simulated": True})
        refs["intent_id"] = intent["payment_intent_id"]
        code, authorised = send("/v1/payment-intents/"+refs["intent_id"]+"/confirm", {"tenant_id": tenant, "payment_method_reference": "pm_dev15_synthetic"})
        check(code == 200 and authorised["status"] == "REQUIRES_CAPTURE")
        refs["payment_id"] = authorised["payment_id"]
        code, captured = send("/v1/payments/"+refs["payment_id"]+"/capture", {"tenant_id": tenant, "amount_minor": 1000})
        check(code == 200 and captured["captured_amount_minor"] == 1000 and captured["provider"]["simulated"] is True)
    observe("PAYMENTS-INTENT-CONFIRM-AUTHORIZE-CAPTURE", lifecycle)

    def cancel():
        code, intent = send("/v1/payment-intents", request_body())
        check(code == 201)
        code, cancelled = send("/v1/payment-intents/"+intent["payment_intent_id"]+"/cancel", {"tenant_id": tenant, "reason": "DEV-15 cancellation"})
        check(code == 200 and cancelled["status"] == "CANCELLED" and cancelled["provider"]["simulated"] is True)
    observe("PAYMENTS-CANCEL", cancel)

    def refund():
        code, result = send("/v1/payments/"+refs["payment_id"]+"/refunds", {"tenant_id": tenant, "amount_minor": 200, "reason": "DEV-15 simulation"})
        check(code == 201 and result["provider"]["simulated"] is True)
        refs["refund_id"] = result["refund_id"]
    observe("PAYMENTS-REFUND", refund)
    observe("PAYMENTS-OVER-REFUND", lambda: check(send("/v1/payments/"+refs["payment_id"]+"/refunds", {"tenant_id": tenant, "amount_minor": 1001, "reason": "DEV-15 denied excess"})[0] in (409, 422)))
    observe("PAYMENTS-IDEMPOTENCY-CONFLICT", lambda: check(send("/v1/payment-intents", {**created_body, "description": "changed"}, key)[0] == 409))

    def lost_response():
        replay_key = "dev15-lost-"+uuid.uuid4().hex
        request = request_body()
        code, initial = send("/v1/payment-intents", request, replay_key)
        check(code == 201)
        # Deliberately discard the first response in the caller protocol;
        # independent witness retains it solely for duplicate-effect checking.
        code, replay = send("/v1/payment-intents", request, replay_key)
        check(code == 201 and replay == initial and driver.last_headers.get("idempotent-replayed") == "true")
    observe("PAYMENTS-LOST-RESPONSE-RETRY", lost_response)

    def isolation():
        other = custody["witness"]["neighbour_context"]["context_id"]
        check(send("/v1/tenants/"+tenant+"/payment-intents/"+refs["intent_id"], context=other, method="GET")[0] in (403, 404))
        wrong = request_body()
        wrong["context"]["tenant_id"] = custody["neighbour"]["id"]
        check(send("/v1/payment-intents", wrong)[0] in (403, 404))
    observe("PAYMENTS-CROSS-TENANT", isolation)

    def currency_market():
        wrong = request_body()
        wrong["context"]["market_id"] = "mkt_unknown"
        check(send("/v1/payment-intents", wrong)[0] == 403)
        wrong = request_body()
        wrong["context"]["source_engine"] = "baobab-subscriptions"
        check(send("/v1/payment-intents", wrong)[0] == 403)
        wrong = request_body()
        wrong["amount"]["currency"] = "USD"
        check(send("/v1/payment-intents", wrong)[0] == 422)
    observe("PAYMENTS-ACTOR-MARKET-CURRENCY", currency_market)
    # The checks above prove routing denials, not a current legal mandate.
    if cases["PAYMENTS-ACTOR-MARKET-CURRENCY"]["result"] == "PASS":
        cases["PAYMENTS-ACTOR-MARKET-CURRENCY"] = {"result": "BLOCKED", "reason": "AUTHORITY_UNQUALIFIED"}

    def credentials():
        for name, statuses in (("revoked", (401,)), ("untrusted_issuer", (401,)), ("expired", (401,)), ("wrong_audience", (401,)), ("missing_scope", (403,))):
            check(send("/v1/payment-intents", request_body(), token=custody["security_tokens"][name])[0] in statuses)
    observe("PAYMENTS-REVOKED-GRANT", credentials)
    # Current grant withdrawal is also observed independently by the common
    # controller using the normal CP owner command and the native denial phase.
    observe("PAYMENTS-UNTRUSTED-WEBHOOK", lambda: check(send("/v1/webhooks/untrusted", {"payment_id": refs["payment_id"], "status": "CAPTURED"})[0] == 404))
    code, health = driver.request("GET", "https://engine.baobab-dev.test/health/ready")
    observe("PAYMENTS-PSP-NOT-CONFIGURED", lambda: check(code == 200 and health["real_payments"] == "NOT_CONFIGURED" and health["payment_provider"] == {"kind": "SANDBOX", "simulated": True}))
    observe("PAYMENTS-SIMULATION-TRUTH", lambda: check(code == 200 and health["store"] == "in-memory" and health["environment"] == "development" and health["payment_provider"]["simulated"] is True))
    cases["PAYMENTS-MISSING-MANDATE"] = {"result": "BLOCKED", "reason": "AUTHORITY_UNQUALIFIED"}
    cases["PAYMENTS-EVENT-REPLAY"] = {"result": "BLOCKED", "reason": "NATIVE_NOT_IMPLEMENTED"}
    return {"cases": cases, "native_refs": refs}


if __name__ == "__main__":
    sys.exit(main())
