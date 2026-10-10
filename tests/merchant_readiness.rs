//! LA-05C3: a sandbox must NEVER assert merchant certification. Only an
//! authenticated Trade workload with separately granted Payments scope can
//! reach the assessment decision; all requests still fail closed.
mod support;
use serde_json::json;
use support::*;

const PATH: &str = "/internal/merchant-readiness/v1/assess";

fn request() -> serde_json::Value {
    json!({
        "tenant_id": TENANT,
        "organisation_id": "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6a",
        "responsible_legal_entity_id": "LE-SYNTHETIC-ZA",
        "market": "ZA",
        "currency_code": "ZAR",
        "capability": "commerce.order.create",
        "operation_reference": "cart/cart-synthetic/complete",
        "mandate_id": "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c"
    })
}

fn trade_token(scope: &str) -> String {
    token_with(trusted(), |claims| {
        claims["azp"] = json!("baobab-trade-workload");
        claims["sub"] = json!("service-account-trade");
        claims["scope"] = json!(scope);
    })
}

#[tokio::test]
async fn unauthenticated_and_unscoped_clients_cannot_request_merchant_readiness() {
    let h = harness();
    let missing = send(&h.app, "POST", PATH, None, None, Some(request())).await;
    assert_problem(&missing, 401, "AUTH_TOKEN_REQUIRED");
    let unscoped = trade_token("payment:read");
    let denied = send(&h.app, "POST", PATH, Some(&unscoped), None, Some(request())).await;
    assert_problem(&denied, 403, "AUTHORIZATION_DENIED");
}

#[tokio::test]
async fn unrelated_workload_with_scope_cannot_assert_trade_merchant_readiness() {
    let h = harness();
    let unrelated = token_with(trusted(), |claims| {
        claims["scope"] = json!("merchant-readiness:assess");
    });
    let denied = send(
        &h.app,
        "POST",
        PATH,
        Some(&unrelated),
        None,
        Some(request()),
    )
    .await;
    assert_problem(&denied, 403, "MERCHANT_READINESS_FORBIDDEN");
}

#[tokio::test]
async fn scoped_trade_token_never_gets_ready_without_certified_provider() {
    let h = harness();
    let authorized = trade_token("merchant-readiness:assess");
    let fail_closed = send(
        &h.app,
        "POST",
        PATH,
        Some(&authorized),
        None,
        Some(request()),
    )
    .await;
    assert_problem(&fail_closed, 503, "MERCHANT_CERTIFICATION_NOT_CONFIGURED");
    assert_ne!(fail_closed.body["outcome"], "READY");
    assert_eq!(fail_closed.headers["cache-control"], "no-store");
    let replay = send(
        &h.app,
        "POST",
        PATH,
        Some(&authorized),
        None,
        Some(request()),
    )
    .await;
    assert_problem(&replay, 503, "MERCHANT_CERTIFICATION_NOT_CONFIGURED");

    let missing_fields = send(
        &h.app,
        "POST",
        PATH,
        Some(&authorized),
        None,
        Some(json!({"tenant_id": TENANT})),
    )
    .await;
    assert_problem(&missing_fields, 400, "INVALID_MERCHANT_READINESS_REQUEST");
    assert!(h.service.events(TENANT).is_empty());
}

#[tokio::test]
async fn readiness_request_must_match_the_shared_contract_strictly() {
    let h = harness();
    let authorized = trade_token("merchant-readiness:assess");
    let mutations: Vec<(&str, serde_json::Value)> = vec![
        ("outcome", json!("READY")),
        ("provider", json!("user-selected")),
        ("tenant_id", json!("TN_UPPER")),
        ("tenant_id", json!("tn_ab")),
        ("market", json!("za")),
        ("market", json!("ZAF")),
        ("currency_code", json!("zar")),
        ("capability", json!("ab")),
        ("operation_reference", json!("x".repeat(161))),
        ("mandate_id", json!("not-a-uuid")),
    ];
    for (key, value) in mutations {
        let mut body = request();
        body[key] = value;
        let res = send(&h.app, "POST", PATH, Some(&authorized), None, Some(body)).await;
        assert_problem(&res, 400, "INVALID_MERCHANT_READINESS_REQUEST");
    }
}
