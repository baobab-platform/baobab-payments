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

use crate::contracts::entry_is_valid;
use crate::documentary_restriction::{documentary_request, require_current_documentary_clearance};
use crate::legal_actor::{
    PaymentLegalOperation, PaymentProviderLegalReadiness, require_current_payment_actor,
};
use crate::wire_json::decode_object;

/// Bound retained bytes before copying a response chunk. Checking size after
/// Response::bytes() would already have buffered an unbounded body. reqwest's
/// total timeout covers headers AND body; redirects stay disabled on the client.
async fn bounded_object(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Value, CpAuthorityError> {
    if response.content_length().is_some_and(|length| length > limit as u64) {
        return Err(CpAuthorityError::Unavailable);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| CpAuthorityError::Unavailable)? {
        if chunk.len() > limit - bytes.len() {
            return Err(CpAuthorityError::Unavailable);
        }
        bytes.extend_from_slice(&chunk);
    }
    decode_object(&bytes).map_err(|_| CpAuthorityError::Unavailable)
}

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
    pub fn new(
        cp_origin: &str,
        iam_token_endpoint: &str,
        workload_client_id: String,
        workload_secret: String,
    ) -> Result<Self, CpAuthorityError> {
        Self::with_policy(
            cp_origin,
            iam_token_endpoint,
            workload_client_id,
            workload_secret,
            false,
        )
    }

    #[cfg(test)]
    fn loopback(
        cp_origin: &str,
        iam_token_endpoint: &str,
        workload_client_id: String,
        workload_secret: String,
    ) -> Result<Self, CpAuthorityError> {
        Self::with_policy(
            cp_origin,
            iam_token_endpoint,
            workload_client_id,
            workload_secret,
            true,
        )
    }

    fn with_policy(
        cp_origin: &str,
        iam_token_endpoint: &str,
        workload_client_id: String,
        workload_secret: String,
        test_loopback: bool,
    ) -> Result<Self, CpAuthorityError> {
        let cp = Url::parse(cp_origin).map_err(|_| CpAuthorityError::Configuration)?;
        let iam = Url::parse(iam_token_endpoint).map_err(|_| CpAuthorityError::Configuration)?;
        for url in [&cp, &iam] {
            let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
            if !(url.scheme() == "https" || test_loopback && loopback && url.scheme() == "http")
                || url.username() != ""
                || url.password().is_some()
                || url.fragment().is_some()
                || url.query().is_some()
            {
                return Err(CpAuthorityError::Configuration);
            }
        }
        if cp.path() != "/"
            || workload_client_id.is_empty()
            || workload_secret.is_empty()
            || workload_secret.contains('\n')
        {
            return Err(CpAuthorityError::Configuration);
        }
        let http = Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| CpAuthorityError::Configuration)?;
        Ok(Self {
            http,
            cp_origin: cp,
            token_endpoint: iam,
            workload_client_id,
            workload_secret,
        })
    }

    async fn token(&self) -> Result<String, CpAuthorityError> {
        let form = format!(
            "grant_type=client_credentials&client_id={}&client_secret={}&scope={}",
            form_encode(&self.workload_client_id),
            form_encode(&self.workload_secret),
            form_encode("context:resolve legal-actor:assess documentary:restriction-assess"),
        );
        let response = self
            .http
            .post(self.token_endpoint.clone())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form)
            .send()
            .await
            .map_err(|_| CpAuthorityError::Unavailable)?;
        if !response.status().is_success() {
            return Err(CpAuthorityError::NotAuthorised);
        }
        // This endpoint is an OAuth client-credentials provider, not the
        // Baobab token-exchange contract. Retain its provider-neutral envelope.
        let token = bounded_object(response, 8_192).await?;
        let token = token
            .get("access_token")
            .and_then(Value::as_str)
            .filter(|s| {
                !s.is_empty() && s.len() <= 7_000 && s.bytes().all(|b| b.is_ascii_graphic())
            })
            .ok_or(CpAuthorityError::NotAuthorised)?;
        Ok(token.to_owned())
    }

    async fn post(
        &self,
        path: &str,
        input: &Value,
        request_entry: &str,
        response_entry: &str,
    ) -> Result<Value, CpAuthorityError> {
        // Invalid local operations must not even obtain a token or contact CP.
        if !entry_is_valid(request_entry, input) {
            return Err(CpAuthorityError::NotAuthorised);
        }
        // Do not cache either token or governance decisions: revocation may
        // happen between the durable reservation and the PSP instruction.
        let token = self.token().await?;
        let url = self
            .cp_origin
            .join(path)
            .map_err(|_| CpAuthorityError::Configuration)?;
        let response = self
            .http
            .post(url)
            .bearer_auth(token)
            .header("Cache-Control", "no-store")
            .header("Accept", "application/json")
            .json(input)
            .send()
            .await
            .map_err(|_| CpAuthorityError::Unavailable)?;
        if !response.status().is_success() {
            return Err(CpAuthorityError::NotAuthorised);
        }
        let body = bounded_object(response, 16_384).await?;
        if !entry_is_valid(response_entry, &body) {
            return Err(CpAuthorityError::NotAuthorised);
        }
        Ok(body)
    }

    /// Mint a bounded CP runtime Context in the identity of THIS workload;
    /// CP validates the requested tenant, operating Organisation and market.
    /// Never reuse a Context ID minted for a different workload.
    pub async fn issue_current_context(
        &self,
        tenant_id: &str,
        organisation_id: Uuid,
        market: &str,
        expected_legal_entity_id: &str,
    ) -> Result<CpRuntimeContext, CpAuthorityError> {
        if !(tenant_id.starts_with("tn_") && tenant_id.len() <= 63)
            || market.len() != 2
            || !market.bytes().all(|b| b.is_ascii_uppercase())
            || expected_legal_entity_id.trim().is_empty()
        {
            return Err(CpAuthorityError::NotAuthorised);
        }
        let response = self
            .post(
                "/v1/platform-context/resolve",
                &json!({
                    "tenant_id": tenant_id,
                    "organisation_id": organisation_id.to_string(),
                    "country_code": market,
                }),
                "context_request",
                "context_response",
            )
            .await?;
        if response.get("tenant_id").and_then(Value::as_str) != Some(tenant_id)
            || response.get("organisation_id").and_then(Value::as_str)
                != Some(organisation_id.to_string().as_str())
            || response.get("country_code").and_then(Value::as_str) != Some(market)
        {
            return Err(CpAuthorityError::NotAuthorised);
        }
        let context_id = response
            .get("context_id")
            .and_then(Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or(CpAuthorityError::NotAuthorised)?;
        let now = OffsetDateTime::now_utc();
        let from = response
            .get("resolved_at")
            .and_then(Value::as_str)
            .and_then(|s| {
                OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
            })
            .ok_or(CpAuthorityError::NotAuthorised)?;
        let until = response
            .get("expires_at")
            .and_then(Value::as_str)
            .and_then(|s| {
                OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
            })
            .ok_or(CpAuthorityError::NotAuthorised)?;
        if from > now + time::Duration::seconds(1)
            || from < now - time::Duration::seconds(5)
            || until <= now
            || until > now + time::Duration::minutes(10)
        {
            return Err(CpAuthorityError::NotAuthorised);
        }
        Ok(CpRuntimeContext {
            context_id,
            primary_organisation_id: organisation_id,
            expected_legal_entity_id: expected_legal_entity_id.into(),
        })
    }

    pub async fn verified_mandate(
        &self,
        context: &CpRuntimeContext,
        operation: &PaymentLegalOperation<'_>,
    ) -> Result<String, CpAuthorityError> {
        if operation.context_id != context.context_id.to_string()
            || operation.expected_legal_entity_id != context.expected_legal_entity_id
        {
            return Err(CpAuthorityError::NotAuthorised);
        }
        let response = self
            .post(
                "/internal/legal-actor/v1/assess",
                &operation
                    .request()
                    .map_err(|_| CpAuthorityError::NotAuthorised)?,
                "legal_request",
                "legal_response",
            )
            .await?;
        require_current_payment_actor(operation, &response, OffsetDateTime::now_utc())
            .map_err(|_| CpAuthorityError::NotAuthorised)
    }

    pub async fn verify_restriction(
        &self,
        context: &CpRuntimeContext,
        operation: &PaymentLegalOperation<'_>,
    ) -> Result<(), CpAuthorityError> {
        if operation.context_id != context.context_id.to_string() {
            return Err(CpAuthorityError::NotAuthorised);
        }
        let body = documentary_request(operation).map_err(|_| CpAuthorityError::NotAuthorised)?;
        let result = self
            .post(
                "/internal/documentary/v1/restrictions/assess",
                &body,
                "documentary_request",
                "documentary_response",
            )
            .await?;
        require_current_documentary_clearance(
            operation,
            &context.primary_organisation_id.to_string(),
            &result,
            OffsetDateTime::now_utc(),
        )
        .map_err(|_| CpAuthorityError::NotAuthorised)
    }

    pub async fn verify_all(
        &self,
        context: &CpRuntimeContext,
        operation: &PaymentLegalOperation<'_>,
        readiness: &dyn PaymentProviderLegalReadiness,
    ) -> Result<String, CpAuthorityError> {
        if operation.capability != "payments.intent.create" {
            return Err(CpAuthorityError::NotAuthorised);
        }
        let first = self.verified_mandate(context, operation).await?;
        self.verify_restriction(context, operation).await?;
        readiness
            .require_ready(operation, &first)
            .map_err(|_| CpAuthorityError::NotAuthorised)?;
        let second = self.verified_mandate(context, operation).await?;
        if first != second {
            return Err(CpAuthorityError::NotAuthorised);
        }
        self.verify_restriction(context, operation).await?;
        Ok(second)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn serve(app: axum::Router) -> (IamCpAuthority, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let authority = IamCpAuthority::loopback(
            &format!("http://{addr}/"), &format!("http://{addr}/token"),
            "workload".into(), "synthetic".into(),
        ).unwrap();
        (authority, server)
    }

    #[tokio::test]
    async fn invalid_context_input_never_contacts_iam_or_cp() {
        // No listener exists: returning NotAuthorised instead of Unavailable
        // proves local grammar is checked before any network attempt.
        let authority = IamCpAuthority::loopback(
            "http://127.0.0.1:1/", "http://127.0.0.1:1/token",
            "workload".into(), "synthetic".into(),
        ).unwrap();
        assert!(matches!(
            authority.issue_current_context("tn_bad!", Uuid::now_v7(), "ZA", "LE-SYNTHETIC-ZA").await,
            Err(CpAuthorityError::NotAuthorised)
        ));
    }

    #[tokio::test]
    async fn context_schema_validity_does_not_replace_binding_and_expiry_checks() {
        use axum::{Json, Router, routing::post};
        let organisation = Uuid::now_v7();
        for variant in ["valid", "wrong-market", "expired", "unknown-property", "missing-market-dependencies"] {
            let now = OffsetDateTime::now_utc();
            let mut response = json!({
                "context_id": Uuid::now_v7(), "tenant_id": "tn_synthetic123",
                "organisation_id": organisation, "country_code": "ZA",
                "market_id": "mkt_synthetic123", "currency_code": "ZAR",
                "resolved_at": now.format(&time::format_description::well_known::Rfc3339).unwrap(),
                "expires_at": (now + time::Duration::seconds(60)).format(&time::format_description::well_known::Rfc3339).unwrap(),
            });
            match variant {
                "wrong-market" => response["country_code"] = json!("UG"),
                "expired" => response["expires_at"] = json!((now - time::Duration::seconds(1)).format(&time::format_description::well_known::Rfc3339).unwrap()),
                "unknown-property" => response["authority_granted"] = json!(true),
                "missing-market-dependencies" => { response.as_object_mut().unwrap().remove("currency_code"); },
                _ => {}
            }
            let app = Router::new()
                .route("/token", post(|| async { Json(json!({"access_token":"synthetic-workload-token-only"})) }))
                .route("/v1/platform-context/resolve", post(move || { let response = response.clone(); async move { Json(response) } }));
            let (authority, server) = serve(app).await;
            let result = authority.issue_current_context("tn_synthetic123", organisation, "ZA", "LE-SYNTHETIC-ZA").await;
            assert_eq!(result.is_ok(), variant == "valid", "{variant}");
            server.abort();
        }
    }

    #[tokio::test]
    async fn duplicate_oversized_and_redirected_authority_are_refused() {
        use axum::{Json, Router, body::Body, http::Response, routing::post};
        for (status, raw, expected) in [
            (200, r#"{"context_id":"one","context_id":"two"}"#.to_owned(), CpAuthorityError::Unavailable),
            (200, "x".repeat(16_385), CpAuthorityError::Unavailable),
            (302, "{}".to_owned(), CpAuthorityError::NotAuthorised),
            (200, r#"{"outcome":"AUTHORIZED"}"#.to_owned(), CpAuthorityError::NotAuthorised),
        ] {
            let app = Router::new()
                .route("/token", post(|| async { Json(json!({"access_token":"synthetic-workload-token-only"})) }))
                .route("/internal/legal-actor/v1/assess", post(move || {
                    let raw = raw.clone();
                    async move { Response::builder().status(status).header("Location", "/must-never-follow").body(Body::from(raw)).unwrap() }
                }));
            let (authority, server) = serve(app).await;
            let request = json!({"context_id":"0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6b",
                "role":"PAYMENT_BENEFICIARY", "activity":"B2B_RECEIPT", "market":"ZA",
                "operation_reference":"synthetic/payment/123"});
            assert_eq!(authority.post("/internal/legal-actor/v1/assess", &request, "legal_request", "legal_response").await, Err(expected));
            server.abort();
        }
    }

    #[tokio::test]
    async fn chunked_limit_and_complete_body_deadline_are_enforced() {
        use std::io::{Read, Write};
        for slow in [false, true] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request).unwrap();
                stream.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
                if slow {
                    // Headers arrive promptly; body exceeds the TOTAL deadline.
                    std::thread::sleep(Duration::from_millis(750));
                }
                let raw = if slow { "{}".to_owned() } else { "x".repeat(16_385) };
                // The peer may cancel immediately after exceeding its bound.
                let _ = write!(stream, "{:X}\r\n{raw}\r\n0\r\n\r\n", raw.len());
            });
            let client = Client::builder().timeout(Duration::from_millis(500)).build().unwrap();
            let response = client.get(format!("http://{addr}/")).send().await.unwrap();
            assert_eq!(bounded_object(response, 16_384).await, Err(CpAuthorityError::Unavailable));
            server.join().unwrap();
        }
    }
    #[test]
    fn transport_requires_https_and_does_not_log_secrets() {
        let err = IamCpAuthority::new(
            "http://cp.example/",
            "https://iam.example/token",
            "baobab-payments-workload".into(),
            "not-a-real-secret".into(),
        );
        assert!(matches!(err, Err(CpAuthorityError::Configuration)));
        let ok = IamCpAuthority::new(
            "https://cp.example/",
            "https://iam.example/token",
            "baobab-payments-workload".into(),
            "not-a-real-secret".into(),
        )
        .unwrap();
        assert!(!format!("{ok:?}").contains("not-a-real-secret"));
    }

    #[tokio::test]
    async fn real_http_iam_token_and_cp_response_must_still_bind_exact_operation() {
        use axum::{Json, Router, routing::post};
        let app = Router::new()
            .route(
                "/token",
                post(|| async {
                    Json(json!({"access_token":"synthetic-signed-token-only","expires_in":30}))
                }),
            )
            .route(
                "/internal/legal-actor/v1/assess",
                post(|| async {
                    Json(json!({"context_id":"wrong-foreign-owner",
                    "operation_reference":"transaction-synthetic-1",
                    "provider_permissions_granted":false,
                    "legal_actor_resolution":{"outcome":"AUTHORIZED"}}))
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let a = IamCpAuthority::loopback(
            &format!("http://{addr}/"),
            &format!("http://{addr}/token"),
            "workload".into(),
            "synthetic".into(),
        )
        .unwrap();
        let operation = PaymentLegalOperation {
            context_id: "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6b",
            expected_legal_entity_id: "LE-SYNTHETIC-ZA",
            role: "PAYMENT_BENEFICIARY",
            activity: "B2B_RECEIPT",
            market: "ZA",
            capability: "payments.intent.create",
            operation_reference: "transaction-synthetic-1",
        };
        let context = CpRuntimeContext {
            context_id: Uuid::parse_str(operation.context_id).unwrap(),
            primary_organisation_id: Uuid::now_v7(),
            expected_legal_entity_id: operation.expected_legal_entity_id.into(),
        };
        assert_eq!(
            a.verified_mandate(&context, &operation).await,
            Err(CpAuthorityError::NotAuthorised)
        );
        server.abort();
    }
}
