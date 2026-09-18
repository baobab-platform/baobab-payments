<!-- Target path: nabhold/engine-template/contracts/README.md (becomes <new-repo>/contracts/README.md in any repo created from this template). -->

# Contracts

This directory is a placeholder for how this engine consumes or publishes
`nabhold/shared` contracts (event/AsyncAPI schemas, API contracts, the
Development Environment Contract this repo already declares under
`.nabhold/`) — not a contract definition of its own. Canonical contract
schemas live in `nabhold/shared`, pinned by tag (e.g. `nabhold/shared@v1`),
not copied or forked into this repo.

Fill this in once this engine actually consumes or publishes a contract:

- Which `nabhold/shared` contract(s), at which pinned version.
- Whether this engine is a producer, a consumer, or both, for each.
- Where in this repo's own code that contract is enforced (generated types,
  schema validation, etc.).

Delete this file (or leave it empty with a one-line "none yet") if this
engine genuinely doesn't touch any shared contract beyond the Development
Environment Contract — don't leave placeholder content that looks real but
isn't.
