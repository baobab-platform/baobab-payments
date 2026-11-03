# DEV10-R3 Payments Rust stable owner qualification

The installed Rust toolchain must be the upstream **1.99.1** patch release,
not unpinned `stable`. This is a disposable development gate; its exact
compiler version is checked before compilation. Source and Cargo.lock do not
change.

The PostgreSQL **17.11** service runs owner-authored journal migrations and
native API roles. The restricted runtime principal and money-movement tests
are not skipped. A loopback synthetic provider is **never** real HyperSwitch
or PSP certification. DEV10/DEV15 protected release authority remains denied.

The Payments Dev Container now runs the exact PostgreSQL **17.11-bookworm**
service, with a dedicated `postgres17-data` volume. The legacy `postgres-data`
volume is intentionally **not** mounted, upgraded or deleted. Anyone needing
existing PostgreSQL 16 development data must back it up and perform an
explicit logical dump/restore into PostgreSQL 17; changing a Docker image tag
does not upgrade a PostgreSQL physical data directory. Never run
`docker compose down --volumes` as part of the migration.

This workflow exercises the **running devcontainer PostgreSQL server**, reads
`server_version_num` and fails unless the major is exactly 17. The actual
journal migration fixture separately interrogates its running PG17 service;
a PG17-labelled image alone is not treated as acceptance. These checks run
on **every PR**, not only edits to the workflow.

Rust's inherited `ghcr.io/devcontainers/features/rust:1` feature still needs
a separate reproducible owner-toolchain decision after the Rust 1.99.1 probe;
it is not silently declared pinned by this PostgreSQL change. Neither this
development database nor the synthetic payment fixture certifies HyperSwitch,
a real PSP, Codespaces, protected staging or production.
