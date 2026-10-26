"""Payments DEV-15 host-only adapter never manufactures financial acceptance."""
from __future__ import annotations

import importlib.util
from pathlib import Path
from unittest.mock import patch
import unittest

ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / ".baobab/development_adapter.py"
SPEC = importlib.util.spec_from_file_location("payments_dev15", PATH)
assert SPEC and SPEC.loader
ADAPTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ADAPTER)


class PaymentsDev15AdapterTests(unittest.TestCase):
    def test_only_actual_native_health_may_pass(self):
        for phase in ("migrate", "seed", "exercise", "cleanup"):
            with self.subTest(phase=phase):
                status, row = ADAPTER.run(phase)
                self.assertEqual(status, 2)
                self.assertEqual(row["result"], "BLOCKED")
                self.assertFalse(row["native_money_effect_observed"])
                self.assertFalse(row["eng_onb_runtime_qualified"])

    def test_unknown_and_external_urls_refused(self):
        self.assertFalse(ADAPTER.probe("http://localhost:8080/health/ready"))
        self.assertFalse(ADAPTER.probe("http://example.org/health/ready"))
        with self.assertRaisesRegex(ValueError, "UNKNOWN_OWNER_PHASE"):
            ADAPTER.run("approve")

    def test_health_reply_rejects_protected_or_nonsandbox_provider(self):
        import io
        import json

        class Reply:
            status = 200

            def __init__(self, body):
                self.content = io.BytesIO(json.dumps(body).encode())

            def read(self, size):
                return self.content.read(size)

            def __enter__(self):
                return self

            def __exit__(self, *_):
                return False

        good = {
            "status": "READY",
            "checks": {"store": "UP", "payment_provider": "UP"},
            "payment_provider": {"kind": "SANDBOX", "simulated": True},
            "real_payments": "NOT_CONFIGURED",
            "store": "in-memory",
            "environment": "development",
        }
        for mutations, expected in (
            ({}, True),
            ({"environment": "production"}, False),
            ({"payment_provider": {"kind": "hyperswitch", "simulated": False}}, False),
            ({"real_payments": "CONFIGURED"}, False),
            ({"status": "NOT_READY"}, False),
        ):
            with self.subTest(mutations=mutations):
                reply = Reply({**good, **mutations})
                with patch.object(ADAPTER.urllib.request, "urlopen", return_value=reply):
                    self.assertEqual(ADAPTER.probe(ADAPTER.LOOPBACK), expected)

    def test_outage_retry_keeps_body_stable_across_fresh_cp_contexts(self):
        import copy
        import json
        import types
        import uuid
        driver=types.SimpleNamespace(context={
            "schema":"baobab.engine-adapter-context.v1","run":{"id":"a"*32},
            "tenant":{"id":"tn_own","legal_entity_id":"DEV-SYNTHETIC","market_id":"mkt_za","country":"ZA"},
            "witness":{"caller_token":"synthetic-component-token","context":{"context_id":str(uuid.uuid4())}},
            "fault_phase":"cp"},last_headers={})
        stored={};bodies=[];unavailable=True
        def native(path,body,token,context,key,method):
            driver.last_headers={}
            if unavailable:return 503,{"code":"SANDBOX_AUTHORITY_UNAVAILABLE"}
            if method=="GET":return 200,copy.deepcopy(next(iter(stored.values()))[1])
            bodies.append(copy.deepcopy(body));fingerprint=json.dumps(body,sort_keys=True)
            if key in stored:
                prior,value=stored[key]
                if fingerprint!=prior:return 409,{"code":"IDEMPOTENCY_KEY_REUSED"}
                driver.last_headers={"idempotent-replayed":"true"};return 201,copy.deepcopy(value)
            value={"payment_intent_id":"payint"+"a"*32,"provider":{"kind":"SANDBOX","simulated":True}}
            stored[key]=(fingerprint,value);return 201,copy.deepcopy(value)
        driver.native=native
        with patch.dict("sys.modules",{"engine_sdk":types.SimpleNamespace(Driver=lambda:driver)}):
            self.assertEqual(ADAPTER.connected("unavailable"),{"observed":True})
            unavailable=False
            for _ in range(2):
                driver.context["witness"]["context"]["context_id"]=str(uuid.uuid4())
                self.assertEqual(ADAPTER.connected("restored"),{"observed":True})
        self.assertEqual(len(stored),1)
        self.assertTrue(all(body==bodies[0] for body in bodies))

    def test_real_health_observation_never_means_payments_authority(self):
        with patch.object(ADAPTER, "probe", return_value=True):
            status, row = ADAPTER.run("readiness")
        self.assertEqual(status, 0)
        self.assertEqual(row["evidence_class"], "REAL_HEALTH_ONLY")
        self.assertFalse(row["native_money_effect_observed"])
        self.assertFalse(row["eng_onb_runtime_qualified"])
        self.assertFalse(row["production_authorized"])


if __name__ == "__main__":
    unittest.main()
