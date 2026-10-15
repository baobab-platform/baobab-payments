#!/usr/bin/env python3
"""P0-04B Gate D1 production exclusion.

The deterministic test processor and the test OIDC issuer exist only to drive
Gate D1 in CI. This check fails if anything deployable could carry them:

  - only acceptance/gate-d1/ may contain them, mention them, or point
    HyperSwitch's Stripe connector anywhere but api.stripe.com;
  - the release Dockerfile and .dockerignore-controlled build context must not
    copy acceptance/;
  - the test services must refuse to start outside BAOBAB_ENVIRONMENT=integration.
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
ALLOWED = ROOT / "acceptance" / "gate-d1"
SELF = pathlib.Path(__file__).resolve()
MARKERS = [
    re.compile(r"test-processor|processor\.py|issuer\.py|gate-d1-processor"),
    re.compile(r"stripe\.base_url\s*=\s*\"(?!https://api\.stripe\.com/)"),
]
SKIP_DIRS = {".git", "target", "node_modules", ".gate-d1"}
# Documentation and the CI workflow that runs the gate may describe it.
DESCRIBES = {ROOT / ".github" / "workflows" / "payments-gate-d1.yml"}


def files():
    for path in ROOT.rglob("*"):
        if path.is_dir() or any(part in SKIP_DIRS for part in path.relative_to(ROOT).parts):
            continue
        if path == SELF or path in DESCRIBES or ALLOWED in path.parents or path.suffix == ".md":
            continue
        yield path


def main() -> int:
    problems = []
    for path in files():
        try:
            text = path.read_text(errors="ignore")
        except OSError:
            continue
        for marker in MARKERS:
            if marker.search(text):
                problems.append(f"{path.relative_to(ROOT)}: references the Gate D1 test services")
                break
    dockerfile = (ROOT / "Dockerfile").read_text()
    if re.search(r"^\s*(COPY|ADD)\s+(\.|acceptance)", dockerfile, re.M):
        problems.append("Dockerfile copies the whole context or acceptance/ into the release image")
    for name in ("processor/processor.py", "issuer/issuer.py"):
        text = (ALLOWED / name).read_text()
        if 'os.environ.get("BAOBAB_ENVIRONMENT") != "integration"' not in text and \
                'os.environ.get(REFUSE_NON_INTEGRATION) != "integration"' not in text:
            problems.append(f"acceptance/gate-d1/{name} does not refuse non-integration environments")
    for problem in problems:
        print(f"production exclusion: {problem}", file=sys.stderr)
    if not problems:
        print("production exclusion: the Gate D1 test services are confined to acceptance/gate-d1")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
