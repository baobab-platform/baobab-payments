#!/usr/bin/env python3
"""P0-04B Gate D1 scenarios for Payments (ADR-SHARED-035).

Every scenario drives the release Payments service through its public HTTP
routes with a real signed workload token, and asserts what persisted on all
three sides: the Payments journal, CP's decision history, HyperSwitch's own
record and the test processor's call log. A scenario passes only when every
assertion holds; the report ends with `Gate D1 failures: N`.
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import traceback
import uuid

sys.path.insert(0, os.path.dirname(__file__))
from d1lib import PAYMENTS, PROCESSOR, ISSUER, Compose, call, key, wait_for  # noqa: E402
import seed  # noqa: E402

TRADE = "baobab-trade-workload"


class Evidence:
    def __init__(self, out: str):
        self.out, self.notes, self.results = out, [], []

    def note(self, kind: str, data: dict) -> None:
        self.notes.append({"kind": kind, **data})


class Failure(AssertionError):
    pass


def check(condition: bool, message: str) -> None:
    if not condition:
        raise Failure(message)


class GateD1:
    def __init__(self, args):
        self.args = args
        self.c = Compose(args.compose)
        self.ev = Evidence(args.out)
        self.hs = seed.HyperSwitch(self.ev)
        self.cp = seed.ControlPlane(self.c, self.ev)

    # --- the stack -------------------------------------------------------
    def payments_env(self) -> dict:
        return {"PAYMENTS_ENGINE_INSTANCE_ID": self.ids["engine_instance"]}

    def start_payments(self) -> None:
        self.c.run("up", "-d", "--no-deps", "payments", env=self.payments_env())
        wait_for(PAYMENTS + "/health/live", 60)

    def kill_payments(self) -> None:
        self.c.run("kill", "payments", env=self.payments_env())

    def seed(self) -> None:
        hs = self.hs.seed()
        open(os.path.join(self.args.run, "payments", "hyperswitch-api-key"), "w").write(hs["api_key"])
        self.ids = self.cp.seed()
        self.profile = hs["profile_id"]
        self.activation = seed.payments_activation(self.c, self.ids, self.profile, self.ev)
        open(os.path.join(self.args.run, "payments", "merchant-bindings.json"), "w").write(
            seed.merchant_bindings(self.ids, self.profile))
        self.start_payments()

    # --- calling Payments as the Trade workload --------------------------
    def token(self, scopes=("payment:execute", "payment:refund", "payment:read"), client=TRADE,
              audience="baobab-payments") -> str:
        return seed.workload_token(client, audience, list(scopes))

    def pay(self, method: str, path: str, body=None, idem: str | None = None, token: str | None = None):
        headers = {"Authorization": "Bearer " + (token or self.token()), "X-Correlation-ID": str(uuid.uuid4())}
        if idem:
            headers["Idempotency-Key"] = idem
        return call(method, PAYMENTS + path, body, headers, timeout=90)

    def intent_body(self, amount: int, **context) -> dict:
        ctx = {"tenant_id": self.ids["tenant"], "legal_entity_id": seed.LEGAL_ENTITY,
               "platform_account_id": seed.CUSTOMER, "currency": seed.CURRENCY, "source_engine": "baobab-trade",
               "source_reference": "order/" + uuid.uuid4().hex[:10], "correlation_id": str(uuid.uuid4())}
        ctx.update(context)
        return {"context": ctx, "amount": {"currency": seed.CURRENCY, "amount_minor": amount},
                "capture_method": "MANUAL"}

    def create(self, amount: int = 1000) -> dict:
        r = self.pay("POST", "/v1/payment-intents", self.intent_body(amount), key("create"))
        check(r.status == 201, f"create intent: {r!r}")
        return r.body

    def authorise(self, amount: int = 1000, card: str = "approve") -> tuple[dict, dict]:
        intent = self.create(amount)
        r = self.pay("POST", f"/v1/payment-intents/{intent['payment_intent_id']}/confirm",
                     {"tenant_id": self.ids["tenant"], "payment_method_reference": self.hs.token(card)},
                     key("confirm"))
        check(r.status == 200, f"confirm: {r!r}")
        return intent, r.body

    def capture(self, payment: str, amount: int | None = None, idem: str | None = None):
        body = {"tenant_id": self.ids["tenant"]}
        if amount is not None:
            body["amount_minor"] = amount
        return self.pay("POST", f"/v1/payments/{payment}/capture", body, idem or key("capture"))

    def refund(self, payment: str, amount: int, idem: str | None = None, reason: str = "buyer returned goods"):
        return self.pay("POST", f"/v1/payments/{payment}/refunds",
                        {"tenant_id": self.ids["tenant"], "amount_minor": amount, "reason": reason},
                        idem or key("refund"))

    def cancel(self, intent: str, idem: str | None = None):
        return self.pay("POST", f"/v1/payment-intents/{intent}/cancel",
                        {"tenant_id": self.ids["tenant"], "reason": "buyer cancelled"}, idem or key("cancel"))

    def read_payment(self, payment: str):
        return self.pay("GET", f"/v1/tenants/{self.ids['tenant']}/payments/{payment}")

    # --- evidence on every side -----------------------------------------
    def journal(self, statement: str) -> list[list[str]]:
        return self.c.sql("paydb", "baobab", "baobab_payments", statement)

    def movements(self, payment: str) -> list[dict]:
        rows = self.journal(f"""
            SELECT e.operation_kind, e.amount_minor, coalesce(payment_ops.execution_outcome(e.execution_id),'NONE'),
                   (SELECT count(*) FROM payment_ops.authority_decision d WHERE d.operation_reference=e.operation_reference),
                   (SELECT count(*) FROM payment_ops.dispatch_attempt p WHERE p.execution_id=e.execution_id),
                   (SELECT string_agg(o.source || ':' || o.outcome, ',' ORDER BY o.observed_at)
                      FROM payment_ops.provider_observation o WHERE o.execution_id=e.execution_id),
                   e.operation_reference
              FROM payment_ops.native_execution e
             WHERE e.canonical_payment_reference='{payment}' OR e.parent_reference='{payment}'
             ORDER BY e.recorded_at""")
        return [{"kind": r[0], "amount": int(r[1]), "outcome": r[2], "decisions": int(r[3]), "dispatches": int(r[4]),
                 "observations": r[5], "operation_reference": r[6]} for r in rows]

    def processor(self) -> dict:
        return call("GET", PROCESSOR + "/_evidence").body

    def processor_calls(self, suffix: str) -> int:
        return sum(1 for c in self.processor()["calls"] if c["path"].endswith(suffix) and c["method"] == "POST")

    def cp_decisions(self, outcome: str | None = None) -> int:
        where = {None: "", "DENIED": " WHERE outcome <> 'AUTHORIZED'"}.get(outcome, f" WHERE outcome='{outcome}'")
        return int(self.c.scalar("cpdb", "controlplane", "controlplane",
                                 f"SELECT count(*) FROM registry.payments_money_movement_decision{where}") or 0)

    def fault(self, mode: str, path_contains: str, count: int = 1) -> None:
        call("POST", PROCESSOR + "/_fault", {"mode": mode, "method": "POST", "path_contains": path_contains,
                                             "count": count})

    # --- scenarios -----------------------------------------------------------
    def s01_authorise_capture_refund(self):
        """Manual authorisation, full capture and full refund: one provider
        operation each, two CP decisions each, and read-back consistency."""
        before = self.processor_calls("/capture")
        intent, confirmed = self.authorise(1000)
        check(confirmed["status"] == "REQUIRES_CAPTURE", f"after authorisation: {confirmed['status']}")
        payment = confirmed["payment_id"]
        check(self.hs.payment(payment)["status"] == "requires_capture", "HyperSwitch is not requires_capture")
        r = self.capture(payment)
        check(r.status == 200 and r.body["status"] == "CAPTURED" and r.body["captured_amount_minor"] == 1000, f"capture: {r!r}")
        r = self.refund(payment, 1000)
        check(r.status == 201 and r.body["status"] == "SUCCEEDED", f"refund: {r!r}")
        m = self.movements(payment)
        kinds = [x["kind"] for x in m]
        check(kinds == ["CREATE_INTENT", "AUTHORIZE", "CAPTURE", "REFUND"], f"journal movements {kinds}")
        for x in m[1:]:
            check(x["outcome"] == "CONFIRMED" and x["decisions"] == 2 and x["dispatches"] == 1,
                  f"movement {x}")
        check(self.processor_calls("/capture") == before + 1, "processor saw other than one capture")
        hs = self.hs.payment(payment)
        read = self.read_payment(payment).body
        check(read["status"] == "REFUNDED" and read["captured_amount_minor"] == 1000
              and read["refunded_amount_minor"] == 1000, f"Payments read-back {read}")
        check(hs.get("amount_received") == 1000, f"HyperSwitch amount_received {hs.get('amount_received')}")
        outbox = self.journal("SELECT count(*) FROM payment_ops.transaction_outbox o JOIN payment_ops.native_execution e "
                              f"USING (execution_id) WHERE e.canonical_payment_reference='{payment}' OR e.parent_reference='{payment}'")
        check(int(outbox[0][0]) >= 10, f"outbox events {outbox}")
        return {"payment": payment, "movements": m, "hyperswitch_status": hs["status"]}

    def s02_partial_capture_and_refunds(self):
        """One partial capture releases the remainder; partial refunds up to the
        captured amount; an over-limit refund is refused before any provider call."""
        _, confirmed = self.authorise(1000)
        payment = confirmed["payment_id"]
        r = self.capture(payment, 600)
        check(r.status == 200 and r.body["captured_amount_minor"] == 600, f"partial capture: {r!r}")
        hs = self.hs.payment(payment)
        check(hs.get("amount_received") == 600, f"HyperSwitch received {hs.get('amount_received')}")
        captures = self.processor_calls("/capture")
        r = self.capture(payment, 400)
        check(r.status == 422 and r.body["code"] == "PAYMENT_PROVIDER_REJECTED",
              f"the released remainder must not be capturable: {r!r}")
        check(self.processor_calls("/capture") <= captures + 1, "more than one attempt at the remainder")
        check(self.refund(payment, 200).status == 201, "first partial refund")
        check(self.refund(payment, 400).status == 201, "second partial refund")
        refunds = self.processor_calls("/v1/refunds")
        r = self.refund(payment, 1)
        check(r.status == 422 and r.body["code"] == "REFUND_EXCEEDS_CAPTURED", f"over-limit refund: {r!r}")
        check(self.processor_calls("/v1/refunds") == refunds, "the over-limit refund reached the processor")
        read = self.read_payment(payment).body
        check(read["captured_amount_minor"] == 600 and read["refunded_amount_minor"] == 600
              and read["status"] == "REFUNDED", f"read-back {read}")
        return {"payment": payment, "hyperswitch_status": hs["status"], "movements": self.movements(payment)}

    def s03_overcapture_and_double_capture(self):
        """Overcapture is refused by Payments' own approval; a second full
        capture is refused by the journal; neither reaches the processor."""
        _, confirmed = self.authorise(1000)
        payment = confirmed["payment_id"]
        captures = self.processor_calls("/capture")
        r = self.capture(payment, 1500)
        check(r.status == 422 and r.body["code"] == "CAPTURE_EXCEEDS_AUTHORIZED", f"overcapture: {r!r}")
        check(self.capture(payment).status == 200, "full capture")
        r = self.capture(payment, 1000)
        check(r.status == 422 and r.body["code"] == "CAPTURE_EXCEEDS_AUTHORIZED", f"double capture: {r!r}")
        check(self.processor_calls("/capture") == captures + 1, "a refused capture reached the processor")
        return {"payment": payment, "movements": self.movements(payment)}

    def s04_void_then_capture(self):
        """A void releases the authorisation; a capture after it is refused."""
        intent, confirmed = self.authorise(800)
        payment = confirmed["payment_id"]
        r = self.cancel(intent["payment_intent_id"])
        check(r.status == 200 and r.body["status"] == "CANCELLED", f"void: {r!r}")
        check(self.hs.payment(payment)["status"] == "cancelled", "HyperSwitch is not cancelled")
        captures = self.processor_calls("/capture")
        r = self.capture(payment)
        check(r.status == 409 and r.body["code"] == "INVALID_PAYMENT_STATE", f"capture after void: {r!r}")
        check(self.processor_calls("/capture") == captures, "capture after void reached the processor")
        return {"payment": payment, "movements": self.movements(payment)}

    def s05_void_after_capture(self):
        intent, confirmed = self.authorise(700)
        payment = confirmed["payment_id"]
        check(self.capture(payment).status == 200, "capture")
        cancels = self.processor_calls("/cancel")
        r = self.cancel(intent["payment_intent_id"])
        check(r.status == 409 and r.body["code"] == "INVALID_PAYMENT_STATE", f"void after capture: {r!r}")
        check(self.processor_calls("/cancel") == cancels, "void after capture reached the processor")
        return {"payment": payment}

    def s06_idempotent_replay_and_key_reuse(self):
        """An identical retry replays without a provider call; the same key for
        a different request is a conflict; a duplicate refund moves nothing."""
        _, confirmed = self.authorise(1000)
        payment = confirmed["payment_id"]
        idem = key("capture-replay")
        check(self.capture(payment, None, idem).status == 200, "capture")
        captures = self.processor_calls("/capture")
        r = self.capture(payment, None, idem)
        check(r.status == 200 and r.headers.get("idempotent-replayed") == "true", f"replay: {r!r}")
        r = self.capture(payment, 10, idem)
        check(r.status == 409 and r.body["code"] == "IDEMPOTENCY_KEY_REUSED", f"changed request: {r!r}")
        check(self.processor_calls("/capture") == captures, "a replay reached the processor")
        refund_key = key("refund-replay")
        check(self.refund(payment, 300, refund_key).status == 201, "refund")
        refunds = self.processor_calls("/v1/refunds")
        r = self.refund(payment, 300, refund_key)
        check(r.status == 201 and r.headers.get("idempotent-replayed") == "true", f"duplicate refund: {r!r}")
        check(self.processor_calls("/v1/refunds") == refunds, "a duplicate refund reached the processor")
        check(self.read_payment(payment).body["refunded_amount_minor"] == 300, "refunded twice")
        return {"payment": payment}

    def s07_decline(self):
        intent, confirmed = self.authorise(500, card="decline")
        check(confirmed["status"] == "FAILED", f"declined authorisation: {confirmed['status']}")
        m = self.movements(confirmed.get("payment_id") or "none")
        check(any(x["kind"] == "AUTHORIZE" and x["outcome"] == "REJECTED" for x in m), f"journal {m}")
        return {"intent": intent["payment_intent_id"]}

    def s08_revocation_between_decision_and_dispatch(self):
        """CP revokes the mandate after the reservation's decision and before
        the execution-point decision: denied, reserved, never dispatched."""
        _, confirmed = self.authorise(900)
        payment = confirmed["payment_id"]
        captures = self.processor_calls("/capture")
        # Token requests per movement: context, decision 1, documentary, decision 2.
        call("POST", ISSUER + "/_hold", {"client": seed.PAYMENTS_CLIENT, "skip": 3})
        import threading
        result = {}
        worker = threading.Thread(target=lambda: result.update(r=self.capture(payment)))
        worker.start()
        try:
            for _ in range(60):
                if call("GET", ISSUER + "/_held").body["held"]:
                    break
                time.sleep(0.5)
            check(call("GET", ISSUER + "/_held").body["held"], "the execution-point token request was never held")
            self.cp.revoke(self.cp.mandate)
        finally:
            call("POST", ISSUER + "/_hold", {"client": None})
            call("POST", ISSUER + "/_release", {})
            worker.join(120)
        r = result["r"]
        check(r.status == 403 and r.body["code"] == "PAYMENT_AUTHORITY_DENIED", f"after revocation: {r!r}")
        m = [x for x in self.movements(payment) if x["kind"] == "CAPTURE"]
        check(len(m) == 1 and m[0]["dispatches"] == 0 and m[0]["decisions"] == 1, f"journal {m}")
        check(self.processor_calls("/capture") == captures, "a revoked capture reached the processor")
        denied = self.cp_decisions("DENIED")
        check(denied >= 1, "CP recorded no denial")
        # A new mandate restores authority for the remaining scenarios.
        self.cp.mandate = self.cp.propose(effective_minutes_ago=1)
        return {"payment": payment, "cp_denials": denied}

    def s09_cp_unavailable(self):
        _, confirmed = self.authorise(400)
        payment = confirmed["payment_id"]
        captures = self.processor_calls("/capture")
        self.c.run("stop", "cp")
        try:
            r = self.capture(payment)
        finally:
            self.c.run("start", "cp")
            wait_for("http://127.0.0.1:18082/healthz", 60)
        check(r.status == 503 and r.body["retryable"] is True, f"CP down: {r!r}")
        check(self.processor_calls("/capture") == captures, "a capture moved without CP")
        check(self.capture(payment).status == 200, "capture once CP is back")
        return {"payment": payment}

    def s10_provider_activation_withdrawn(self):
        """CP authorises, Payments' own readiness refuses: no provider call."""
        _, confirmed = self.authorise(300)
        payment = confirmed["payment_id"]
        captures = self.processor_calls("/capture")
        self.journal(f"INSERT INTO payment_ops.provider_activation_revocation (activation_id, tenant_id, "
                     f"responsible_legal_entity_id, reason) VALUES ('{self.activation}', '{self.ids['tenant']}', "
                     f"'{seed.LEGAL_ENTITY}', 'gate-d1 withdrawal')")
        r = self.capture(payment)
        check(r.status == 503 and r.body["code"] == "PAYMENT_PROVIDER_NOT_READY", f"withdrawn activation: {r!r}")
        check(self.processor_calls("/capture") == captures, "a capture moved without provider readiness")
        self.activation = seed.payments_activation(self.c, self.ids, self.profile, self.ev)
        check(self.capture(payment).status == 200, "capture once re-activated")
        return {"payment": payment}

    def s11_forged_context(self):
        """Another legal entity, a foreign platform account, an unbound tenant,
        another engine's token, no scope: refused before any movement."""
        executions = self.journal("SELECT count(*) FROM payment_ops.native_execution")[0][0]
        r = self.pay("POST", "/v1/payment-intents", self.intent_body(1000, legal_entity_id="LE-FORGED"), key("forged"))
        check(r.status == 403 and r.body["code"] == "PAYMENT_CONTEXT_MISMATCH", f"forged LE: {r!r}")
        r = self.pay("POST", "/v1/payment-intents", self.intent_body(1000, platform_account_id="pacct_stranger"),
                     key("forged"))
        check(r.status == 403 and r.body["code"] == "PAYMENT_CONTEXT_MISMATCH", f"foreign account: {r!r}")
        r = self.pay("POST", "/v1/payment-intents", self.intent_body(1000, tenant_id="tn_unboundtenant"), key("forged"))
        check(r.status == 503 and r.body["code"] == "MERCHANT_NOT_ACTIVATED", f"unbound tenant: {r!r}")
        r = self.pay("POST", "/v1/payment-intents", self.intent_body(1000), key("noscope"),
                     token=self.token(scopes=("payment:read",)))
        check(r.status == 403, f"missing scope: {r!r}")
        r = self.pay("POST", "/v1/payment-intents", self.intent_body(1000), key("noauth"), token="not-a-token")
        check(r.status == 401, f"no valid token: {r!r}")
        r = self.pay("POST", "/v1/payment-intents", self.intent_body(1000), key("wrongaud"),
                     token=self.token(audience="baobab-control-plane"))
        check(r.status == 401, f"token for another audience: {r!r}")
        check(self.journal("SELECT count(*) FROM payment_ops.native_execution")[0][0] == executions,
              "a forged request reached the journal")
        return {}

    def s12_crash_window(self):
        """The provider applies a capture; Payments dies before recording the
        outcome; after restart the retry reconciles from the provider and never
        captures twice."""
        _, confirmed = self.authorise(1000)
        payment = confirmed["payment_id"]
        captures = self.processor_calls("/capture")
        self.fault("hold_response", "/capture")
        idem = key("crash")
        import threading
        worker = threading.Thread(target=lambda: self.capture(payment, None, idem), daemon=True)
        worker.start()
        for _ in range(120):
            if call("GET", PROCESSOR + "/_held").body["held"]:
                break
            time.sleep(0.25)
        check(call("GET", PROCESSOR + "/_held").body["held"], "the capture never reached the processor")
        self.kill_payments()
        call("POST", PROCESSOR + "/_release", {})
        m = [x for x in self.movements(payment) if x["kind"] == "CAPTURE"]
        check(len(m) == 1 and m[0]["dispatches"] == 1 and not m[0]["observations"],
              f"the crash left {m} instead of an unobserved dispatch")
        self.start_payments()
        time.sleep(2)
        r = self.capture(payment, None, idem)
        check(r.status == 200 and r.body["captured_amount_minor"] == 1000, f"retry after crash: {r!r}")
        check(self.processor_calls("/capture") == captures + 1, "the retry captured a second time")
        m = [x for x in self.movements(payment) if x["kind"] == "CAPTURE"]
        check(m[0]["outcome"] == "CONFIRMED" and "RECONCILIATION:CONFIRMED" in (m[0]["observations"] or ""),
              f"not reconciled: {m}")
        return {"payment": payment, "capture": m[0]}

    def processor_effects(self, suffix: str) -> int:
        """Calls the processor accepted (an effect), not merely received."""
        return sum(1 for c in self.processor()["calls"]
                   if c["path"].endswith(suffix) and c["method"] == "POST" and c["status"] == 200)

    def s13_lost_response(self):
        """The processor applies the capture and its answer is lost. HyperSwitch
        retries without an idempotency key, the processor refuses the retry and
        HyperSwitch records the payment as failed although the money moved.
        Payments must not believe either answer: the capture stays UNKNOWN, is
        never re-sent, and keeps holding the authorised balance."""
        _, confirmed = self.authorise(1000)
        payment = confirmed["payment_id"]
        effects = self.processor_effects("/capture")
        self.fault("lose_response", "/capture")
        idem = key("lost")
        first = self.capture(payment, None, idem)
        check(first.status == 202 and first.body["code"] == "PAYMENT_OUTCOME_PENDING", f"lost response: {first!r}")
        again = self.capture(payment, None, idem)
        check(again.status == 202, f"retry while HyperSwitch disagrees with the processor: {again!r}")
        check(self.processor_effects("/capture") == effects + 1, "the processor captured other than once")
        m = [x for x in self.movements(payment) if x["kind"] == "CAPTURE"]
        check(len(m) == 1 and m[0]["outcome"] == "NONE" and "REJECTED" not in (m[0]["observations"] or ""),
              f"the journal settled an unknowable outcome: {m}")
        other = self.capture(payment)
        check(other.status == 422 and other.body["code"] == "CAPTURE_EXCEEDS_AUTHORIZED",
              f"an unknown capture must keep holding the balance: {other!r}")
        hs = self.hs.payment(payment)
        self.ev.note("finding", {"label": "HyperSwitch v1.127 Stripe connector retries a capture whose response "
                                          "was lost, without an idempotency key; the processor refuses the retry "
                                          "and HyperSwitch marks the payment failed although it was captured",
                                 "hyperswitch_status": hs.get("status"), "payment": payment})
        return {"payment": payment, "capture": m, "hyperswitch_status": hs.get("status")}

    def s14_dropped_request(self):
        """The request never reaches the processor: the retry proves it absent
        and dispatches once more on a fresh CP decision."""
        _, confirmed = self.authorise(1000)
        payment = confirmed["payment_id"]
        self.fault("drop_request", "/capture")
        idem = key("dropped")
        r = self.capture(payment, None, idem)
        check(r.status in (202, 422, 503), f"dropped request: {r!r}")
        r = self.capture(payment, None, idem)
        check(r.status == 200 and r.body["captured_amount_minor"] == 1000, f"retry: {r!r}")
        m = [x for x in self.movements(payment) if x["kind"] == "CAPTURE"]
        check(m[0]["outcome"] == "CONFIRMED", f"journal {m}")
        return {"payment": payment, "capture": m}

    def s15_payout_and_direct_sql(self):
        """Payout has no route, and the runtime role can neither reserve a payout,
        dispatch without a decision, rewrite history, nor activate a provider."""
        r = self.pay("POST", "/v1/payouts", {"tenant_id": self.ids["tenant"]}, key("payout"))
        check(r.status in (404, 405), f"payout route: {r!r}")
        password = open(os.path.join(self.args.run, "payments", "payments-database-url")).read().split(":")[2].split("@")[0]
        def runtime(statement: str) -> str:
            p = subprocess.run(self.c.base + ["exec", "-T", "-e", f"PGPASSWORD={password}", "paydb", "psql", "-X", "-q",
                                              "-At", "-h", "127.0.0.1", "-U", "payment_runtime", "-d", "baobab_payments",
                                              "-c", "SELECT set_config('baobab.tenant_id','" + self.ids["tenant"] +
                                              "',false), set_config('baobab.responsible_legal_entity_id','" +
                                              seed.LEGAL_ENTITY + "',false);" + statement],
                               capture_output=True, text=True)
            return p.stderr
        refusals = {
            "payout": ("SELECT payment_ops.reserve_native_execution('" + self.ids["tenant"] + "','PAYOUT',"
                       "'gate-d1-payout-000001',repeat('a',64),'payout_d1','" + self.ids["organisation"] + "','"
                       + seed.LEGAL_ENTITY + "',gen_random_uuid(),'x','x/cert','x/act','ZA','ZAR',100)", "payouts are not activated"),
            "dispatch without decision": ("INSERT INTO payment_ops.dispatch_attempt (tenant_id, responsible_legal_entity_id, "
                                          "execution_id, decision_id, activation_id) SELECT tenant_id, responsible_legal_entity_id, "
                                          "execution_id, gen_random_uuid(), gen_random_uuid() FROM payment_ops.native_execution "
                                          "WHERE operation_kind='CAPTURE' LIMIT 1", "no current execution-point CP decision"),
            "rewrite history": ("UPDATE payment_ops.native_execution SET amount_minor = 1", "permission denied"),
            "delete history": ("DELETE FROM payment_ops.provider_observation", "permission denied"),
            "self-activation": ("INSERT INTO payment_ops.provider_activation (tenant_id, responsible_legal_entity_id, "
                                "merchant_profile_reference, market_code, currency_code, operation_kinds, provider_environment, "
                                "provider_certification_reference, merchant_activation_reference, valid_from, valid_until) VALUES "
                                "('x','LE-X','p','ZA','ZAR',ARRAY['CAPTURE'],'SANDBOX','c','a',now(),now()+interval '1 day')",
                                "permission denied"),
        }
        seen = {}
        # Even the owner cannot rewrite history: the journal's own triggers refuse it.
        for label, statement in (("owner rewrite", "UPDATE payment_ops.native_execution SET amount_minor = 1"),
                                 ("owner delete", "DELETE FROM payment_ops.authority_decision")):
            p = subprocess.run(self.c.base + ["exec", "-T", "paydb", "psql", "-X", "-q", "-U", "baobab", "-d",
                                              "baobab_payments", "-c", statement], capture_output=True, text=True)
            seen[label] = p.stderr.strip().splitlines()[0] if p.stderr.strip() else "ACCEPTED"
            check("immutable" in p.stderr, f"{label} was not refused: {seen[label]}")
        for label, (statement, expected) in refusals.items():
            err = runtime(statement)
            seen[label] = err.strip().splitlines()[0] if err.strip() else "ACCEPTED"
            check(expected in err, f"{label} was not refused: {seen[label]}")
        return {"refusals": seen}

    def s16_protected_environment_refuses_native(self):
        """Configuration refuses the native backend in staging and production."""
        out = {}
        for environment in ("staging", "production"):
            p = subprocess.run(self.c.base + ["run", "--rm", "--no-deps", "-e", f"BAOBAB_ENVIRONMENT={environment}",
                                              "payments"], capture_output=True, text=True,
                               env=dict(os.environ, **self.payments_env()))
            out[environment] = p.returncode
            check(p.returncode == 2 and "accepted only in development and integration" in (p.stdout + p.stderr),
                  f"{environment}: exit {p.returncode}")
        return out

    SCENARIOS = ["s01_authorise_capture_refund", "s02_partial_capture_and_refunds",
                 "s03_overcapture_and_double_capture", "s04_void_then_capture", "s05_void_after_capture",
                 "s06_idempotent_replay_and_key_reuse", "s07_decline", "s08_revocation_between_decision_and_dispatch",
                 "s09_cp_unavailable", "s10_provider_activation_withdrawn", "s11_forged_context", "s12_crash_window",
                 "s13_lost_response", "s14_dropped_request", "s15_payout_and_direct_sql",
                 "s16_protected_environment_refuses_native"]

    def run(self) -> int:
        failures = 0
        try:
            self.seed()
        except Exception as e:  # the seed is a failure of the gate, not a skip
            self.ev.results.append({"scenario": "seed", "status": "FAIL", "error": repr(e),
                                    "trace": traceback.format_exc()[-3000:]})
            failures += 1
        only = os.environ.get("GATE_D1_ONLY")
        for name in self.SCENARIOS if not failures else []:
            if only and name not in only.split(","):
                continue
            started = time.time()
            try:
                detail = getattr(self, name)()
                self.ev.results.append({"scenario": name, "status": "PASS", "seconds": round(time.time() - started, 1),
                                        "detail": detail})
            except Exception as e:
                failures += 1
                self.ev.results.append({"scenario": name, "status": "FAIL", "error": str(e)[:2000],
                                        "trace": traceback.format_exc()[-3000:]})
            print(f"[gate-d1] {name}: {self.ev.results[-1]['status']}", flush=True)
        self.write(failures)
        return failures

    def write(self, failures: int) -> None:
        out = self.args.out
        bundle = {"notes": self.ev.notes, "results": self.ev.results}
        for name, fetch in (("processor-evidence", self.processor),
                            ("issuer-log", lambda: call("GET", ISSUER + "/_log").body)):
            try:
                bundle[name] = fetch()
            except Exception as e:  # evidence collection must not hide results
                bundle[name] = {"error": repr(e)}
        try:
            bundle["payments-journal"] = self.journal("""
                SELECT e.operation_kind, e.operation_class, e.canonical_payment_reference, e.parent_reference,
                       e.amount_minor, e.instruction_fingerprint, coalesce(payment_ops.execution_outcome(e.execution_id),'')
                  FROM payment_ops.native_execution e ORDER BY e.recorded_at""")
            bundle["cp-decisions"] = self.c.sql("cpdb", "controlplane", "controlplane",
                "SELECT operation_class, operation_reference, attempt, outcome, instruction_fingerprint "
                "FROM registry.payments_money_movement_decision ORDER BY recorded_at")
        except Exception as e:
            bundle["journal-error"] = repr(e)
        json.dump(bundle, open(os.path.join(out, "gate-d1-evidence.json"), "w"), indent=1, default=str)
        lines = ["P0-04B Gate D1 - Payments money movement (ADR-SHARED-035)", ""]
        for r in self.ev.results:
            lines.append(f"{r['status']:4}  {r['scenario']}" + (f"  -- {r['error'][:300]}" if r["status"] == "FAIL" else ""))
        synthetic = [n["label"] for n in self.ev.notes if n["kind"].startswith("synthetic")]
        lines += ["", "Synthetic seeds (no admin API, recorded as evidence):"] + [f"  - {s}" for s in synthetic]
        lines += ["", f"Gate D1 failures: {failures}"]
        open(os.path.join(out, "gate-d1-report.txt"), "w").write("\n".join(lines) + "\n")
        print("\n".join(lines))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--compose", required=True)
    sys.exit(1 if GateD1(parser.parse_args()).run() else 0)


if __name__ == "__main__":
    main()
