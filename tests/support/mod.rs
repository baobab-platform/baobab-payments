//! Shared test harness: a router with fixed signing keys, token minting and
//! request helpers.
#![allow(dead_code)]

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair as _};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use baobab_payments::auth::{Authenticator, Keys};
use baobab_payments::contracts;
use baobab_payments::http::{AppState, router};
use baobab_payments::provider::SandboxProvider;
use baobab_payments::service::PaymentService;
use http_body_util::BodyExt;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, encode};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tower::ServiceExt;

pub const ISSUER: &str = "https://iam.baobab.test/realms/baobab";
pub const TENANT: &str = "tn_01k9acmeltd";
pub const OTHER_TENANT: &str = "tn_01k4m7x9q2v6c8r3d5f1h0j4";

pub struct KeyPair {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

/// A fresh P-256 key pair (ES256).
fn generate() -> KeyPair {
    let pkcs8 =
        EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &SystemRandom::new())
            .expect("EC key");
    let pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref())
        .expect("EC key pair");
    KeyPair {
        encoding: EncodingKey::from_ec_der(pkcs8.as_ref()),
        decoding: DecodingKey::from_ec_der(pair.public_key().as_ref()),
    }
}

pub fn trusted() -> &'static KeyPair {
    static KEY: OnceLock<KeyPair> = OnceLock::new();
    KEY.get_or_init(generate)
}

pub fn untrusted() -> &'static KeyPair {
    static KEY: OnceLock<KeyPair> = OnceLock::new();
    KEY.get_or_init(generate)
}

pub struct Harness {
    pub app: Router,
    pub service: Arc<PaymentService>,
}

pub fn harness() -> Harness {
    let service = Arc::new(PaymentService::new(
        Arc::new(SandboxProvider),
        Arc::new(OffsetDateTime::now_utc),
    ));
    let auth = Arc::new(Authenticator::new(
        Keys::Static(HashMap::from([(
            "k1".to_string(),
            trusted().decoding.clone(),
        )])),
        ISSUER.into(),
        "baobab-payments".into(),
        BTreeSet::from([
            "baobab-subscriptions".to_string(),
            "baobab-trade".to_string(),
        ]),
    ));
    let app = router(AppState {
        service: service.clone(),
        auth,
        environment: "development",
    });
    Harness { app, service }
}

/// A workload token; `edit` adjusts the claims.
pub fn token_with(key: &KeyPair, edit: impl FnOnce(&mut Value)) -> String {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let mut claims = json!({
        "iss": ISSUER, "sub": "service-account-subscriptions", "aud": "baobab-payments",
        "iat": now, "exp": now + 300, "jti": uuid::Uuid::now_v7().to_string(),
        "actor_type": "workload", "azp": "baobab-subscriptions",
        "scope": "payment:execute payment:refund payment:read",
    });
    edit(&mut claims);
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some("k1".into());
    encode(&header, &claims, &key.encoding).expect("token")
}

pub fn token() -> String {
    token_with(trusted(), |_| {})
}

pub struct Reply {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    pub body: Value,
}

pub async fn send(
    app: &Router,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    key: Option<&str>,
    body: Option<Value>,
) -> Reply {
    send_raw(
        app,
        method,
        path,
        bearer,
        key,
        body.map(|b| b.to_string().into_bytes()),
        &[],
    )
    .await
}

pub async fn send_raw(
    app: &Router,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    key: Option<&str>,
    body: Option<Vec<u8>>,
    extra: &[(&str, &str)],
) -> Reply {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(bearer) = bearer {
        request = request.header("authorization", format!("Bearer {bearer}"));
    }
    if let Some(key) = key {
        request = request.header("idempotency-key", key);
    }
    for (name, value) in extra {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.unwrap_or_default())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("JSON body")
    };
    Reply {
        status,
        headers,
        body,
    }
}

pub fn new_key() -> String {
    format!("key-{}", uuid::Uuid::now_v7())
}

pub fn intent_request(capture: &str) -> Value {
    json!({
        "context": {
            "tenant_id": TENANT, "legal_entity_id": "LE-01K9ACMELTD", "market_id": "KE",
            "platform_account_id": "pacct_01k9acme", "currency": "KES",
            "source_engine": "baobab-subscriptions", "source_reference": "bsub_01k9acme",
            "correlation_id": "0192a1b0-7c3e-7a10-8000-000000000501"
        },
        "amount": { "currency": "KES", "amount_minor": 250000 },
        "capture_method": capture,
        "description": "Monthly platform subscription"
    })
}

pub fn assert_conforms(reference: &str, value: &Value) {
    let problems = contracts::problems(reference, value);
    assert!(problems.is_empty(), "{reference}: {problems:?}\n{value}");
}

pub fn assert_problem(reply: &Reply, status: u16, code: &str) {
    assert_eq!(reply.status.as_u16(), status, "{}", reply.body);
    assert_eq!(reply.headers["content-type"], "application/problem+json");
    assert_eq!(reply.body["code"], code, "{}", reply.body);
    assert_conforms(contracts::PROBLEM, &reply.body);
}
