# DEV10-R3 Payments Rust stable owner qualification

The installed Rust toolchain must be the upstream **1.99.1** patch release,
not unpinned `stable`. This is a disposable development gate; its exact
compiler version is checked before compilation. Source and Cargo.lock do not
change.

The PostgreSQL **17.11** service runs owner-authored journal migrations and
native API roles. The restricted runtime principal and money-movement tests
are not skipped. A loopback synthetic provider is **never** real HyperSwitch
or PSP certification. DEV10/DEV15 protected release authority remains denied.

The existing Payments Dev Container inherits the unversioned
`ghcr.io/devcontainers/features/rust:1` feature and PostgreSQL16. Separate
owner contract migration to pinned Rust and PostgreSQL17 is required after
this probe passes, with backup/recovery and signed image evidence.
