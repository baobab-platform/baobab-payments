# LA-05E / PEO-02T — candidate negative-only Payments execution veto

**Status: candidate-only contract seam, no runtime activation.** Payments is
currently an in-memory simulated sandbox; it is not a HyperSwitch/PSP
provider, durable payment ledger, live merchant or production settlement
service. Its staging/production configuration prohibition remains unchanged.

`src/documentary_restriction.rs` exports an explicitly opt-in future PSP
execution guard. It requires a current, workload-owned CP runtime Context,
the separately trusted operating Organisation ID, exact market, capability
and payment operation reference. It accepts only CP's canonical
`ADR-BCP-026/PEO-02T:negative-only` response when `blocked=false`,
`authority_granted=false`, and the decision's UTC lease is strictly
positive, unexpired and at most two seconds. It never infers merchant KYC,
regulated PSP permissions or settlement authority.

The opt-in callback-based PEP checks **both** CP legal actor and current
documentary veto before PSP readiness, then checks them **again** afterwards
at the consequential point of use. Any missing CP response, stale/mismatched
scope, newly imposed hold, changed legal mandate or unready PSP denies.
The API transports no payment secrets and never trusts browser-supplied
entity, provider or legal actor identities.

This increment intentionally does **not** wire existing sandbox
`/v1/payments/{id}/capture`, refund or intent-confirmation handlers to a
false positive readiness source. Before live PSP integration:

- Implement and register genuine IAM-authenticated CP client adapters and
  workload scopes for exact Payments audience and Context ownership;
- Bind the governed PEP to every irreversible provider execution path
  (capture, refund, void and relevant authorization), including retry/replay;
- Supply independent merchant/legal/regulatory/provider readiness,
  durable idempotency ledger, real HyperSwitch integration and proof of
  fail-closed negative/revocation races;
- Complete protected staging and operational LA-07/PEO-07 certification.

The synthetic unit test callback is **not** evidence that a live payment,
merchant account, CIPC incorporation or PSP legal permission exists.
