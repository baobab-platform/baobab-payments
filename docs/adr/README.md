<!-- Target path: nabhold/engine-template/docs/adr/README.md (becomes <new-repo>/docs/adr/README.md in any repo created from this template). -->

# ADRs

Ecosystem-level decisions (new engines, cross-repo contract changes, org-wide
tooling like this template) are recorded centrally as numbered ADRs in
`nabhold/shared/docs/adr/`, continuing that repo's existing sequence — not
here. This engine's own scaffolding decision (its addition to the ecosystem)
should have an entry there; see `README.md`'s "ADR-000N" reference.

Whether *this repo* should also keep repo-local ADRs for engine-internal
decisions (as opposed to ecosystem-level ones) is not yet a settled
convention — `nabhold/shared`'s governance strategy flags "is
`nabhold/shared/docs/adr/` meant to be the single ADR log for the whole org,
or just for contract-schema decisions specifically?" as still open. Until
that's resolved, don't assume this folder is the right place for a
repo-local decision log — check with whoever owns that governance doc first.
