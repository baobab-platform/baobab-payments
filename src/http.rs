//! The Baobab Payment API over HTTP. Every /v1 route requires a workload token
//! with the route's scope (Shared authorization/v1: payment:execute,
//! payment:refund, payment:read); every mutation requires an Idempotency-Key.
//! Errors are RFC 9457 problems (Shared errors/v1). Logs carry the route
//! template, never bodies, tokens or payment method references.

use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::{DefaultBodyLimit, MatchedPath, Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde_json::{Value, json};

use crate::auth::{Authenticator, Caller};
use crate::service::{Call, Outcome, PaymentService, Refusal};

const MAX_BODY: usize = 64 * 1024;

pub struct AppState {
    pub service: Arc<PaymentService>,
    pub auth: Arc<Authenticator>,
    pub environment: &'static str,
}

#[derive(Clone)]
struct Correlation(String);

#[derive(Clone)]
struct ClientId(String);

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/health/live",
            get(|| async { json_response(StatusCode::OK, &json!({ "status": "UP" }), false) }),
        )
        .route("/health/ready", get(ready))
        .route("/v1/payment-intents", post(create_intent))
        .route("/v1/payment-intents/{id}/confirm", post(confirm_intent))
        .route("/v1/payment-intents/{id}/cancel", post(cancel_intent))
        .route("/v1/payments/{id}/capture", post(capture))
        .route("/v1/payments/{id}/refunds", post(refund))
        .route(
            "/v1/tenants/{tenant_id}/payment-intents/{id}",
            get(get_intent),
        )
        .route("/v1/tenants/{tenant_id}/payments/{id}", get(get_payment))
        .fallback(|| async {
            problem_response(&Refusal::new(404, "ROUTE_NOT_FOUND", "no such route"), None)
        })
        .method_not_allowed_fallback(|| async {
            problem_response(
                &Refusal::new(405, "METHOD_NOT_ALLOWED", "method not allowed"),
                None,
            )
        })
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(middleware::from_fn(observe))
        .with_state(Arc::new(state))
}

fn uuid_like(value: &str) -> bool {
    uuid::Uuid::try_parse(value).is_ok() && value.len() == 36
}

/// Correlation IDs and the request log.
async fn observe(mut request: Request, next: Next) -> Response {
    let started = Instant::now();
    let correlation = request
        .headers()
        .get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| uuid_like(v))
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
    let method = request.method().to_string();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "unmatched".into());
    request
        .extensions_mut()
        .insert(Correlation(correlation.clone()));
    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&correlation) {
        response.headers_mut().insert("x-correlation-id", value);
    }
    let client = response.extensions().get::<ClientId>().map(|c| c.0.clone());
    tracing::info!(
        event = "http.request",
        method,
        route,
        status = response.status().as_u16(),
        duration_ms = started.elapsed().as_millis() as u64,
        correlation_id = correlation,
        client_id = client.as_deref().unwrap_or(""),
    );
    response
}

fn json_response(status: StatusCode, body: &Value, replayed: bool) -> Response {
    let mut response = (status, body.to_string()).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    if replayed {
        headers.insert("idempotent-replayed", HeaderValue::from_static("true"));
    }
    response
}

fn title(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Payload Too Large",
        422 => "Unprocessable Content",
        503 => "Service Unavailable",
        s if s >= 500 => "Internal Server Error",
        _ => "Request Refused",
    }
}

fn problem_response(refusal: &Refusal, correlation: Option<&Correlation>) -> Response {
    let correlation_id = correlation
        .map(|c| c.0.clone())
        .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
    let mut body = json!({
        "type": format!("urn:baobab-platform:problem:{}", refusal.code.to_ascii_lowercase().replace('_', "-")),
        "title": title(refusal.status),
        "status": refusal.status,
        "detail": refusal.detail.chars().take(2048).collect::<String>(),
        "code": refusal.code,
        "correlation_id": correlation_id,
        "retryable": refusal.retryable,
    });
    if !refusal.problems.is_empty() {
        body["errors"] = refusal
            .problems
            .iter()
            .take(50)
            .map(|p| json!({ "code": "SCHEMA_VIOLATION", "message": p.chars().take(500).collect::<String>() }))
            .collect();
    }
    let status = StatusCode::from_u16(refusal.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut response = json_response(status, &body, false);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    response
}

async fn authorize(state: &AppState, headers: &HeaderMap, scope: &str) -> Result<Caller, Refusal> {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    state
        .auth
        .authenticate(authorization, scope)
        .await
        .map_err(|e| Refusal::new(e.status, e.code, e.detail))
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, Refusal> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .filter(|k| {
            (16..=128).contains(&k.len())
                && k.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        })
        .map(String::from)
        .ok_or_else(|| {
            Refusal::new(
                400,
                "INVALID_IDEMPOTENCY_KEY",
                "an Idempotency-Key of 16 to 128 characters [A-Za-z0-9._:-] is required",
            )
        })
}

fn body_bytes(body: Result<Bytes, BytesRejection>) -> Result<Bytes, Refusal> {
    body.map_err(|rejection| {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            Refusal::new(413, "PAYLOAD_TOO_LARGE", "the request body is too large")
        } else {
            Refusal::new(400, "INVALID_REQUEST", "the request body could not be read")
        }
    })
}

fn with_client(mut response: Response, caller: Option<&Caller>) -> Response {
    if let Some(caller) = caller {
        response
            .extensions_mut()
            .insert(ClientId(caller.client_id.clone()));
    }
    response
}

/// Authorise, require an Idempotency-Key, read the body, then run `op`.
async fn mutate(
    state: &AppState,
    correlation: &Correlation,
    headers: &HeaderMap,
    scope: &str,
    body: Result<Bytes, BytesRejection>,
    op: impl FnOnce(&PaymentService, &Call<'_>, &[u8]) -> Result<Outcome, Refusal>,
) -> Response {
    let caller = match authorize(state, headers, scope).await {
        Ok(caller) => caller,
        Err(refusal) => return problem_response(&refusal, Some(correlation)),
    };
    let result = idempotency_key(headers).and_then(|key| {
        let bytes = body_bytes(body)?;
        let call = Call {
            client_id: &caller.client_id,
            idempotency_key: &key,
            correlation_id: &correlation.0,
        };
        op(&state.service, &call, &bytes)
    });
    let response = match result {
        Ok(outcome) => json_response(
            StatusCode::from_u16(outcome.status).unwrap_or(StatusCode::OK),
            &outcome.body,
            outcome.replayed,
        ),
        Err(refusal) => problem_response(&refusal, Some(correlation)),
    };
    with_client(response, Some(&caller))
}

async fn read(
    state: &AppState,
    correlation: &Correlation,
    headers: &HeaderMap,
    op: impl FnOnce(&PaymentService) -> Result<Value, Refusal>,
) -> Response {
    let caller = match authorize(state, headers, "payment:read").await {
        Ok(caller) => caller,
        Err(refusal) => return problem_response(&refusal, Some(correlation)),
    };
    let response = match op(&state.service) {
        Ok(body) => json_response(StatusCode::OK, &body, false),
        Err(refusal) => problem_response(&refusal, Some(correlation)),
    };
    with_client(response, Some(&caller))
}

async fn create_intent(
    State(s): State<Arc<AppState>>,
    axum::Extension(c): axum::Extension<Correlation>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    mutate(
        &s,
        &c,
        &headers,
        "payment:execute",
        body,
        |svc, call, raw| svc.create_intent(call, raw),
    )
    .await
}

async fn confirm_intent(
    State(s): State<Arc<AppState>>,
    axum::Extension(c): axum::Extension<Correlation>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    mutate(
        &s,
        &c,
        &headers,
        "payment:execute",
        body,
        |svc, call, raw| svc.confirm_intent(call, &id, raw),
    )
    .await
}

async fn cancel_intent(
    State(s): State<Arc<AppState>>,
    axum::Extension(c): axum::Extension<Correlation>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    mutate(
        &s,
        &c,
        &headers,
        "payment:execute",
        body,
        |svc, call, raw| svc.cancel_intent(call, &id, raw),
    )
    .await
}

async fn capture(
    State(s): State<Arc<AppState>>,
    axum::Extension(c): axum::Extension<Correlation>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    mutate(
        &s,
        &c,
        &headers,
        "payment:execute",
        body,
        |svc, call, raw| svc.capture(call, &id, raw),
    )
    .await
}

async fn refund(
    State(s): State<Arc<AppState>>,
    axum::Extension(c): axum::Extension<Correlation>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    mutate(
        &s,
        &c,
        &headers,
        "payment:refund",
        body,
        |svc, call, raw| svc.refund(call, &id, raw),
    )
    .await
}

async fn get_intent(
    State(s): State<Arc<AppState>>,
    axum::Extension(c): axum::Extension<Correlation>,
    headers: HeaderMap,
    Path((tenant, id)): Path<(String, String)>,
) -> Response {
    read(&s, &c, &headers, |svc| svc.intent(&tenant, &id)).await
}

async fn get_payment(
    State(s): State<Arc<AppState>>,
    axum::Extension(c): axum::Extension<Correlation>,
    headers: HeaderMap,
    Path((tenant, id)): Path<(String, String)>,
) -> Response {
    read(&s, &c, &headers, |svc| svc.payment(&tenant, &id)).await
}

async fn ready(State(s): State<Arc<AppState>>) -> Response {
    let provider = s.service.provider();
    let up = provider.healthy();
    let body = json!({
        "status": if up { "READY" } else { "NOT_READY" },
        "checks": { "store": "UP", "payment_provider": if up { "UP" } else { "DOWN" } },
        "payment_provider": { "kind": provider.kind(), "simulated": provider.simulated() },
        // Serving says nothing about money: no real payment path exists while
        // the provider is simulated.
        "real_payments": if provider.simulated() { "NOT_CONFIGURED" } else { "CONFIGURED" },
        "store": "in-memory",
        "environment": s.environment,
    });
    json_response(
        if up {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        &body,
        false,
    )
}
