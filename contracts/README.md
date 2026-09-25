# Contracts

This engine defines no canonical schema of its own. The canonical contracts live in [`baobab-platform/shared`](https://github.com/baobab-platform/shared) and are pinned by commit.

| Shared contract | Role here |
|---|---|
| `contracts/payments/v1` | **Producer**: the Baobab Payment API (payment intents, payments, refunds) and `payment.*` events. Sandbox results are always marked `simulated`. |

Where each contract is enforced in code is documented with the runtime.
