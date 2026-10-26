//! Component enforcement tests. Explicit test ports and test-signed tokens
//! cannot qualify genuine IAM/CP/DEV-15 runtime acceptance.
mod support;
use baobab_payments::sandbox_authority::{
    HttpSandboxAuthority, SandboxAuthority, SandboxLease, SandboxRequest,
};
use baobab_payments::service::{Call, Refusal};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use support::*;
use time::OffsetDateTime;
struct RevocableAuthority(AtomicBool);
impl SandboxAuthority for RevocableAuthority {
    fn authorise<'a>(
        &'a self,
        _r: SandboxRequest<'a>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SandboxLease, Refusal>> + Send + 'a>,
    > {
        Box::pin(async {
            if !self.0.load(Ordering::SeqCst) {
                return Err(Refusal::new(
                    403,
                    "SANDBOX_CAPABILITY_DENIED",
                    "test port withdrew authority",
                ));
            }
            Ok(SandboxLease {
                expires_at: OffsetDateTime::now_utc() + time::Duration::seconds(20),
            })
        })
    }
}
#[tokio::test]
async fn missing_authority_denies_native_effects() {
    let h = harness_with_authority(None);
    let response = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&token()),
        Some(&new_key()),
        Some(intent_request("MANUAL")),
    )
    .await;
    assert_eq!(response.status, 503);
    assert!(h.service.events(TENANT).is_empty());
}
#[tokio::test]
async fn replay_and_reads_recheck_withdrawn_authority() {
    let authority = Arc::new(RevocableAuthority(AtomicBool::new(true)));
    let h = harness_with_authority(Some(authority.clone()));
    let bearer = token();
    let key = new_key();
    let body = intent_request("MANUAL");
    let created = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&bearer),
        Some(&key),
        Some(body.clone()),
    )
    .await;
    assert_eq!(created.status, 201);
    let before = h.service.events(TENANT).len();
    authority.0.store(false, Ordering::SeqCst);
    let replay = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&bearer),
        Some(&key),
        Some(body),
    )
    .await;
    assert_eq!(replay.status, 403);
    let id = created.body["payment_intent_id"].as_str().unwrap();
    let read = send(
        &h.app,
        "GET",
        &format!("/v1/tenants/{TENANT}/payment-intents/{id}"),
        Some(&bearer),
        None,
        None,
    )
    .await;
    assert_eq!(read.status, 403);
    assert_eq!(h.service.events(TENANT).len(), before);
}
#[test]
fn native_state_fence_rejects_expired_authority_before_effects() {
    let h = harness();
    let key = new_key();
    let call = Call {
        client_id: "baobab-subscriptions-workload",
        engine: "baobab-subscriptions",
        idempotency_key: &key,
        correlation_id: "0190f7a2-7a1e-7c3e-9d2b-3c4d5e6f7a8b",
        authority_expires_at: Some(OffsetDateTime::now_utc() - time::Duration::seconds(1)),
    };
    let outcome = h
        .service
        .create_intent(&call, intent_request("MANUAL").to_string().as_bytes());
    assert_eq!(outcome.unwrap_err().code, "SANDBOX_CAPABILITY_EXPIRED");
    assert!(h.service.events(TENANT).is_empty());
}
#[test]
fn partial_or_non_https_authority_configuration_is_refused() {
    assert!(HttpSandboxAuthority::from_env(|_| None).unwrap().is_none());
    assert!(
        HttpSandboxAuthority::from_env(
            |k| (k == "SANDBOX_CP_ORIGIN").then(|| "http://untrusted.invalid".into())
        )
        .is_err()
    );
}
