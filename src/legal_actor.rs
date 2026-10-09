//! LA-05E: fail-closed, opt-in payments legal-actor policy enforcement.
//!
//! The current PaymentService is sandbox-only and cannot be production enabled.
//! A PaymentContext.legal_entity_id is a routing datum, NEVER evidence that
//! this entity is the verified PAYMENT_BENEFICIARY / CONTRACTING_PARTY.

use serde_json::{Value, json};
use time::{Duration, OffsetDateTime};
use time::format_description::well_known::Rfc3339;

const MAX_DECISION_AGE: Duration = Duration::seconds(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegalActorGateError {
    MissingContext,
    Unavailable,
    NotAuthorised,
    ProviderNotReady,
}

/// Supplied by a trusted Payments workload context integration, never a buyer
/// or a payment client. CP will bind the context to that workload principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentLegalOperation<'a> {
    pub context_id: &'a str,
    pub expected_legal_entity_id: &'a str,
    pub role: &'a str,
    pub activity: &'a str,
    pub market: &'a str,
    pub capability: &'a str,
    pub operation_reference: &'a str,
}

impl PaymentLegalOperation<'_> {
    pub fn request(&self) -> Result<Value, LegalActorGateError> {
        if self.context_id.is_empty() || self.expected_legal_entity_id.is_empty()
            || self.activity.is_empty() || self.capability.is_empty()
            || self.operation_reference.is_empty()
            || !matches!(self.role, "PAYMENT_BENEFICIARY" | "CONTRACTING_PARTY")
            || self.market.len() != 2
            || !self.market.bytes().all(|b| b.is_ascii_uppercase())
        {
            return Err(LegalActorGateError::MissingContext);
        }
        // Tenant, Organisation, legal actor, mandate, approval evidence and
        // effective time cannot be selected by a caller. CP resolves them.
        Ok(json!({
            "context_id":self.context_id,
            "role":self.role,
            "activity":self.activity,
            "market":self.market,
            "capability":self.capability,
            "operation_reference":self.operation_reference,
        }))
    }
}

/// Validate a fresh CP response. This does NOT grant PSP or settlement
/// permissions. It just proves a scoped, current legal-responsibility fact.
pub fn require_current_payment_actor(
    operation: &PaymentLegalOperation<'_>,
    response: &Value,
    now: OffsetDateTime,
) -> Result<String, LegalActorGateError> {
    operation.request()?;
    if response.get("context_id").and_then(Value::as_str) != Some(operation.context_id)
        || response.get("operation_reference").and_then(Value::as_str) != Some(operation.operation_reference)
        || response.get("provider_permissions_granted") != Some(&Value::Bool(false))
    {
        return Err(LegalActorGateError::NotAuthorised);
    }
    let r = &response["legal_actor_resolution"];
    if r["outcome"].as_str() != Some("AUTHORIZED")
        || r["responsible_legal_entity_id"].as_str() != Some(operation.expected_legal_entity_id)
        || r["policy_reference"].as_str().is_none_or(str::is_empty)
        || r["evidence_references"].as_array().is_none_or(|e| e.is_empty()
            || e.iter().any(|x| x.as_str().is_none_or(str::is_empty)))
    {
        return Err(LegalActorGateError::NotAuthorised);
    }
    let mandate = r["mandate_id"].as_str().filter(|x| uuid::Uuid::parse_str(x).is_ok())
        .ok_or(LegalActorGateError::NotAuthorised)?;
    let dates = || -> Option<(OffsetDateTime, OffsetDateTime)> {
        let evaluated = OffsetDateTime::parse(r["evaluated_at"].as_str()?, &Rfc3339).ok()?;
        let until = OffsetDateTime::parse(r["valid_until"].as_str()?, &Rfc3339).ok()?;
        Some((evaluated, until))
    };
    let (evaluated, until) = dates().ok_or(LegalActorGateError::NotAuthorised)?;
    if evaluated > now + Duration::seconds(1)
        || evaluated < now - MAX_DECISION_AGE
        || until <= now
        || until > evaluated + MAX_DECISION_AGE + Duration::seconds(1)
    {
        return Err(LegalActorGateError::NotAuthorised);
    }
    Ok(mandate.to_owned())
}

/// PSP/merchant mapping, market, beneficiary, KYC and settlement readiness
/// are deliberately independent of CP's legal-actor resolution.
pub trait PaymentProviderLegalReadiness {
    fn require_ready(
        &self, operation: &PaymentLegalOperation<'_>, mandate_id: &str
    ) -> Result<(), LegalActorGateError>;
}

/// Must wrap each irreversible payment action/retry, not merely an initial
/// payment-intent creation. An error cannot run `execute`.
pub fn execute_with_current_payment_actor<T>(
    operation: &PaymentLegalOperation<'_>,
    cp_result: Result<&Value, LegalActorGateError>,
    provider: &dyn PaymentProviderLegalReadiness,
    now: OffsetDateTime,
    execute: impl FnOnce() -> T,
) -> Result<T, LegalActorGateError> {
    let body = cp_result?;
    let mandate = require_current_payment_actor(operation, body, now)?;
    provider.require_ready(operation, &mandate)?;
    Ok(execute())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn operation() -> PaymentLegalOperation<'static> {
        PaymentLegalOperation {
            context_id:"0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6b",
            expected_legal_entity_id:"LE-SYNTHETIC-ZA",
            role:"PAYMENT_BENEFICIARY",
            activity:"B2B_RECEIPT", market:"ZA",
            capability:"payments.capture",
            operation_reference:"payment-transaction-123",
        }
    }
    fn evidence(now: OffsetDateTime) -> Value {
        json!({
            "context_id":operation().context_id,
            "operation_reference":operation().operation_reference,
            "provider_permissions_granted":false,
            "legal_actor_resolution":{
                "outcome":"AUTHORIZED",
                "policy_reference":"ADR-BCP-027",
                "mandate_id":"0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d",
                "responsible_legal_entity_id":"LE-SYNTHETIC-ZA",
                "evidence_references":["synthetic/approved-registration"],
                "evaluated_at":now.format(&Rfc3339).unwrap(),
                "valid_until":(now+Duration::seconds(10)).format(&Rfc3339).unwrap(),
            }
        })
    }
    struct Ready(bool);
    impl PaymentProviderLegalReadiness for Ready {
        fn require_ready(&self, _: &PaymentLegalOperation<'_>, _: &str)
            -> Result<(), LegalActorGateError> {
            if self.0 { Ok(()) } else { Err(LegalActorGateError::ProviderNotReady) }
        }
    }
    #[test]
    fn authorised_cp_and_psp_allows_only_single_intended_action() {
        let now = OffsetDateTime::now_utc();
        let evidence = evidence(now);
        let called = Cell::new(0);
        let result = execute_with_current_payment_actor(
            &operation(), Ok(&evidence), &Ready(true), now,
            || { called.set(called.get()+1); "captured" });
        assert_eq!(result,Ok("captured"));
        assert_eq!(called.get(),1);
    }
    #[test]
    fn refuses_revoked_wrong_actor_wrong_context_stale_and_psp_not_ready() {
        let now = OffsetDateTime::now_utc();
        let mut cases = Vec::new();
        let mut revoked=evidence(now);
        revoked["legal_actor_resolution"]["outcome"]=json!("REVOKED_OR_EXPIRED");
        cases.push(revoked);
        let mut foreign=evidence(now);
        foreign["legal_actor_resolution"]["responsible_legal_entity_id"]=json!("LE-OTHER");
        cases.push(foreign);
        let mut context=evidence(now);
        context["context_id"]=json!("foreign-context");
        cases.push(context);
        let mut expired=evidence(now);
        expired["legal_actor_resolution"]["valid_until"]=json!((now-Duration::seconds(1)).format(&Rfc3339).unwrap());
        cases.push(expired);
        let mut too_old=evidence(now);
        too_old["legal_actor_resolution"]["evaluated_at"]=json!((now-Duration::seconds(31)).format(&Rfc3339).unwrap());
        cases.push(too_old);
        for candidate in &cases {
            let called=Cell::new(false);
            assert_eq!(execute_with_current_payment_actor(
                &operation(),Ok(candidate),&Ready(true),now,||called.set(true)),
                Err(LegalActorGateError::NotAuthorised));
            assert!(!called.get());
        }
        let good=evidence(now);
        let called=Cell::new(false);
        assert_eq!(execute_with_current_payment_actor(
            &operation(),Ok(&good),&Ready(false),now,||called.set(true)),
            Err(LegalActorGateError::ProviderNotReady));
        assert!(!called.get());
    }
    #[test]
    fn operation_cannot_select_tenant_organisation_actor_or_mandate() {
        let request=operation().request().unwrap();
        assert!(request.get("tenant_id").is_none());
        assert!(request.get("operating_organisation_id").is_none());
        assert!(request.get("responsible_legal_entity_id").is_none());
        assert!(request.get("mandate_id").is_none());
    }
}
