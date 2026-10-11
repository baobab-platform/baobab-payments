//! LA-05E / PEO-02T: current negative-only documentary remediation veto.
//!
//! A clear scope NEVER grants PSP, merchant, financial or legal authority.
//! No runtime activation: PaymentService remains an in-memory sandbox.

use std::future::Future;

use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use crate::legal_actor::{
    LegalActorGateError, PaymentLegalOperation, PaymentProviderLegalReadiness,
    require_current_payment_actor,
};

const POLICY: &str = "ADR-BCP-026/PEO-02T:negative-only";
const MAX_LEASE: Duration = Duration::seconds(2);
const MAX_AGE: Duration = Duration::seconds(3);

/// The requesting workload owns the Context; CP resolves its PRIMARY
/// Organisation rather than permitting a browser/request to select one.
pub fn documentary_request(
    operation: &PaymentLegalOperation<'_>,
) -> Result<Value, LegalActorGateError> {
    operation.request()?;
    Ok(serde_json::json!({
        "context_id": operation.context_id,
        "capability": operation.capability,
        "market": operation.market,
        "operation_reference": operation.operation_reference,
    }))
}

/// Checks the exact operation, server-trusted Organisation, negative-only
/// outcome and a fresh, positive <=2-second validity window.
pub fn require_current_documentary_clearance(
    operation: &PaymentLegalOperation<'_>,
    organisation_id: &str,
    response: &Value,
    now: OffsetDateTime,
) -> Result<(), LegalActorGateError> {
    documentary_request(operation)?;
    if uuid::Uuid::try_parse(organisation_id).is_err()
        || response["context_id"].as_str() != Some(operation.context_id)
        || response["organisation_id"].as_str() != Some(organisation_id)
        || response["market"].as_str() != Some(operation.market)
        || response["capability"].as_str() != Some(operation.capability)
        || response["operation_reference"].as_str() != Some(operation.operation_reference)
        || response["policy_reference"].as_str() != Some(POLICY)
        || response["blocked"] != Value::Bool(false)
        || response["authority_granted"] != Value::Bool(false)
    {
        return Err(LegalActorGateError::NotAuthorised);
    }

    let evaluated = response["evaluated_at"]
        .as_str()
        .and_then(|v| OffsetDateTime::parse(v, &Rfc3339).ok())
        .ok_or(LegalActorGateError::NotAuthorised)?;
    let expires = response["expires_at"]
        .as_str()
        .and_then(|v| OffsetDateTime::parse(v, &Rfc3339).ok())
        .ok_or(LegalActorGateError::NotAuthorised)?;
    if evaluated > now + Duration::seconds(1)
        || evaluated < now - MAX_AGE
        || expires <= now
        || expires <= evaluated
        || expires - evaluated > MAX_LEASE
    {
        return Err(LegalActorGateError::NotAuthorised);
    }
    Ok(())
}

/// Opt-in irreversible-operation seam for future certified provider adapters.
/// Re-assess BOTH current CP legal responsibility and documentary holds after
/// native PSP readiness: a hold or mandate may change during provider I/O.
///
/// These callbacks MUST query CP with the same authenticated workload Context
/// at point of use; passing cached event or client assertions is prohibited.
/// No current PaymentService HTTP handler wires or activates this seam.
pub async fn execute_with_current_documentary_payment_authority<T, F>(
    operation: &PaymentLegalOperation<'_>,
    organisation_id: &str,
    mut current_legal_actor: impl FnMut() -> Result<Value, LegalActorGateError>,
    mut current_documentary_scope: impl FnMut() -> Result<Value, LegalActorGateError>,
    provider: &dyn PaymentProviderLegalReadiness,
    now: impl Fn() -> OffsetDateTime,
    execute: impl FnOnce() -> F,
) -> Result<T, LegalActorGateError>
where
    F: Future<Output = T>,
{
    let mandate = require_current_payment_actor(operation, &current_legal_actor()?, now())?;
    require_current_documentary_clearance(
        operation,
        organisation_id,
        &current_documentary_scope()?,
        now(),
    )?;
    provider.require_ready(operation, &mandate)?;
    let final_mandate = require_current_payment_actor(operation, &current_legal_actor()?, now())?;
    if mandate != final_mandate {
        return Err(LegalActorGateError::NotAuthorised);
    }
    require_current_documentary_clearance(
        operation,
        organisation_id,
        &current_documentary_scope()?,
        now(),
    )?;
    // The mutation future must be awaited IN this guarded operation. Returning
    // an unpolled future would let a caller execute after this lease expired.
    Ok(execute().await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::Cell;

    const ORGANISATION: &str = "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c";

    fn op() -> PaymentLegalOperation<'static> {
        PaymentLegalOperation {
            context_id: "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6b",
            expected_legal_entity_id: "LE-SYNTHETIC-ZA",
            role: "PAYMENT_BENEFICIARY",
            activity: "B2B_RECEIPT",
            market: "ZA",
            capability: "payments.capture",
            operation_reference: "payment-synthetic-123",
        }
    }
    fn clear(now: OffsetDateTime) -> Value {
        json!({
            "context_id": op().context_id,
            "organisation_id": ORGANISATION,
            "market": op().market,
            "capability": op().capability,
            "operation_reference": op().operation_reference,
            "blocked": false,
            "authority_granted": false,
            "policy_reference": POLICY,
            "evaluated_at": now.format(&Rfc3339).unwrap(),
            "expires_at": (now + Duration::seconds(1)).format(&Rfc3339).unwrap(),
        })
    }
    fn legal(now: OffsetDateTime) -> Value {
        json!({
            "context_id": op().context_id,
            "operation_reference": op().operation_reference,
            "provider_permissions_granted": false,
            "legal_actor_resolution": {
                "outcome": "AUTHORIZED",
                "policy_reference": "ADR-BCP-027/LA-04",
                "mandate_id": "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d",
                "responsible_legal_entity_id": op().expected_legal_entity_id,
                "evidence_references": ["synthetic/approved-evidence"],
                "evaluated_at": now.format(&Rfc3339).unwrap(),
                "valid_until": (now + Duration::seconds(10)).format(&Rfc3339).unwrap(),
            }
        })
    }
    struct Ready(bool);
    impl PaymentProviderLegalReadiness for Ready {
        fn require_ready(
            &self,
            _: &PaymentLegalOperation<'_>,
            _: &str,
        ) -> Result<(), LegalActorGateError> {
            if self.0 {
                Ok(())
            } else {
                Err(LegalActorGateError::ProviderNotReady)
            }
        }
    }

    #[test]
    fn clear_scope_has_no_positive_authority_and_validates_exact_tuple() {
        let now = OffsetDateTime::now_utc();
        assert_eq!(
            require_current_documentary_clearance(&op(), ORGANISATION, &clear(now), now),
            Ok(())
        );
        let mut altered = Vec::new();
        for (field, value) in [
            ("context_id", json!("other-context")),
            (
                "organisation_id",
                json!("0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f599"),
            ),
            ("market", json!("UG")),
            ("capability", json!("payments.refund")),
            ("operation_reference", json!("another-operation")),
            ("blocked", json!(true)),
            ("blocked", json!("false")),
            ("authority_granted", json!(true)),
            ("policy_reference", json!("forged-policy")),
            (
                "expires_at",
                json!((now + Duration::seconds(20)).format(&Rfc3339).unwrap()),
            ),
            (
                "expires_at",
                json!((now - Duration::seconds(1)).format(&Rfc3339).unwrap()),
            ),
            (
                "evaluated_at",
                json!((now - Duration::seconds(10)).format(&Rfc3339).unwrap()),
            ),
        ] {
            let mut forged = clear(now);
            forged[field] = value;
            altered.push(forged);
        }
        for value in altered {
            assert_eq!(
                require_current_documentary_clearance(&op(), ORGANISATION, &value, now),
                Err(LegalActorGateError::NotAuthorised),
            );
        }
    }

    #[tokio::test]
    async fn live_rechecks_deny_new_holds_mandate_revocation_and_unready_psp() {
        let now = OffsetDateTime::now_utc();
        let executions = Cell::new(0);
        let legal_calls = Cell::new(0);
        let documentary_calls = Cell::new(0);
        let result = execute_with_current_documentary_payment_authority(
            &op(),
            ORGANISATION,
            || {
                legal_calls.set(legal_calls.get() + 1);
                Ok(legal(now))
            },
            || {
                documentary_calls.set(documentary_calls.get() + 1);
                let mut decision = clear(now);
                if documentary_calls.get() == 2 {
                    decision["blocked"] = json!(true);
                }
                Ok(decision)
            },
            &Ready(true),
            || now,
            || async { executions.set(executions.get() + 1) },
        )
        .await;
        assert_eq!(result, Err(LegalActorGateError::NotAuthorised));
        assert_eq!(
            (legal_calls.get(), documentary_calls.get(), executions.get()),
            (2, 2, 0)
        );

        let calls = Cell::new(0);
        let result = execute_with_current_documentary_payment_authority(
            &op(),
            ORGANISATION,
            || {
                calls.set(calls.get() + 1);
                let mut evidence = legal(now);
                if calls.get() == 2 {
                    evidence["legal_actor_resolution"]["mandate_id"] =
                        json!("0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a99");
                }
                Ok(evidence)
            },
            || Ok(clear(now)),
            &Ready(true),
            || now,
            || async { executions.set(executions.get() + 1) },
        )
        .await;
        assert_eq!(result, Err(LegalActorGateError::NotAuthorised));
        assert_eq!(executions.get(), 0);

        let result = execute_with_current_documentary_payment_authority(
            &op(),
            ORGANISATION,
            || Ok(legal(now)),
            || Ok(clear(now)),
            &Ready(false),
            || now,
            || async { executions.set(executions.get() + 1) },
        )
        .await;
        assert_eq!(result, Err(LegalActorGateError::ProviderNotReady));
        assert_eq!(executions.get(), 0);
    }

    #[tokio::test]
    async fn only_independently_ready_and_double_checked_synthetic_execution_reaches_callback() {
        let now = OffsetDateTime::now_utc();
        let result = execute_with_current_documentary_payment_authority(
            &op(),
            ORGANISATION,
            || Ok(legal(now)),
            || Ok(clear(now)),
            &Ready(true),
            || now,
            || async { "synthetic" },
        )
        .await;
        assert_eq!(result, Ok("synthetic"));
    }
}
