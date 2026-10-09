//! LA-05E candidate PEP tests. Pure policy tests do not prove HyperSwitch.
use std::cell::Cell;
use baobab_payments::legal_actor::{
    LegalActorAssessmentSource, MerchantLegalReadiness,
    PaymentLegalActorOperation, PaymentLegalActorRequest, PaymentLegalActorResponse,
    PaymentLegalActorResolution, execute_governed_payment,
};
use baobab_payments::service::Refusal;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use time::macros::datetime;

struct Cp {
    outcome: &'static str,
    actor: &'static str,
    calls: Cell<usize>,
    offset: i64,
}
impl LegalActorAssessmentSource for Cp {
    fn assess(&self, req: &PaymentLegalActorRequest<'_>) -> Result<PaymentLegalActorResponse, Refusal> {
        self.calls.set(self.calls.get() + 1);
        assert_eq!(req.role, "PAYMENT_BENEFICIARY");
        assert_eq!(req.market, "ZA");
        assert_eq!(req.capability, "payments.payment.capture");
        let t = datetime!(2026-10-09 14:00 UTC) + time::Duration::seconds(self.offset);
        Ok(PaymentLegalActorResponse {
            context_id: req.context_id.to_string(),
            operation_reference: req.operation_reference.to_string(),
            provider_permissions_granted: false,
            legal_actor_resolution: PaymentLegalActorResolution {
                outcome: self.outcome.into(),
                evaluated_at: t.format(&Rfc3339).unwrap(),
                policy_reference: "ADR-BCP-027/LA-04D".into(),
                mandate_id: Some("0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d".into()),
                responsible_legal_entity_id: Some(self.actor.into()),
                evidence_references: Some(vec!["test/registry-evidence".into()]),
                valid_until: Some((t + time::Duration::seconds(20)).format(&Rfc3339).unwrap()),
            },
        })
    }
}

struct Merchant {
    ready: bool,
    calls: Cell<usize>,
}
impl MerchantLegalReadiness for Merchant {
    fn assert_ready(&self, _: &PaymentLegalActorOperation<'_>, _: &str) -> Result<(), Refusal> {
        self.calls.set(self.calls.get() + 1);
        if self.ready { Ok(()) }
        else { Err(Refusal::new(403, "MERCHANT_NOT_READY", "merchant not verified")) }
    }
}

fn op<'a>() -> PaymentLegalActorOperation<'a> {
    PaymentLegalActorOperation {
        context_id: "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6b",
        expected_legal_entity_id: "LE-TEST-BENEFICIARY",
        activity: "B2B_COFFEE_CAPTURE",
        market: "ZA",
        capability: "payments.payment.capture",
        operation_reference: "payment/synthetic-001",
    }
}

fn cp(outcome: &'static str, actor: &'static str, offset: i64) -> Cp {
    Cp { outcome, actor, calls: Cell::new(0), offset }
}
fn now() -> OffsetDateTime { datetime!(2026-10-09 14:00 UTC) }

#[test]
fn valid_legal_beneficiary_and_merchant_rechecked_even_on_replay() {
    let cp = cp("AUTHORIZED", "LE-TEST-BENEFICIARY", 0);
    let merchant = Merchant { ready: true, calls: Cell::new(0) };
    let mut executed = 0;
    for _ in 0..2 {
        assert_eq!(execute_governed_payment(&op(), &cp, &merchant, now(), || {
            executed += 1;
            Ok::<_, Refusal>("sandbox-only")
        }).unwrap(), "sandbox-only");
    }
    assert_eq!((cp.calls.get(), merchant.calls.get(), executed), (2, 2, 2));
}

#[test]
fn denied_or_stale_cp_fact_never_reaches_provider_or_mutation() {
    for (outcome, actor, offset) in [
        ("REVOKED_OR_EXPIRED", "LE-TEST-BENEFICIARY", 0),
        ("AUTHORIZED", "LE-OTHER", 0),
        ("AUTHORIZED", "LE-TEST-BENEFICIARY", -50),
        ("AUTHORIZED", "LE-TEST-BENEFICIARY", -25),
    ] {
        let cp = cp(outcome, actor, offset);
        let merchant = Merchant { ready: true, calls: Cell::new(0) };
        let executed = Cell::new(false);
        assert!(execute_governed_payment(&op(), &cp, &merchant, now(), || {
            executed.set(true);
            Ok::<_, Refusal>(())
        }).is_err());
        assert!(!executed.get());
        assert_eq!(merchant.calls.get(), 0);
    }
}

#[test]
fn current_cp_authorization_is_not_a_merchant_or_payout_grant() {
    let cp = cp("AUTHORIZED", "LE-TEST-BENEFICIARY", 0);
    let merchant = Merchant { ready: false, calls: Cell::new(0) };
    let executed = Cell::new(false);
    assert!(execute_governed_payment(&op(), &cp, &merchant, now(), || {
        executed.set(true); Ok::<_, Refusal>(())
    }).is_err());
    assert!(!executed.get());
    assert_eq!(merchant.calls.get(), 1);
}

#[test]
fn no_default_company_or_market_inference() {
    let cp = cp("AUTHORIZED", "LE-TEST-BENEFICIARY", 0);
    let merchant = Merchant { ready: true, calls: Cell::new(0) };
    let mut invalid = op();
    invalid.expected_legal_entity_id = "";
    assert!(execute_governed_payment(&invalid, &cp, &merchant, now(), || {
        Ok::<_, Refusal>(())
    }).is_err());
    assert_eq!(cp.calls.get(), 0);
    assert_eq!(merchant.calls.get(), 0);
}
