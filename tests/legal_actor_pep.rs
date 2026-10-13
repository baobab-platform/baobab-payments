//! LA-05E PEP tests at the public API. Pure policy tests do not prove HyperSwitch.
use std::cell::Cell;

use baobab_payments::legal_actor::{
    LegalActorGateError, PaymentLegalOperation, PaymentProviderLegalReadiness,
    execute_with_current_payment_actor,
};
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;
use time::macros::datetime;
use time::{Duration, OffsetDateTime};

struct Merchant {
    ready: bool,
    calls: Cell<usize>,
}
impl PaymentProviderLegalReadiness for Merchant {
    fn require_ready(
        &self,
        _: &PaymentLegalOperation<'_>,
        _: &str,
    ) -> Result<(), LegalActorGateError> {
        self.calls.set(self.calls.get() + 1);
        if self.ready {
            Ok(())
        } else {
            Err(LegalActorGateError::ProviderNotReady)
        }
    }
}

fn merchant(ready: bool) -> Merchant {
    Merchant {
        ready,
        calls: Cell::new(0),
    }
}

fn op() -> PaymentLegalOperation<'static> {
    PaymentLegalOperation {
        context_id: "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6b",
        expected_legal_entity_id: "LE-TEST-BENEFICIARY",
        role: "PAYMENT_BENEFICIARY",
        activity: "B2B_COFFEE_CAPTURE",
        market: "ZA",
        capability: "payments.payment.capture",
        operation_reference: "payment/synthetic-001",
    }
}

fn now() -> OffsetDateTime {
    datetime!(2026-10-09 14:00 UTC)
}

fn cp(outcome: &str, actor: &str, offset: i64) -> Value {
    let t = now() + Duration::seconds(offset);
    json!({
        "context_id": op().context_id,
        "operation_reference": op().operation_reference,
        "provider_permissions_granted": false,
        "legal_actor_resolution": {
            "outcome": outcome,
            "policy_reference": "ADR-BCP-027/LA-04D",
            "mandate_id": "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d",
            "responsible_legal_entity_id": actor,
            "evidence_references": ["test/registry-evidence"],
            "evaluated_at": t.format(&Rfc3339).unwrap(),
            "valid_until": (t + Duration::seconds(20)).format(&Rfc3339).unwrap(),
        }
    })
}

#[test]
fn valid_beneficiary_and_merchant_are_rechecked_on_every_retry() {
    let body = cp("AUTHORIZED", "LE-TEST-BENEFICIARY", 0);
    let m = merchant(true);
    let mut executed = 0;
    for _ in 0..2 {
        let out = execute_with_current_payment_actor(&op(), Ok(&body), &m, now(), || {
            executed += 1;
            "sandbox-only"
        });
        assert_eq!(out, Ok("sandbox-only"));
    }
    assert_eq!((m.calls.get(), executed), (2, 2));
}

#[test]
fn denied_or_stale_cp_fact_never_reaches_provider_or_mutation() {
    for (outcome, actor, offset) in [
        ("REVOKED_OR_EXPIRED", "LE-TEST-BENEFICIARY", 0),
        ("AUTHORIZED", "LE-OTHER", 0),
        ("AUTHORIZED", "LE-TEST-BENEFICIARY", -50),
        ("AUTHORIZED", "LE-TEST-BENEFICIARY", -25),
    ] {
        let body = cp(outcome, actor, offset);
        let m = merchant(true);
        let executed = Cell::new(false);
        let out =
            execute_with_current_payment_actor(&op(), Ok(&body), &m, now(), || executed.set(true));
        assert_eq!(out, Err(LegalActorGateError::NotAuthorised));
        assert!(!executed.get());
        assert_eq!(m.calls.get(), 0);
    }
}

#[test]
fn unavailable_cp_never_reaches_provider_or_mutation() {
    let m = merchant(true);
    let executed = Cell::new(false);
    let out = execute_with_current_payment_actor(
        &op(),
        Err(LegalActorGateError::Unavailable),
        &m,
        now(),
        || executed.set(true),
    );
    assert_eq!(out, Err(LegalActorGateError::Unavailable));
    assert!(!executed.get());
    assert_eq!(m.calls.get(), 0);
}

#[test]
fn current_cp_authorization_is_not_a_merchant_or_payout_grant() {
    let body = cp("AUTHORIZED", "LE-TEST-BENEFICIARY", 0);
    let m = merchant(false);
    let executed = Cell::new(false);
    let out =
        execute_with_current_payment_actor(&op(), Ok(&body), &m, now(), || executed.set(true));
    assert_eq!(out, Err(LegalActorGateError::ProviderNotReady));
    assert!(!executed.get());
    assert_eq!(m.calls.get(), 1);
}

#[test]
fn no_default_company_or_market_inference() {
    let body = cp("AUTHORIZED", "LE-TEST-BENEFICIARY", 0);
    let m = merchant(true);
    for bad in [
        PaymentLegalOperation {
            expected_legal_entity_id: "",
            ..op()
        },
        PaymentLegalOperation {
            market: "za",
            ..op()
        },
    ] {
        let out = execute_with_current_payment_actor(&bad, Ok(&body), &m, now(), || ());
        assert_eq!(out, Err(LegalActorGateError::MissingContext));
    }
    assert_eq!(m.calls.get(), 0);
}

#[test]
fn malformed_closed_responses_never_reach_readiness_or_execution() {
    let good = cp("AUTHORIZED", "LE-TEST-BENEFICIARY", 0);
    let mut candidates = Vec::new();
    for (field, value) in [
        ("extra_authority", json!(true)),
        ("context_id", json!(null)),
        ("provider_permissions_granted", json!(true)),
    ] {
        let mut body = good.clone();
        body[field] = value;
        candidates.push(body);
    }
    for (field, value) in [
        ("provider_override", json!(true)),
        ("policy_reference", json!("x".repeat(501))),
        ("evidence_references", json!(["x".repeat(501)])),
        ("evaluated_at", json!("2026-02-30T00:00:00Z")),
    ] {
        let mut body = good.clone();
        body["legal_actor_resolution"][field] = value;
        candidates.push(body);
    }
    for body in candidates {
        let provider = merchant(true);
        let executed = Cell::new(false);
        assert_eq!(
            execute_with_current_payment_actor(&op(), Ok(&body), &provider, now(), || executed
                .set(true)),
            Err(LegalActorGateError::NotAuthorised)
        );
        assert_eq!(provider.calls.get(), 0);
        assert!(!executed.get());
    }
}

#[test]
fn invalid_operation_is_rejected_before_any_policy_or_provider_use() {
    let long = "x".repeat(257);
    for operation in [
        PaymentLegalOperation {
            context_id: "legacy-opaque-handle",
            ..op()
        },
        PaymentLegalOperation {
            activity: "x",
            ..op()
        },
        PaymentLegalOperation {
            activity: &long,
            ..op()
        },
        PaymentLegalOperation {
            operation_reference: &long,
            ..op()
        },
        PaymentLegalOperation {
            capability: &long,
            ..op()
        },
    ] {
        assert_eq!(
            operation.request(),
            Err(LegalActorGateError::MissingContext)
        );
    }
}

#[test]
fn canonical_negative_assessment_is_valid_wire_and_still_denies_execution() {
    let mut denied = cp("AUTHORIZED", "LE-TEST-BENEFICIARY", 0);
    denied["legal_actor_resolution"] = json!({
        "outcome": "NO_APPLICABLE_MANDATE", "policy_reference": "ADR-BCP-027/LA-04",
        "evaluated_at": now().format(&Rfc3339).unwrap()
    });
    assert!(baobab_payments::contracts::entry_is_valid(
        "legal_response",
        &denied
    ));
    let provider = merchant(true);
    let executed = Cell::new(false);
    assert_eq!(
        execute_with_current_payment_actor(&op(), Ok(&denied), &provider, now(), || executed
            .set(true)),
        Err(LegalActorGateError::NotAuthorised)
    );
    assert_eq!(provider.calls.get(), 0);
    assert!(!executed.get());
}
