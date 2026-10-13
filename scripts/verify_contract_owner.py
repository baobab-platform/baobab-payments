#!/usr/bin/env python3
"""Compare synthetic golden vectors with immutable Shared owner schemas.

The independent Python engine loads original Git blobs, not Payments' embedded
copies. Optionally compare the SAME vectors with a standalone release binary.
Dependencies are maintainer/CI-only: PyYAML, jsonschema[format-nongpl].
"""
import argparse
import json
import subprocess
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker
from referencing import Registry, Resource

from sync_contracts import ENTRIES, ROOT, generate


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--shared-checkout", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    args = parser.parse_args()
    files, _ = generate(args.shared_checkout)
    resources = []
    for path, raw in files.items():
        if path.endswith(".schema.json"):
            schema = json.loads(raw)
            Draft202012Validator.check_schema(schema)
            resources.append((schema["$id"], Resource.from_contents(schema)))
    registry = Registry().with_resources(resources)
    format_checker = FormatChecker()
    for name in ("uuid", "date-time"):
        if name not in format_checker.checkers:
            raise ValueError(f"Required format checker unavailable: {name}")
    validators = {
        name: Draft202012Validator(
            {"$ref": "https://contracts.baobab-platform.com/" + path + "#/$defs/" + definition},
            registry=registry, format_checker=format_checker,
        ) for name, (path, definition) in ENTRIES.items()
    }
    vectors = ROOT / "tests/fixtures/authority-contract-cases.json"
    cases = json.loads(vectors.read_bytes())
    if {c["contract"] for c in cases} != set(ENTRIES):
        raise ValueError("Every consumed authority entry needs golden coverage")
    if len({c["name"] for c in cases}) != len(cases):
        raise ValueError("Duplicate golden names")
    for case in cases:
        if validators[case["contract"]].is_valid(case["instance"]) != case["valid"]:
            raise ValueError(f"Golden differs from immutable owner: {case['name']}")
    if args.binary:
        binary = str(args.binary.resolve())
        # Run away from the checkout: the executable must own its entire closure.
        subprocess.run([binary, "contract-self-test"], cwd="/tmp", check=True)
        subprocess.run([binary, "contract-conformance", str(vectors)], cwd="/tmp", check=True)
    print(f"Pinned owner agrees with {len(cases)} synthetic wire vectors")


if __name__ == "__main__":
    main()
