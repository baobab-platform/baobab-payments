//! ADR-BCP-027 LA-05E: opt-in payment beneficiary PEP for the future durable
//! payment execution boundary. The current in-memory sandbox is NOT a live
//! processor/HyperSwitch integration and must remain blocked in production.
//!
//! A tenant/merchant/legal_entity_id in PaymentContext is never evidence of
//! legal authority. The CP-owned RUNTIME context and provider-readiness proof
//! are independent and both mandatory for every money-moving operation.
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::service::Refusal;

#[derive(Debug, Serialize)]
pub struct PaymentLegalActorRequest<'a> {
    pub context_id: &'a str,
    pub role: &'static str,
    pub activity: &'a str,
    pub market: &'a str,
    pub capability: &'a str,
    pub operation_reference: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentLegalActorResolution {
    pub outcome: String,
    pub evaluated_at: String,
    pub policy_reference: String,
    pub mandate_id: Option<String>,
    pub responsible_legal_entity_id: Option<String>,
    pub evidence_references: Option<Vec<String>>,
    pub valid_until: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentLegalActorResponse {
    pub context_id: String,
    pub operation_reference: String,
    pub provider_permissions_granted: bool,
    pub legal_actor_resolution: PaymentLegalActorResolution,
}

/// Explicit current legal-actor intent. Never computed from HyperSwitch's
/// Organisation -> Merchant -> Profile hierarchy or from group sponsorship.
pub struct PaymentLegalActorOperation<'a> {
    pub context_id: &'a str,
    pub expected_legal_entity_id: &'a str,
    pub activity: &'a str,
    pub market: &'a str,
    pub capability: &'a str,
    pub operation_reference: &'a str,
}

pub trait LegalActorAssessmentSource {
    /// An authenticated, no-cache call to CP's LA-05A endpoint under the
    /// same canonical workload principal that owns the RUNTIME context.
    fn assess(
        &self,
        request: &PaymentLegalActorRequest<'_>,
    ) -> Result<PaymentLegalActorResponse, Refusal>;
}

pub trait MerchantLegalReadiness {
    /// Independently verify licensed merchant, profile, settlement account,
    /// market and processor readiness. A CP mandate is NOT a payout grant.
    fn assert_ready(
        &self,
        operation: &PaymentLegalActorOperation<'_>,
        mandate_id: &str,
    ) -> Result<(), Refusal>;
}

fn deny() -> Refusal {
    Refusal::new(403, "LEGAL_ACTOR_NOT_AUTHORISED", "current legal actor and merchant authority required")
}

/// Evaluates the CP fact and processor readiness *before* an execution
/// closure can create, capture or transfer money. This is not yet mounted
/// on the existing sandbox HTTP handlers (see LA-05E acceptance runbook).
pub fn execute_governed_payment<T>(
    operation: &PaymentLegalActorOperation<'_>,
    cp: &dyn LegalActorAssessmentSource,
    merchant: &dyn MerchantLegalReadiness,
    now: OffsetDateTime,
    execute: impl FnOnce() -> Result<T, Refusal>,
) -> Result<T, Refusal> {
    if operation.context_id.is_empty() || operation.expected_legal_entity_id.is_empty()
        || operation.activity.is_empty() || operation.capability.is_empty()
        || operation.operation_reference.is_empty()
        || operation.market.len() != 2
        || !operation.market.bytes().all(|b| b.is_ascii_uppercase())
    {
        return Err(deny());
    }
    let request = PaymentLegalActorRequest {
        context_id: operation.context_id,
        role: "PAYMENT_BENEFICIARY",
        activity: operation.activity,
        market: operation.market,
        capability: operation.capability,
        operation_reference: operation.operation_reference,
    };
    let result = cp.assess(&request)?;
    let fact = &result.legal_actor_resolution;
    let (Some(mandate), Some(entity), Some(evidence), Some(valid_until)) = (
        fact.mandate_id.as_ref(),
        fact.responsible_legal_entity_id.as_ref(),
        fact.evidence_references.as_ref(),
        fact.valid_until.as_ref(),
    ) else {
        return Err(deny());
    };
    let evaluated = OffsetDateTime::parse(&fact.evaluated_at, &Rfc3339).map_err(|_| deny())?;
    let until = OffsetDateTime::parse(valid_until, &Rfc3339).map_err(|_| deny())?;
    if result.context_id != operation.context_id
        || result.operation_reference != operation.operation_reference
        || result.provider_permissions_granted
        || fact.outcome != "AUTHORIZED"
        || fact.policy_reference.is_empty()
        || entity != operation.expected_legal_entity_id
        || mandate.is_empty()
        || evidence.is_empty()
        || evidence.iter().any(|x| x.is_empty())
        || evaluated > now + time::Duration::seconds(1)
        || evaluated < now - time::Duration::seconds(30)
        || until <= now
        || until > evaluated + time::Duration::seconds(31)
    {
        return Err(deny());
    }
    merchant.assert_ready(operation, mandate)?;
    execute()
}
