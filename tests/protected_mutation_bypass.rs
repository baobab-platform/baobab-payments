//! ADR-BCP-026/027 Package 4: no direct sandbox/legacy protected payment
//! path may bypass real CP legal actor, documentary veto or PSP certification.
mod support;
use serde_json::json;
use support::*;

#[tokio::test]
async fn staging_and_production_never_execute_any_legacy_money_mutation_or_replay() {
    // P0-04B: a native backend present in a protected runtime (which
    // configuration refuses at startup) still opens no route.
    let cases = ["staging", "production", "unknown"]
        .into_iter()
        .flat_map(|e| [(e, None), (e, Some(unreachable_native_backend()))]);
    for (environment, native) in cases {
        let h = harness_with(environment, native);
        let bearer = token();
        let operations = [
            ("POST", "/v1/payment-intents", "payment:execute"),
            (
                "POST",
                "/v1/payment-intents/pi_synthetic/confirm",
                "payment:execute",
            ),
            (
                "POST",
                "/v1/payment-intents/pi_synthetic/cancel",
                "payment:execute",
            ),
            (
                "POST",
                "/v1/payments/pay_synthetic/capture",
                "payment:execute",
            ),
            (
                "POST",
                "/v1/payments/pay_synthetic/refunds",
                "payment:refund",
            ),
        ];
        for (verb, route, _scope) in operations {
            // Use a scope-bearing workload token and a valid idempotency key.
            // A *replayed* key must still be refused in protected runtimes.
            let key = new_key();
            for _ in 0..2 {
                let result = send(
                    &h.app,
                    verb,
                    route,
                    Some(&bearer),
                    Some(&key),
                    Some(json!({"tenant_id":TENANT,"context":{
                        "tenant_id": TENANT, "legal_entity_id": "LE-SYNTHETIC",
                        "source_engine": "baobab-subscriptions"
                    }})),
                )
                .await;
                assert_problem(&result, 503, "PAYMENT_EXECUTION_NOT_CERTIFIED");
                assert_ne!(
                    result
                        .headers
                        .get("idempotent-replayed")
                        .map(|v| v.as_bytes()),
                    Some(b"true".as_slice())
                );
            }
        }
        assert!(
            h.service.events(TENANT).is_empty(),
            "protected bypass published simulated financial events"
        );
    }
}

#[tokio::test]
async fn protected_denial_does_not_skip_workload_authentication() {
    let h = harness_for_environment("staging");
    let denied = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        None,
        Some(&new_key()),
        Some(intent_request("AUTOMATIC")),
    )
    .await;
    assert_problem(&denied, 401, "AUTH_TOKEN_REQUIRED");
    let unscoped = token_with(trusted(), |claims| claims["scope"] = json!("payment:read"));
    let denied = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&unscoped),
        Some(&new_key()),
        Some(intent_request("AUTOMATIC")),
    )
    .await;
    assert_problem(&denied, 403, "AUTHORIZATION_DENIED");
    assert!(h.service.events(TENANT).is_empty());
}
