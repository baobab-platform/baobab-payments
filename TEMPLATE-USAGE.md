<!-- Target path: nabhold/engine-template/TEMPLATE-USAGE.md (becomes <new-repo>/TEMPLATE-USAGE.md in any repo created from this template — delete it there once followed, per its own last checklist item). -->

# Using this template

Delete this file once you've worked through the checklist below — it isn't
part of any real engine repo, only the template.

## 1. Create the repo

```bash
gh repo create nabhold/<engine-repo-name> --template nabhold/engine-template --private
```

(`--private` matches the ecosystem's default — flip it if this repo is
meant to be public from day one.)

## 2. Scaffold-only step (do this immediately, before any application code)

This is Foundation 0 — README + CODEOWNERS + branch protection, plus the
toolchain-agnostic org-standard CI checks that don't need application code
to run. Nothing below this point requires application code to exist yet.

- [ ] Edit `README.md`: real repo name, real ADR number, real Role/Ownership/
      Contract-dependencies content. File the ADR in
      `nabhold/shared/docs/adr/` first if it doesn't exist yet (check
      `docs/adr/` there for the next free number — `nabhold/shared`'s own
      reorg plan expected ADR-0002 for one purpose but a different ADR
      already claimed that number in practice, so don't assume a number
      from an older doc is still free).
- [ ] Edit `.github/CODEOWNERS`: this template ships it already pointed at
      the real, verified-live org convention
      (`@nabhold/platform-engineering` / `@nabhold/security` — confirmed
      against `nabhold/shared`, `nabhold/baobab-dev`, and
      `nabhold/baobab-cp`'s actual CODEOWNERS files on 2026-09-03, not the
      unadopted `@nabhold/<repo>-maintainers` proposal some older project
      docs describe). Re-check it still matches a live repo's CODEOWNERS
      before merging if this template is more than a few weeks old, and
      add repo-specific path rules if this engine needs them.
- [ ] `greetings.yml`, `enforce-action-pinning.yml`, `dependabot.yml` +
      `enforce-dependabot-config.yml`, and `security-secrets-scan.yml` are
      already live (not `.example`) — they're toolchain-agnostic and need
      no application code to run cleanly on an empty scaffold. Just adjust
      `dependabot.yml`'s `timezone`/`target-branch` if this repo's team or
      default branch differs from the org default already filled in.
- [ ] Apply branch protection / a repository ruleset on `main`. Minimum
      shape, per `nabhold/shared/docs/governance/foundation-4-branch-protection.md`:
      require a pull request before merging; require at least one approval
      and a CODEOWNER review; dismiss stale approvals on new commits;
      require conversation resolution; require branches up to date before
      merging; block force pushes and branch deletion; apply to
      administrators unless an emergency bypass is explicitly audited. Add
      `Enforce Action Pinning`, `Enforce Dependabot Config`, and
      `Security — Secret Scanning` as required status checks now (they're
      already live per the item above) — `Foundation Repository Gates`
      gets added as a required check in step 3 instead, once it can
      actually pass.

At this point the repo can sit scaffolded, exactly like `baobab-trade`,
`baobab-erp`, and `baobab-pulse` did before their own build-out — there's no
requirement to do the rest immediately.

## 3. Build-start step (do this when application code actually starts)

- [ ] Decide this engine's language stack and which `baobab-dev` profile
      fits it (`full`, `frontend`, `frontend-e2e`, or `infra` — see
      `nabhold/shared`'s `contracts/development-environment/schema.yaml` for
      what each profile actually includes; pick the narrowest one that
      covers this repo's real needs, not `full` by default).
- [ ] Copy `.nabhold/environment.yaml.example` to `.nabhold/environment.yaml`
      and fill in every placeholder. Confirm the current `baobab-dev`
      release tag (check its `CHANGELOG.md` / GHCR package page — don't
      assume any version number written elsewhere is still current) and use
      the same version in both `minimum_version` here and the image tag in
      `.devcontainer/devcontainer.json`.
- [ ] Fill in `.devcontainer/devcontainer.json`'s placeholders the same way,
      then delete `.nabhold/environment.yaml.example` and
      `.devcontainer/devcontainer.json`'s inline TODO comments.
- [ ] Rename `.github/workflows/foundation.yml.example` to
      `.github/workflows/foundation.yml` (drop the `.example` suffix — do
      not create a second, differently-named file). Confirm the pinned
      commit SHA it calls (`nabhold/shared/.github/workflows/
      foundation-repository-gates.yml@<sha>`) is still current — diff it
      against that file's current `main` before relying on an old pin, and
      check whether a new `nabhold/shared` tag now covers it (as of
      2026-09-03 it didn't yet — main-only).
- [ ] Set `dockerfile: ""` in that workflow's `with:` block if this engine
      doesn't build a container image yet (the reusable workflow's default
      assumes a Dockerfile exists at the repo root); update it once one
      does.
- [ ] Once Foundation CI is green on a real PR, add `Foundation Repository
      Gates` as a required status check on `main`'s branch protection from
      step 2.
- [ ] Adopt whichever of the conditional `.example` workflows actually
      apply to this engine, renaming each to drop `.example` and resolving
      its own TODOs — don't adopt one that doesn't apply, per each file's
      own header:
      - `security-codeql.yml.example` — only if this repo has a
        CodeQL-supported language.
      - `security-python.yml.example` — only if this repo is a uv-managed
        Python project.
      - `release.yml.example` — only if this repo's releases are notes-only
        (not a fit for a bespoke container/package publish pipeline like
        `baobab-dev`'s own `publish.yml` — write a repo-specific one
        instead if so).
      - `pages.yml.example` — only if this repo publishes docs via the
        org's Zensical/uv toolchain.
- [ ] Fill in `contracts/README.md` with the actual contract(s) this engine
      consumes or publishes, or delete it if none apply yet.
- [ ] Delete this file.
