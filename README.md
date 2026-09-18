<!-- Target path: nabhold/engine-template/README.md (becomes <new-repo>/README.md in any repo created from this template). -->

# <engine-repo-name>

<!--
  TODO before this repo's first real PR merges — then delete this comment block:
  1. Replace the title above with the real repo name (e.g. `baobab-iam`), matching
     the naming convention: short, hyphenated, no `-engine`/`-control-plane` suffix
     (see nabhold/baobab-cp, nabhold/baobab-trade, nabhold/baobab-erp,
     nabhold/baobab-pulse, nabhold/baobab-cms for precedent).
  2. Replace ADR-000N below with the real ADR number recording this engine's
     addition to the ecosystem. File it in nabhold/shared/docs/adr/, continuing
     the existing sequence (see that repo's docs/adr/ for the next free number).
  3. Fill in the "Role", "Ownership", and "Contract dependencies" sections below
     with what's actually true for this engine — do not leave the placeholder
     prose in place.
  4. See TEMPLATE-USAGE.md in this repo's root for the full activation checklist
     (CODEOWNERS, devcontainer, Foundation gates, branch protection) — do that
     before writing application code, then delete that file too.
-->

> **Status:** scaffolded, not yet built — see ADR-000N.

## Role

One paragraph: what this engine owns, in the ecosystem's own vocabulary — and,
just as important, what it explicitly does *not* own (business logic that
belongs to another engine, contracts that belong to `nabhold/shared`,
infrastructure that belongs to `nabhold/infrastructure`). Model this on the
"Role" section of an existing repo's README rather than writing it from
scratch — see `nabhold/infrastructure`'s README for the shape.

## Ownership

This repository will contain:

- TODO

It must not contain:

- TODO

## Contract dependencies

Note which `nabhold/shared` contracts this engine consumes or publishes
(event schemas, API contracts, the Development Environment Contract), and at
what pinned version/tag — e.g. `nabhold/shared@v1`. Do not commit to a
contract here until it's actually confirmed; an empty scaffold doesn't need
one yet.

## Local development

This repository uses the shared `baobab-dev` devcontainer image. See
`.nabhold/environment.yaml` for the declared profile and required
capabilities, and `.devcontainer/devcontainer.json` for the pinned image tag.

(Both of those are still `.example` files until this repo's language stack
and `baobab-dev` profile are decided — see `TEMPLATE-USAGE.md`.)

## Foundation status

Foundation 0 (this scaffold: README, CODEOWNERS, branch protection) is
complete. Foundation 1 (application code, real devcontainer/environment
declaration, Foundation CI gates) has not started.
