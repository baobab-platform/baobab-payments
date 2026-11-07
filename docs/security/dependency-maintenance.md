# BA-SEC-DEP-001 — baobab-payments dependency and security maintenance audit

**Review date:** 7 November 2026. **Repository:** `baobab-platform/baobab-payments`. **Status:** SOURCE_COMMITTED only.

## Observed manifests and Dependabot coverage

GitHub Actions /; Cargo / (Cargo.toml + Cargo.lock); Docker /; Docker Compose /.devcontainer; Docker Compose /acceptance/gate-d1.

## Actual dependency/security surveillance

Daily RustSec cargo-audit pinned 0.22.2 scans committed Cargo.lock without vulnerability ignore settings. Existing native PostgreSQL journal/reconciliation, IAM/CP legal actor and PSP-negative acceptance remain unchanged.

## Findings, unqualified boundaries and exception custody

acceptance/gate-d1/images contains cp.Dockerfile, payments-local.Dockerfile and python.Dockerfile, not default Dockerfile filenames. Do NOT claim that standard Docker Dependabot automatically maintains these image builders; independent provenance and native image scanning remain outstanding. Existing Payments ENG-ONB and event replay gaps must remain blocked pending native proof. No real PSP sandbox success can be inferred from fixtures.

**Current disposition:** Cargo audit source exists, but actual RustSec index reachability, locked graph severity and provider-native effects must be checked.

## Governance and evidence levels

Governing authority: [approved BA-SEC-DEP-001 v1.0, Shared #429](https://github.com/baobab-platform/shared/pull/429). The proposed stricter Shared validator is NOT released; this branch uses the SHA-pinned accepted Shared v2.3.0 structural validator and an owner-local positive/negative source suite. Weekly Monday Africa/Johannesburg version updates are distinct from advisory-driven security response. Routine groups apply only to minor/patch version-updates; major or security updates require independent review.

Evidence state **SOURCE_COMMITTED** means source reviewed/committed, not executed. **CI_VERIFIED**, **SECURITY_ALERT_PATH_VERIFIED**, **CONSUMER_QUALIFIED**, **STAGING_VERIFIED**, and **PRODUCTION_VERIFIED** are NOT claimed. Findings are **BLOCKED** pending an independently approved remediation or expiring exception. **WAIVED is not PASSED**. Security CODEOWNERS review applies to manifest and exception changes.

## Exit criteria

1. Run the actual final-head policy regression and accepted Shared validator; inspect live GitHub Dependabot job logs, dependency graph, Alerts, automatic security update settings and actual dependency scan results.
2. Repair reported security vulnerabilities promptly using real installed package/image provenance; independent time-bounded evidence and Security approval are required for any unpatched exceptions. Do not permit scanner errors, no-ops or ignored advisories to appear as success.
3. Re-run owner-specific native runtime, IAM/CP authority, supplier/consumer isolation, replay, migration and negative acceptance tests after a major or security update. No synthetic PSP, carrier, rule or issuer success counts as a real integration.
4. Obtain separate staging/production approvals and deployment evidence. No protected environment, credential, provider or release is activated by this policy adoption.

No CI run or monitoring, PR merge or protected business approval has been claimed.
