//! P0-04B Payments (ADR-SHARED-035): the catalogued money movements, the
//! canonical instruction a CP decision is bound to, and the checks Payments
//! applies to that decision. CP legal authority and Payments' own financial
//! approval are independent: both must permit, neither overrides the other.

use serde::Serialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::contracts::entry_is_valid;

const MAX_LEASE: Duration = Duration::seconds(30);

/// The pinned `payments/v2/money-movement-operations.yaml` catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationClass {
    Authorize,
    Capture,
    Refund,
    Void,
    /// Catalogued so that every route to it is refused by name. Collection
    /// authority is never disbursement authority (ADR-PAY-0016).
    Payout,
}

impl OperationClass {
    pub const ALL: [Self; 5] = [
        Self::Authorize,
        Self::Capture,
        Self::Refund,
        Self::Void,
        Self::Payout,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Authorize => "payments.authorization.create",
            Self::Capture => "payments.capture.execute",
            Self::Refund => "payments.refund.execute",
            Self::Void => "payments.authorization.void",
            Self::Payout => "payments.payout.execute",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == raw)
    }

    /// The journal `operation_kind` the reservation is recorded under.
    pub fn journal_kind(self) -> &'static str {
        match self {
            Self::Authorize => "AUTHORIZE",
            Self::Capture => "CAPTURE",
            Self::Refund => "REFUND",
            Self::Void => "CANCEL",
            Self::Payout => "PAYOUT",
        }
    }

    pub fn activatable(self) -> bool {
        !matches!(self, Self::Payout)
    }

    pub fn activity(self) -> &'static str {
        match self {
            Self::Authorize | Self::Capture => "PAYMENT_COLLECTION",
            Self::Refund => "PAYMENT_REFUND",
            Self::Void => "PAYMENT_VOID",
            Self::Payout => "BENEFICIARY_DISBURSEMENT",
        }
    }

    pub fn capability(self) -> Option<&'static str> {
        match self {
            Self::Authorize => Some("payment.payment.authorize"),
            Self::Capture => Some("payment.payment.capture"),
            Self::Refund => Some("payment.refund.create"),
            Self::Void => Some("payment.intent.cancel"),
            Self::Payout => None,
        }
    }
}

/// Refunds may only return funds to where they came from (ADR-PAY-0013).
pub const ORIGINAL_PAYMENT_METHOD: &str = "ORIGINAL_PAYMENT_METHOD";

/// The canonical, server-verified values of one financial instruction. Built
/// from the CP RUNTIME context, Payments' own durable records and its merchant
/// profile; never copied from a client body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Instruction {
    pub tenant_id: String,
    pub operating_organisation_id: String,
    pub market: String,
    pub currency: String,
    pub amount_minor: i64,
    pub payment_reference: String,
    pub parent_reference: Option<String>,
    pub merchant_profile_reference: String,
    pub destination_reference: Option<String>,
}

fn printable(value: &str) -> bool {
    (1..=256).contains(&value.len()) && value.bytes().all(|b| (0x21..=0x7e).contains(&b))
}

impl Instruction {
    /// Per-class shape (Shared request schema): authorisation has no parent or
    /// destination; capture and void name the parent; a refund names the parent
    /// and the original payment method only; a payout names a destination.
    pub fn shape_is_valid(&self, class: OperationClass) -> bool {
        let refs = printable(&self.payment_reference)
            && printable(&self.merchant_profile_reference)
            && self.parent_reference.as_deref().is_none_or(printable)
            && self.destination_reference.as_deref().is_none_or(printable);
        let scalars = self.amount_minor > 0
            && self.market.len() == 2
            && self.market.bytes().all(|b| b.is_ascii_uppercase())
            && self.currency.len() == 3
            && self.currency.bytes().all(|b| b.is_ascii_uppercase());
        let per_class = match class {
            OperationClass::Authorize => {
                self.parent_reference.is_none() && self.destination_reference.is_none()
            }
            OperationClass::Capture | OperationClass::Void => {
                self.parent_reference.is_some() && self.destination_reference.is_none()
            }
            OperationClass::Refund => {
                self.parent_reference.is_some()
                    && self.destination_reference.as_deref() == Some(ORIGINAL_PAYMENT_METHOD)
            }
            OperationClass::Payout => {
                self.parent_reference.is_none() && self.destination_reference.is_some()
            }
        };
        refs && scalars && per_class
    }
}

/// The canonical encoding of `payments/v2/README.md`: exactly eleven keys in
/// ascending order, no insignificant whitespace, UTF-8, null for an absent
/// reference. serde_json's default map is ordered, and it writes non-ASCII as
/// UTF-8 rather than `\u` escapes, matching the Shared vectors.
pub fn canonical(
    class: OperationClass,
    operation_reference: &str,
    instruction: &Instruction,
) -> String {
    let mut doc = match serde_json::to_value(instruction).expect("instruction serialises") {
        Value::Object(map) => map,
        _ => unreachable!("an instruction is an object"),
    };
    doc.insert("operation_class".into(), json!(class.as_str()));
    doc.insert("operation_reference".into(), json!(operation_reference));
    let ordered: Map<String, Value> = doc.into_iter().collect();
    Value::Object(ordered).to_string()
}

pub fn fingerprint(
    class: OperationClass,
    operation_reference: &str,
    instruction: &Instruction,
) -> String {
    hex::encode(Sha256::digest(
        canonical(class, operation_reference, instruction).as_bytes(),
    ))
}

/// One `MoneyMovementAuthorityRequest`, validated against the pinned schema
/// before it can reach CP.
pub fn authority_request(
    context_id: Uuid,
    engine_instance_id: &str,
    class: OperationClass,
    operation_reference: &str,
    attempt: u16,
    instruction: &Instruction,
) -> Option<Value> {
    let request = json!({
        "context_id": context_id.to_string(),
        "engine_instance_id": engine_instance_id,
        "operation_class": class.as_str(),
        "operation_reference": operation_reference,
        "attempt": attempt,
        "instruction": instruction,
        "instruction_fingerprint": fingerprint(class, operation_reference, instruction),
    });
    entry_is_valid("money_movement_request", &request).then_some(request)
}

/// The parts of an AUTHORIZED decision Payments journals and fences on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedDecision {
    pub decision_id: Uuid,
    pub mandate_id: Uuid,
    pub attempt: u16,
    pub instruction_fingerprint: String,
    pub evaluated_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionRefusal {
    /// CP decided, and the answer was not AUTHORIZED (or not for this actor).
    Denied(String),
    /// The response is not a trustworthy answer to this request.
    Invalid,
}

fn timestamp(value: &Value) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value.as_str()?, &Rfc3339).ok()
}

/// Accept a CP decision only if it answers exactly this request, is AUTHORIZED
/// for the expected responsible legal entity and still within a lease of at
/// most 30 seconds. Never a provider permission.
pub fn require_authorized(
    request: &Value,
    response: &Value,
    expected_legal_entity_id: &str,
    now: OffsetDateTime,
) -> Result<AuthorizedDecision, DecisionRefusal> {
    if !entry_is_valid("money_movement_decision", response) {
        return Err(DecisionRefusal::Invalid);
    }
    for key in [
        "context_id",
        "engine_instance_id",
        "operation_class",
        "operation_reference",
        "attempt",
        "instruction_fingerprint",
    ] {
        if response.get(key) != request.get(key) {
            return Err(DecisionRefusal::Invalid);
        }
    }
    let instruction = &request["instruction"];
    if response["provider_permissions_granted"] != json!(false) {
        return Err(DecisionRefusal::Invalid);
    }
    let outcome = response["outcome"].as_str().unwrap_or_default();
    if outcome != "AUTHORIZED" {
        return Err(DecisionRefusal::Denied(outcome.to_owned()));
    }
    if response["responsible_legal_entity_id"].as_str() != Some(expected_legal_entity_id)
        || response["tenant_id"] != instruction["tenant_id"]
        || response["operating_organisation_id"] != instruction["operating_organisation_id"]
        || response["market"] != instruction["market"]
    {
        return Err(DecisionRefusal::Denied("ACTOR_MISMATCH".into()));
    }
    let class = request["operation_class"]
        .as_str()
        .and_then(OperationClass::parse)
        .ok_or(DecisionRefusal::Invalid)?;
    if response["activity"].as_str() != Some(class.activity())
        || response["capability_key"].as_str() != class.capability()
    {
        return Err(DecisionRefusal::Invalid);
    }
    let evaluated = timestamp(&response["evaluated_at"]).ok_or(DecisionRefusal::Invalid)?;
    let expires = timestamp(&response["expires_at"]).ok_or(DecisionRefusal::Invalid)?;
    if expires <= evaluated
        || expires > evaluated + MAX_LEASE
        || evaluated > now + Duration::seconds(1)
        || expires <= now
    {
        return Err(DecisionRefusal::Denied("REVOKED_OR_EXPIRED".into()));
    }
    let uuid = |key: &str| response[key].as_str().and_then(|s| Uuid::parse_str(s).ok());
    Ok(AuthorizedDecision {
        decision_id: uuid("decision_id").ok_or(DecisionRefusal::Invalid)?,
        mandate_id: uuid("mandate_id").ok_or(DecisionRefusal::Invalid)?,
        attempt: response["attempt"]
            .as_u64()
            .and_then(|a| u16::try_from(a).ok())
            .ok_or(DecisionRefusal::Invalid)?,
        instruction_fingerprint: response["instruction_fingerprint"]
            .as_str()
            .ok_or(DecisionRefusal::Invalid)?
            .to_owned(),
        evaluated_at: evaluated,
        expires_at: expires,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vectors() -> Vec<Value> {
        let raw = crate::contracts::embedded("payments/v2/examples/fingerprint-vectors.json")
            .expect("pinned vectors");
        serde_json::from_str::<Value>(raw).unwrap()["vectors"]
            .as_array()
            .unwrap()
            .clone()
    }

    fn instruction(v: &Value) -> Instruction {
        let s = |k: &str| v["instruction"][k].as_str().map(str::to_owned);
        Instruction {
            tenant_id: s("tenant_id").unwrap(),
            operating_organisation_id: s("operating_organisation_id").unwrap(),
            market: s("market").unwrap(),
            currency: s("currency").unwrap(),
            amount_minor: v["instruction"]["amount_minor"].as_i64().unwrap(),
            payment_reference: s("payment_reference").unwrap(),
            parent_reference: s("parent_reference"),
            merchant_profile_reference: s("merchant_profile_reference").unwrap(),
            destination_reference: s("destination_reference"),
        }
    }

    #[test]
    fn reproduces_every_shared_fingerprint_vector() {
        let vectors = vectors();
        assert!(vectors.len() >= 6);
        for v in &vectors {
            let class = OperationClass::parse(v["operation_class"].as_str().unwrap()).unwrap();
            let reference = v["operation_reference"].as_str().unwrap();
            let i = instruction(v);
            assert_eq!(
                canonical(class, reference, &i),
                v["canonical"].as_str().unwrap()
            );
            assert_eq!(
                fingerprint(class, reference, &i),
                v["instruction_fingerprint"].as_str().unwrap()
            );
            assert!(i.shape_is_valid(class), "{reference}");
        }
    }

    #[test]
    fn every_financial_field_moves_the_fingerprint() {
        let v = &vectors()[1];
        let class = OperationClass::Capture;
        let reference = v["operation_reference"].as_str().unwrap();
        let base = instruction(v);
        let original = fingerprint(class, reference, &base);
        let mut changes: Vec<Instruction> = Vec::new();
        for mutate in [
            (|i: &mut Instruction| i.amount_minor += 1) as fn(&mut Instruction),
            |i| i.currency = "KES".into(),
            |i| i.market = "KE".into(),
            |i| i.tenant_id = "tn_other01".into(),
            |i| i.merchant_profile_reference = "mprof_other".into(),
            |i| i.parent_reference = Some("pay_other".into()),
            |i| i.payment_reference = "pay_other".into(),
            |i| i.destination_reference = Some("bnf_other".into()),
            |i| i.operating_organisation_id = "3d9f5a7b-2c4e-4f6a-9b8d-0e1f2a3b4c5d".into(),
        ] {
            let mut changed = base.clone();
            mutate(&mut changed);
            changes.push(changed);
        }
        for changed in changes {
            assert_ne!(fingerprint(class, reference, &changed), original);
        }
        assert_ne!(
            fingerprint(OperationClass::Refund, reference, &base),
            original
        );
        assert_ne!(fingerprint(class, "other", &base), original);
    }

    #[test]
    fn catalogue_matches_the_pinned_owner() {
        let raw = crate::contracts::embedded("payments/v2/money-movement-operations.yaml").unwrap();
        for class in OperationClass::ALL {
            let block = raw
                .split("- operation_class: ")
                .find(|b| b.starts_with(class.as_str()))
                .unwrap_or_else(|| panic!("{} not catalogued", class.as_str()));
            assert!(block.contains(&format!("activity: {}", class.activity())));
            assert!(block.contains(&format!("activatable: {}", class.activatable())));
            match class.capability() {
                Some(c) => assert!(block.contains(&format!("capability_key: {c}"))),
                None => assert!(block.contains("capability_key: null")),
            }
        }
        assert_eq!(
            raw.matches("- operation_class: ").count(),
            OperationClass::ALL.len()
        );
    }

    #[test]
    fn instruction_shape_is_bound_to_its_class() {
        let v = &vectors()[3];
        let refund = instruction(v);
        assert!(refund.shape_is_valid(OperationClass::Refund));
        let mut elsewhere = refund.clone();
        elsewhere.destination_reference = Some("bnf_elsewhere".into());
        assert!(!elsewhere.shape_is_valid(OperationClass::Refund));
        assert!(!refund.shape_is_valid(OperationClass::Capture));
        let mut zero = refund.clone();
        zero.amount_minor = 0;
        assert!(!zero.shape_is_valid(OperationClass::Refund));
    }

    fn decision(request: &Value, now: OffsetDateTime) -> Value {
        json!({
            "decision_id": "a3e9c7d1-5b2f-4e6a-8c1d-9f0b7e3a2c55",
            "context_id": request["context_id"], "engine_instance_id": request["engine_instance_id"],
            "operation_class": request["operation_class"], "operation_reference": request["operation_reference"],
            "attempt": request["attempt"], "instruction_fingerprint": request["instruction_fingerprint"],
            "outcome": "AUTHORIZED", "tenant_id": request["instruction"]["tenant_id"],
            "operating_organisation_id": request["instruction"]["operating_organisation_id"],
            "market": request["instruction"]["market"], "legal_actor_role": "PAYMENT_BENEFICIARY",
            "activity": "PAYMENT_COLLECTION", "capability_key": "payment.payment.capture",
            "mandate_id": "7d2b9e4f-6a1c-4f3e-b8d5-0c9a8e7f6d22", "responsible_legal_entity_id": "SYNTHETIC-LE-01",
            "evidence_references": ["synthetic:mandate-evidence/02"],
            "mandate_effective_from": "2026-10-01T00:00:00Z",
            "evaluated_at": now.format(&Rfc3339).unwrap(),
            "expires_at": (now + Duration::seconds(30)).format(&Rfc3339).unwrap(),
            "policy_reference": "ADR-SHARED-035/payments-money-movement-authority:v1",
            "provider_permissions_granted": false
        })
    }

    fn capture_request() -> Value {
        let v = &vectors()[1];
        authority_request(
            Uuid::parse_str("6f1c2a9e-4b7d-4c1e-9a3f-2d5e8b7c1a40").unwrap(),
            "ei_paystaging01",
            OperationClass::Capture,
            v["operation_reference"].as_str().unwrap(),
            2,
            &instruction(v),
        )
        .unwrap()
    }

    #[test]
    fn accepts_only_an_exact_current_authorized_answer() {
        let now = OffsetDateTime::now_utc();
        let request = capture_request();
        let good = decision(&request, now);
        let accepted = require_authorized(&request, &good, "SYNTHETIC-LE-01", now).unwrap();
        assert_eq!(accepted.attempt, 2);
        assert_eq!(
            accepted.instruction_fingerprint,
            request["instruction_fingerprint"]
        );

        let refused = |mutate: &dyn Fn(&mut Value), le: &str| {
            let mut d = good.clone();
            mutate(&mut d);
            require_authorized(&request, &d, le, now).is_err()
        };
        assert!(refused(&|_| {}, "LE-OTHER"), "another legal entity");
        assert!(
            refused(&|d| d["attempt"] = json!(1), "SYNTHETIC-LE-01"),
            "another attempt"
        );
        assert!(refused(
            &|d| d["instruction_fingerprint"] = json!("0".repeat(64)),
            "SYNTHETIC-LE-01"
        ));
        assert!(refused(
            &|d| d["operation_reference"] = json!("other"),
            "SYNTHETIC-LE-01"
        ));
        assert!(
            refused(&|d| d["market"] = json!("KE"), "SYNTHETIC-LE-01"),
            "another market"
        );
        assert!(refused(
            &|d| d["tenant_id"] = json!("tn_other01"),
            "SYNTHETIC-LE-01"
        ));
        assert!(refused(
            &|d| d["provider_permissions_granted"] = json!(true),
            "SYNTHETIC-LE-01"
        ));
        assert!(refused(
            &|d| d["capability_key"] = json!("payment.refund.create"),
            "SYNTHETIC-LE-01"
        ));
        assert!(
            refused(
                &|d| d["expires_at"] =
                    json!((now - Duration::seconds(1)).format(&Rfc3339).unwrap()),
                "SYNTHETIC-LE-01"
            ),
            "expired"
        );
        let mut denied = good.clone();
        for key in [
            "mandate_id",
            "responsible_legal_entity_id",
            "evidence_references",
            "mandate_effective_from",
            "expires_at",
            "capability_key",
        ] {
            denied.as_object_mut().unwrap().remove(key);
        }
        denied["outcome"] = json!("REVOKED_OR_EXPIRED");
        assert_eq!(
            require_authorized(&request, &denied, "SYNTHETIC-LE-01", now),
            Err(DecisionRefusal::Denied("REVOKED_OR_EXPIRED".into()))
        );
    }

    #[test]
    fn payout_is_never_activatable_and_has_no_capability() {
        assert!(!OperationClass::Payout.activatable());
        assert_eq!(OperationClass::Payout.capability(), None);
        assert!(OperationClass::ALL[..4].iter().all(|c| c.activatable()));
    }
}
