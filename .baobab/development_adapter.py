#!/usr/bin/env python3
"""Host-side DEV-15 Payments probe adapter (not inside the Rust service image).

Only the unprivileged native health endpoint may pass at this stage. Actual
money-movement and authority cases require real IAM tokens, CP governance and
Rust HTTP effects. Nothing here mints identity or fabricates a PSP receipt.
"""
from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request

PHASES = ("readiness", "migrate", "seed", "exercise", "cleanup")
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
                and payload.get("payment_provider") == {"kind": "sandbox", "simulated": True}
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
    parser.add_argument("phase", choices=PHASES)
    args = parser.parse_args(argv)
    status, receipt = run(args.phase)
    print(json.dumps(receipt, sort_keys=True))
    return status


if __name__ == "__main__":
    sys.exit(main())
