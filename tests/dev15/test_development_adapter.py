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
