//! LA-05E6: authenticated, fresh, CP-owned legal-actor and documentary
//! assessments for one native payment operation. No client-supplied CP facts.
//! CP itself verifies JWT audience, canonical workload registry, Context
//! issuance/owner, tenant, primary Organisation and current mandate/restriction.
//! This client NEVER upgrades an unblocked document into a PSP permission.
use std::time::Duration;

use reqwest::{Client, Url};
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::documentary_restriction::require_current_documentary_clearance;
use crate::legal_actor::{
    PaymentLegalOperation, PaymentProviderLegalReadiness,
    require_current_payment_actor,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpAuthorityError {
    Configuration,
    Unavailable,
    NotAuthorised,
}

#[derive(Debug, Clone)]
pub struct CpRuntimeContext {
    pub context_id: Uuid,
    pub primary_organisation_id: Uuid,
    /// Expected responsible legal actor is derived from a trusted business
    /// operation, not from the CP response or a client-selected field.
    pub expected_legal_entity_id: String,
}

#[derive(Clone)]
pub struct IamCpAuthority {
    http: Client,
    cp_origin: Url,
    token_endpoint: Url,
    workload_client_id: String,
    workload_secret: String,
}

impl std::fmt::Debug for IamCpAuthority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IamCpAuthority")
            .field("cp_origin", &self.cp_origin)
            .field("token_endpoint", &self.token_endpoint)
            .field("client_secret", &"<redacted>")
            .finish_non_exhaustive()
    }
}

/// application/x-www-form-urlencoded: encode every non-unreserved byte so
/// client credentials cannot inject extra OAuth request parameters.
fn form_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

impl IamCpAuthority {
    pub fn new(cp_origin: &str, iam_token_endpoint: &str,
        workload_client_id: String, workload_secret: String,
    ) -> Result<Self, CpAuthorityError> {
        Self::with_policy(cp_origin, iam_token_endpoint, workload_client_id, workload_secret, false)
    }

    #[cfg(test)]
    fn loopback(cp_origin: &str, iam_token_endpoint: &str,
        workload_client_id: String, workload_secret: String,
    ) -> Result<Self, CpAuthorityError> {
        Self::with_policy(cp_origin, iam_token_endpoint, workload_client_id, workload_secret, true)
    }

    fn with_policy(cp_origin: &str, iam_token_endpoint: &str, workload_client_id: String,
        workload_secret: String, test_loopback: bool,
    ) -> Result<Self, CpAuthorityError> {
        let cp = Url::parse(cp_origin).map_err(|_| CpAuthorityError::Configuration)?;
        let iam = Url::parse(iam_token_endpoint).map_err(|_| CpAuthorityError::Configuration)?;
        for url in [&cp, &iam] {
            let loopback = matches!(url.host_str(), Some("localhost"|"127.0.0.1"|"[::1]"));
            if !(url.scheme() == "https" || test_loopback && loopback && url.scheme() == "http")
                || url.username() != "" || url.password().is_some()
                || url.fragment().is_some() || url.query().is_some()
            {
                return Err(CpAuthorityError::Configuration);
            }
        }
        if cp.path() != "/" || workload_client_id.is_empty()
            || workload_secret.is_empty() || workload_secret.contains('\n')
        {
            return Err(CpAuthorityError::Configuration);
        }
        let http = Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build().map_err(|_| CpAuthorityError::Configuration)?;
        Ok(Self { http, cp_origin: cp, token_endpoint: iam,
            workload_client_id, workload_secret })
    }

    async fn token(&self) -> Result<String, CpAuthorityError> {
        let form = format!(
            "grant_type=client_credentials&client_id={}&client_secret={}&scope={}",
            form_encode(&self.workload_client_id),
            form_encode(&self.workload_secret),
            form_encode("context:resolve legal-actor:assess documentary:restriction-assess"),
        );
        let response = self.http.post(self.token_endpoint.clone())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form)
            .send().await.map_err(|_| CpAuthorityError::Unavailable)?;
        if !response.status().is_success() {
            return Err(CpAuthorityError::NotAuthorised);
        }
        let bytes = response.bytes().await.map_err(|_| CpAuthorityError::Unavailable)?;
        if bytes.len() > 8_192 { return Err(CpAuthorityError::Unavailable); }
        let token: Value = serde_json::from_slice(&bytes).map_err(|_| CpAuthorityError::Unavailable)?;
        let token = token.get("access_token").and_then(Value::as_str)
            .filter(|s| !s.is_empty() && s.len() <= 7_000
                && s.bytes().all(|b| b.is_ascii_graphic()))
            .ok_or(CpAuthorityError::NotAuthorised)?;
        Ok(token.to_owned())
    }

    async fn post(&self, path: &str, input: &Value) -> Result<Value, CpAuthorityError> {
        // Do not cache either token or governance decisions: revocation may
        // happen between the durable reservation and the PSP instruction.
        let token = self.token().await?;
        let url = self.cp_origin.join(path).map_err(|_| CpAuthorityError::Configuration)?;
        let response = self.http.post(url)
            .bearer_auth(token)
            .header("Cache-Control", "no-store")
            .header("Accept", "application/json")
            .json(input)
            .send().await.map_err(|_| CpAuthorityError::Unavailable)?;
        if !response.status().is_success() { return Err(CpAuthorityError::NotAuthorised); }
        let bytes = response.bytes().await.map_err(|_| CpAuthorityError::Unavailable)?;
        if bytes.len() > 16_384 { return Err(CpAuthorityError::Unavailable); }
        serde_json::from_slice(&bytes).map_err(|_| CpAuthorityError::Unavailable)
    }

    /// Mint a bounded CP runtime Context in the identity of THIS workload;
    /// CP validates the requested tenant, operating Organisation and market.
    /// Never reuse a Context ID minted for a different workload.
    pub async fn issue_current_context(
        &self, tenant_id: &str, organisation_id: Uuid,
        market: &str, expected_legal_entity_id: &str,
    ) -> Result<CpRuntimeContext, CpAuthorityError> {
        if !(tenant_id.starts_with("tn_") && tenant_id.len() <= 63)
            || market.len() != 2 || !market.bytes().all(|b| b.is_ascii_uppercase())
            || expected_legal_entity_id.trim().is_empty()
        {
            return Err(CpAuthorityError::NotAuthorised);
        }
        let response = self.post("/v1/platform-context/resolve", &json!({
            "tenant_id": tenant_id,
            "organisation_id": organisation_id.to_string(),
            "country_code": market,
        })).await?;
        if response.get("tenant_id").and_then(Value::as_str) != Some(tenant_id)
            || response.get("organisation_id").and_then(Value::as_str)
                != Some(organisation_id.to_string().as_str())
            || response.get("country_code").and_then(Value::as_str) != Some(market)
        {
            return Err(CpAuthorityError::NotAuthorised);
        }
        let context_id = response.get("context_id").and_then(Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or(CpAuthorityError::NotAuthorised)?;
        let now = OffsetDateTime::now_utc();
        let from = response.get("resolved_at").and_then(Value::as_str)
            .and_then(|s| OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok())
            .ok_or(CpAuthorityError::NotAuthorised)?;
        let until = response.get("expires_at").and_then(Value::as_str)
            .and_then(|s| OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok())
            .ok_or(CpAuthorityError::NotAuthorised)?;
        if from > now + time::Duration::seconds(1)
            || from < now - time::Duration::seconds(5)
            || until <= now || until > now + time::Duration::minutes(10) {
            return Err(CpAuthorityError::NotAuthorised);
        }
        Ok(CpRuntimeContext { context_id, primary_organisation_id: organisation_id,
            expected_legal_entity_id: expected_legal_entity_id.into() })
    }

    pub async fn verified_mandate(&self, context: &CpRuntimeContext,
        operation: &PaymentLegalOperation<'_>,
    ) -> Result<String, CpAuthorityError> {
        if operation.context_id != context.context_id.to_string()
            || operation.expected_legal_entity_id != context.expected_legal_entity_id
        { return Err(CpAuthorityError::NotAuthorised); }
        let response = self.post("/internal/legal-actor/v1/assess", &operation.request()
            .map_err(|_| CpAuthorityError::NotAuthorised)?).await?;
        require_current_payment_actor(operation, &response, OffsetDateTime::now_utc())
            .map_err(|_| CpAuthorityError::NotAuthorised)
    }

    pub async fn verify_restriction(&self, context: &CpRuntimeContext,
        operation: &PaymentLegalOperation<'_>,
    ) -> Result<(), CpAuthorityError> {
        if operation.context_id != context.context_id.to_string()
        { return Err(CpAuthorityError::NotAuthorised); }
        let body = json!({
            "context_id": operation.context_id,
            "capability": operation.capability,
            "market": operation.market,
            "operation_reference": operation.operation_reference,
        });
        let result = self.post("/internal/documentary/v1/restrictions/assess", &body).await?;
        require_current_documentary_clearance(
            operation, &context.primary_organisation_id.to_string(), &result, OffsetDateTime::now_utc(),
        ).map_err(|_| CpAuthorityError::NotAuthorised)
    }

    pub async fn verify_all(&self, context: &CpRuntimeContext,
        operation: &PaymentLegalOperation<'_>,
        readiness: &dyn PaymentProviderLegalReadiness,
    ) -> Result<String, CpAuthorityError> {
        if operation.capability != "payments.intent.create" {
            return Err(CpAuthorityError::NotAuthorised);
        }
        let first = self.verified_mandate(context, operation).await?;
        self.verify_restriction(context, operation).await?;
        readiness.require_ready(operation, &first)
            .map_err(|_| CpAuthorityError::NotAuthorised)?;
        let second = self.verified_mandate(context, operation, OffsetDateTime::now_utc()).await?;
        if first != second { return Err(CpAuthorityError::NotAuthorised); }
        self.verify_restriction(context, operation, OffsetDateTime::now_utc()).await?;
        Ok(second)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_requires_https_and_does_not_log_secrets() {
        let err = IamCpAuthority::new("http://cp.example/", "https://iam.example/token",
            "baobab-payments-workload".into(), "not-a-real-secret".into());
        assert!(matches!(err, Err(CpAuthorityError::Configuration)));
        let ok = IamCpAuthority::new("https://cp.example/", "https://iam.example/token",
            "baobab-payments-workload".into(), "not-a-real-secret".into()).unwrap();
        assert!(!format!("{ok:?}").contains("not-a-real-secret"));
    }

    #[tokio::test]
    async fn real_http_iam_token_and_cp_response_must_still_bind_exact_operation() {
        use axum::{Router, routing::post, Json};
        let app = Router::new()
            .route("/token", post(|| async {
                Json(json!({"access_token":"synthetic-signed-token-only","expires_in":30}))
            }))
            .route("/internal/legal-actor/v1/assess", post(|| async {
                Json(json!({"context_id":"wrong-foreign-owner",
                    "operation_reference":"transaction-synthetic-1",
                    "provider_permissions_granted":false,
                    "legal_actor_resolution":{"outcome":"AUTHORIZED"}}))
            }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let a = IamCpAuthority::loopback(&format!("http://{addr}/"),
            &format!("http://{addr}/token"), "workload".into(), "synthetic".into()).unwrap();
        let operation = PaymentLegalOperation {
            context_id: "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6b",
            expected_legal_entity_id: "LE-SYNTHETIC-ZA",
            role: "PAYMENT_BENEFICIARY", activity: "B2B_RECEIPT",
            market: "ZA", capability: "payments.intent.create",
            operation_reference: "transaction-synthetic-1",
        };
        let context = CpRuntimeContext {
            context_id: Uuid::parse_str(operation.context_id).unwrap(),
            primary_organisation_id: Uuid::now_v7(),
            expected_legal_entity_id: operation.expected_legal_entity_id.into(),
        };
        assert_eq!(a.verified_mandate(&context, &operation).await,
            Err(CpAuthorityError::NotAuthorised));
        server.abort();
    }
}
