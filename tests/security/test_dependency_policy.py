"""BA-SEC-DEP-001: source-level manifest and security-maintenance negatives.

Actual GitHub services, native vulnerability findings and protected runtime
approval MUST be independently verified; no mock-positive evidence here.
"""
from __future__ import annotations

from copy import deepcopy
from datetime import date
from pathlib import Path
import json
import re
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]
REPO = "baobab-payments"
EXPECTED = {("github-actions", "/"), ("cargo", "/"), ("docker", "/"), ("docker-compose", "/.devcontainer"), ("docker-compose", "/acceptance/gate-d1")}
COMPOSE = ("compose.yaml", "compose.yml", "docker-compose.yaml", "docker-compose.yml")


def violations(data: object) -> list[str]:
    if not isinstance(data, dict) or data.get("version") != 2:
        return ["version"]
    entries = data.get("updates")
    if not isinstance(entries, list) or not entries:
        return ["updates"]
    errors: list[str] = []
    seen = set()
    for entry in entries:
        if not isinstance(entry, dict):
            errors.append("invalid entry")
            continue
        eco, directory = entry.get("package-ecosystem"), entry.get("directory")
        if not isinstance(eco, str) or not isinstance(directory, str):
            errors.append("missing ecosystem or directory")
            continue
        key = eco, directory
        if key in seen:
            errors.append("duplicate entry")
        seen.add(key)
        if not directory.startswith("/") or ".." in directory.split("/"):
            errors.append("unsafe directory")
            continue
        path = ROOT / directory.lstrip("/")
        if not path.is_dir():
            errors.append("missing directory")
            continue
        if eco == "github-actions" and not (ROOT / ".github/workflows").is_dir():
            errors.append("missing GitHub Actions")
        if eco == "uv" and not ((path / "pyproject.toml").is_file() and (path / "uv.lock").is_file()):
            errors.append("uv lock absent")
        if eco == "pip" and not (
            (path / "pyproject.toml").is_file()
            or any(path.glob("requirements*.txt"))
        ):
            errors.append("pip manifest absent")
        if eco == "maven" and not (path / "pom.xml").is_file():
            errors.append("Maven pom absent")
        if eco == "cargo" and not (path / "Cargo.toml").is_file():
            errors.append("Cargo manifest absent")
        if eco == "npm" and not (path / "package.json").is_file():
            errors.append("npm manifest absent")
        if eco == "docker" and not (path / "Dockerfile").is_file():
            errors.append("Dockerfile absent")
        if eco == "docker-compose" and not any((path / f).is_file() for f in COMPOSE):
            errors.append("Compose manifest absent")
        schedule = entry.get("schedule")
        if not isinstance(schedule, dict) or (
            schedule.get("interval") != "weekly"
            or schedule.get("day") != "monday"
            or schedule.get("timezone") != "Africa/Johannesburg"
            or not schedule.get("time")
        ):
            errors.append("uncontrolled schedule")
        if "target-branch" in entry:
            errors.append("branch override")
        limit = entry.get("open-pull-requests-limit")
        if type(limit) is not int or limit <= 0:
            errors.append("disabled PR limit")
        if not isinstance(entry.get("labels"), list) or not entry["labels"]:
            errors.append("missing labels")
        msg = entry.get("commit-message")
        if not isinstance(msg, dict) or not msg.get("prefix"):
            errors.append("missing commit prefix")
        for group in (entry.get("groups") or {}).values():
            if not isinstance(group, dict) or group.get("applies-to") != "version-updates":
                errors.append("mixed security/version group")
            elif not (
                isinstance(group.get("patterns"), list)
                and group["patterns"]
                and isinstance(group.get("update-types"), list)
                and group["update-types"]
                and set(group["update-types"]).issubset({"minor", "patch"})
            ):
                errors.append("major or wildcard routine grouping")
        for ignored in entry.get("ignore", []):
            if not isinstance(ignored, dict) or not ignored.get("update-types"):
                errors.append("unconditional ignore")
    if seen != EXPECTED:
        errors.append("manifest inventory mismatch")
    return errors


class DependencyPolicyTests(unittest.TestCase):
    def setUp(self):
        self.source = yaml.safe_load((ROOT / ".github/dependabot.yml").read_text())

    def refuse(self, violation: str, mutate):
        candidate = deepcopy(self.source)
        mutate(candidate)
        self.assertIn(violation, violations(candidate))

    def test_exact_real_manifest_inventory(self):
        self.assertEqual([], violations(self.source))
        self.assertEqual(len(EXPECTED), len(self.source["updates"]))

    def test_missing_dependency_surface_refused(self):
        self.refuse("manifest inventory mismatch",
                    lambda d: d["updates"].pop())

    def test_undeclared_dependency_surface_refused(self):
        self.refuse("manifest inventory mismatch",
                    lambda d: d["updates"].append({
                        "package-ecosystem": "docker", "directory": "/invented"
                    }))

    def test_duplicate_manifest_refused(self):
        self.refuse("duplicate entry",
                    lambda d: d["updates"].append(deepcopy(d["updates"][0])))

    def test_no_unsupported_target_branch(self):
        self.refuse("branch override",
                    lambda d: d["updates"][0].update({"target-branch": "main"}))

    def test_unsafe_path_refused(self):
        self.refuse("unsafe directory",
                    lambda d: d["updates"][0].update({"directory": "/../other"}))

    def test_group_must_exclude_security_updates(self):
        self.refuse("mixed security/version group",
                    lambda d: d["updates"][0].update({
                        "groups": {"bad": {"patterns": ["*"]}}
                    }))

    def test_group_cannot_hide_major_upgrades(self):
        self.refuse("major or wildcard routine grouping",
                    lambda d: d["updates"][0].update({
                        "groups": {"bad": {
                            "patterns": ["*"], "applies-to": "version-updates",
                            "update-types": ["major", "minor", "patch"],
                        }}
                    }))

    def test_blanket_ignore_refused(self):
        self.refuse("unconditional ignore",
                    lambda d: d["updates"][0].update({
                        "ignore": [{"dependency-name": "*"}]
                    }))

    def test_disabled_pr_limit_refused(self):
        self.refuse("disabled PR limit",
                    lambda d: d["updates"][0].update({
                        "open-pull-requests-limit": 0,
                    }))

    def test_missing_security_timezone_refused(self):
        self.refuse("uncontrolled schedule",
                    lambda d: d["updates"][0]["schedule"].pop("timezone"))

    def test_missing_labels_refused(self):
        self.refuse("missing labels",
                    lambda d: d["updates"][0].update({"labels": []}))

    def test_accepted_sha_pinned_shared_validator_inputs(self):
        content = (ROOT / ".github/workflows/enforce-dependabot-config.yml").read_text()
        self.assertIn(
            "enforce-dependabot-config.yml@"
            "31de2bc3dcd56128c019c640dc7e12c10ae9ca69",
            content,
        )
        r = re.search(r'required-ecosystems:s*"([^"]+)"', content)
        self.assertIsNotNone(r)
        self.assertEqual({eco for eco, _ in EXPECTED}, set(r.group(1).split(",")))
        self.assertNotIn('policy-level: "strict"', content)

    def test_policy_review_and_security_codeowners(self):
        content = (ROOT / ".github/CODEOWNERS").read_text()
        self.assertIn(
            "/docs/security/dependency-maintenance.md "
            "@baobab-platform/platform-engineering @baobab-platform/security",
            content,
        )
        self.assertIn("@baobab-platform/security", content)
        doc = (ROOT / "docs/security/dependency-maintenance.md").read_text()
        for key in (
            "BA-SEC-DEP-001", "SOURCE_COMMITTED", "CI_VERIFIED",
            "SECURITY_ALERT_PATH_VERIFIED", "BLOCKED", "CONSUMER_QUALIFIED",
        ):
            self.assertIn(key, doc)

    def test_native_scan_is_never_protected_deployment(self):
        path = ROOT / ".github/workflows/dependency-security.yml"
        if not path.is_file():
            self.assertEqual(REPO, "baobab-scf")
            return
        text = path.read_text()
        self.assertIn("cron:", text)
        self.assertNotIn("id-token: write", text)
        self.assertNotIn("terraform apply", text)
        self.assertNotIn("continue-on-error: true", text)

    def test_legacy_template_does_not_become_runtime(self):
        if REPO == "baobab-scf":
            self.assertFalse((ROOT / "pyproject.toml").exists())
            self.assertFalse((ROOT / "package.json").exists())
            self.assertFalse((ROOT / "Dockerfile").exists())
            self.assertEqual({("github-actions", "/")}, EXPECTED)
        elif REPO == "baobab-tms":
            package = json.loads((ROOT / "package.json").read_text())
            self.assertFalse(package.get("dependencies"))
            self.assertFalse(package.get("devDependencies"))
            self.assertFalse((ROOT / "package-lock.json").exists())
            self.assertEqual("UNQUALIFIED", json.loads(
                (ROOT / "config/hybrid/upstreams.lock.json").read_text()
            )["releaseStatus"])

    def test_cms_waiver_not_infinite_or_treated_as_a_pass(self):
        if REPO != "baobab-cms":
            return
        data = yaml.safe_load((ROOT / ".baobab/repository.yaml").read_text())
        waiver = data["exceptions"]["dependency-audit"]
        until = waiver["expires"]
        if isinstance(until, str):
            until = date.fromisoformat(until)
        self.assertGreaterEqual(
            until, date.today(),
            "CMS audit exemption expired: BLOCKED, requires independently approved remediation",
        )
        ignore = (ROOT / ".trivyignore").read_text()
        self.assertIn("CVE-2026-93687 exp:2026-12-31", ignore)
        self.assertNotRegex(ignore, r"(?m)^CVE-d{4}-d+s*$")

    def test_rust_cargo_lock_and_document_hybrid_custody(self):
        if REPO == "baobab-payments":
            self.assertTrue((ROOT / "Cargo.lock").is_file())
            self.assertTrue((ROOT / "acceptance/gate-d1/compose.yaml").is_file())
        if REPO == "baobab-trade-docs":
            self.assertTrue((ROOT / "rust-kernel/Cargo.lock").is_file())
            text = (ROOT / "runtime/requirements.lock").read_text()
            self.assertIn("--hash=sha256:", text)
            self.assertTrue((ROOT / "runtime/requirements.in").is_file())

    def test_python_lock_state_and_known_regulations_gap(self):
        if REPO == "baobab-pulse":
            self.assertTrue((ROOT / "uv.lock").is_file())
            text = (ROOT / ".github/workflows/security-python.yml").read_text()
            self.assertIn('cron: "0 5 * * *"', text)
        if REPO == "baobab-regulations":
            self.assertTrue((ROOT / "pyproject.toml").is_file())
            self.assertFalse((ROOT / "uv.lock").exists())
            self.assertIn("UNLOCKED_PYTHON", (
                ROOT / "docs/security/dependency-maintenance.md"
            ).read_text())

    def test_scanner_controls_match_native_manifest(self):
        if REPO == "baobab-scf":
            return
        if REPO == "baobab-pulse":
            text = (ROOT / ".github/workflows/security-python.yml").read_text()
            self.assertIn("security-python.yml@31de2bc3dcd56128c019c640dc7e12c10ae9ca69", text)
            return
        text = (ROOT / ".github/workflows/dependency-security.yml").read_text()
        if REPO == "baobab-cms":
            self.assertIn("npm audit --audit-level=high", text)
        if REPO == "baobab-payments" or REPO == "baobab-trade-docs":
            self.assertIn("cargo audit", text)
        if REPO == "baobab-subscriptions":
            self.assertIn("dependency-check-maven:12.1.0:check", text)
        if REPO == "baobab-regulations":
            self.assertIn("pip-audit --local --strict", text)
        if REPO == "baobab-tms":
            self.assertIn("untracked third-party dependencies", text)


if __name__ == "__main__":
    unittest.main()
