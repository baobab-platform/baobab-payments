//! The Baobab Payment API end to end over the router: the sandbox lifecycle,
//! honest simulation markers, idempotency, context rules, tenant isolation
//! and contract conformance of every resource and event.

mod support;

use baobab_payments::contracts;
use serde_json::{Value, json};
use support::*;

fn events_conform(h: &Harness, tenant: &str) -> Vec<String> {
    let mut types = Vec::new();
    for event in h.service.events(tenant) {
        assert_conforms(contracts::ENVELOPE, &event.envelope);
        let schema = event.envelope["dataschema"].as_str().unwrap();
        let definition = schema.rsplit('/').next().unwrap();
        assert_conforms(
            &contracts::def(contracts::EVENTS, definition),
            &event.envelope["data"],
        );
        assert_eq!(
            event.envelope["data"]["simulated"], true,
            "every sandbox event is marked simulated"
        );
        let text = event.envelope.to_string();
        assert!(
            !text.contains("pm_"),
            "no payment method reference reaches an event: {text}"
        );
        types.push(event.event_type);
    }
    types
}

#[tokio::test]
async fn automatic_capture_then_refund_is_simulated_end_to_end() {
    let h = harness();
    let bearer = token();
    let created = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&bearer),
        Some(&new_key()),
        Some(intent_request("AUTOMATIC")),
    )
    .await;
    assert_eq!(created.status, 201, "{}", created.body);
    assert_conforms(
        &contracts::def(contracts::PAYMENT, "PaymentIntent"),
        &created.body,
    );
    assert_eq!(created.body["status"], "REQUIRES_PAYMENT_METHOD");
    assert_eq!(
        created.body["provider"],
        json!({ "kind": "SANDBOX", "simulated": true })
    );
    let intent = created.body["payment_intent_id"]
        .as_str()
        .unwrap()
        .to_string();

    let confirmed = send(
        &h.app,
        "POST",
        &format!("/v1/payment-intents/{intent}/confirm"),
        Some(&bearer),
        Some(&new_key()),
        Some(json!({ "tenant_id": TENANT, "payment_method_reference": "pm_tok_opaque_123" })),
    )
    .await;
    assert_eq!(confirmed.status, 200, "{}", confirmed.body);
    assert_conforms(
        &contracts::def(contracts::PAYMENT, "PaymentIntent"),
        &confirmed.body,
    );
    assert_eq!(confirmed.body["status"], "SUCCEEDED");
    assert!(
        !confirmed.body.to_string().contains("pm_tok_opaque_123"),
        "the payment method reference is never returned"
    );
    let payment_id = confirmed.body["payment_id"].as_str().unwrap().to_string();

    let payment = send(
        &h.app,
        "GET",
        &format!("/v1/tenants/{TENANT}/payments/{payment_id}"),
        Some(&bearer),
        None,
        None,
    )
    .await;
    assert_eq!(payment.status, 200);
    assert_conforms(
        &contracts::def(contracts::PAYMENT, "Payment"),
        &payment.body,
    );
    assert_eq!(payment.body["status"], "CAPTURED");
    assert_eq!(payment.body["provider"]["simulated"], true);

    let refund = send(
        &h.app,
        "POST",
        &format!("/v1/payments/{payment_id}/refunds"),
        Some(&bearer),
        Some(&new_key()),
        Some(
            json!({ "tenant_id": TENANT, "amount_minor": 100000, "reason": "Pro-rated downgrade" }),
        ),
    )
    .await;
    assert_eq!(refund.status, 201, "{}", refund.body);
    assert_conforms(&contracts::def(contracts::PAYMENT, "Refund"), &refund.body);
    let over = send(
        &h.app,
        "POST",
        &format!("/v1/payments/{payment_id}/refunds"),
        Some(&bearer),
        Some(&new_key()),
        Some(json!({ "tenant_id": TENANT, "amount_minor": 200000, "reason": "Too much" })),
    )
    .await;
    assert_problem(&over, 422, "REFUND_EXCEEDS_CAPTURED");
    let payment = send(
        &h.app,
        "GET",
        &format!("/v1/tenants/{TENANT}/payments/{payment_id}"),
        Some(&bearer),
        None,
        None,
    )
    .await;
    assert_eq!(payment.body["status"], "PARTIALLY_REFUNDED");
    assert_eq!(payment.body["refunded_amount_minor"], 100000);

    assert_eq!(
        events_conform(&h, TENANT),
        vec![
            "com.baobab-platform.payments.payment.created.v1",
            "com.baobab-platform.payments.payment.authorized.v1",
            "com.baobab-platform.payments.payment.captured.v1",
            "com.baobab-platform.payments.payment.refunded.v1",
        ]
    );
}

#[tokio::test]
async fn manual_capture_decline_and_cancel() {
    let h = harness();
    let bearer = token();
    let intent = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&bearer),
        Some(&new_key()),
        Some(intent_request("MANUAL")),
    )
    .await;
    let id = intent.body["payment_intent_id"]
        .as_str()
        .unwrap()
        .to_string();
    let authorised = send(
        &h.app,
        "POST",
        &format!("/v1/payment-intents/{id}/confirm"),
        Some(&bearer),
        Some(&new_key()),
        Some(json!({ "tenant_id": TENANT, "payment_method_reference": "pm_tok_manual" })),
    )
    .await;
    assert_eq!(authorised.body["status"], "REQUIRES_CAPTURE");
    let payment_id = authorised.body["payment_id"].as_str().unwrap().to_string();
    let too_much = send(
        &h.app,
        "POST",
        &format!("/v1/payments/{payment_id}/capture"),
        Some(&bearer),
        Some(&new_key()),
        Some(json!({ "tenant_id": TENANT, "amount_minor": 250001 })),
    )
    .await;
    assert_problem(&too_much, 422, "CAPTURE_EXCEEDS_AUTHORIZED");
    let captured = send(
        &h.app,
        "POST",
        &format!("/v1/payments/{payment_id}/capture"),
        Some(&bearer),
        Some(&new_key()),
        Some(json!({ "tenant_id": TENANT, "amount_minor": 200000 })),
    )
    .await;
    assert_eq!(captured.status, 200, "{}", captured.body);
    assert_eq!(captured.body["captured_amount_minor"], 200000);
    let late_cancel = send(
        &h.app,
        "POST",
        &format!("/v1/payment-intents/{id}/cancel"),
        Some(&bearer),
        Some(&new_key()),
        Some(json!({ "tenant_id": TENANT, "reason": "Customer changed plan" })),
    )
    .await;
    assert_problem(&late_cancel, 409, "INVALID_PAYMENT_STATE");

    let declined = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&bearer),
        Some(&new_key()),
        Some(intent_request("AUTOMATIC")),
    )
    .await;
    let declined_id = declined.body["payment_intent_id"]
        .as_str()
        .unwrap()
        .to_string();
    let failed = send(
        &h.app,
        "POST",
        &format!("/v1/payment-intents/{declined_id}/confirm"),
        Some(&bearer),
        Some(&new_key()),
        Some(json!({ "tenant_id": TENANT, "payment_method_reference": "pm_sandbox_decline" })),
    )
    .await;
    assert_eq!(failed.body["status"], "FAILED");

    let open = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&bearer),
        Some(&new_key()),
        Some(intent_request("MANUAL")),
    )
    .await;
    let open_id = open.body["payment_intent_id"].as_str().unwrap().to_string();
    let cancelled = send(
        &h.app,
        "POST",
        &format!("/v1/payment-intents/{open_id}/cancel"),
        Some(&bearer),
        Some(&new_key()),
        Some(json!({ "tenant_id": TENANT, "reason": "Superseded" })),
    )
    .await;
    assert_eq!(cancelled.body["status"], "CANCELLED");

    let types = events_conform(&h, TENANT);
    assert!(types.contains(&"com.baobab-platform.payments.payment.failed.v1".to_string()));
    assert!(types.contains(&"com.baobab-platform.payments.payment.cancelled.v1".to_string()));
}

#[tokio::test]
async fn money_moving_requests_are_idempotent() {
    let h = harness();
    let bearer = token();
    let key = new_key();
    let first = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&bearer),
        Some(&key),
        Some(intent_request("AUTOMATIC")),
    )
    .await;
    let replay = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&bearer),
        Some(&key),
        Some(intent_request("AUTOMATIC")),
    )
    .await;
    assert_eq!(replay.status, 201);
    assert_eq!(replay.headers["idempotent-replayed"], "true");
    assert_eq!(first.body, replay.body);
    assert_eq!(
        h.service.events(TENANT).len(),
        1,
        "a replay moves nothing and publishes nothing"
    );

    let reused = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&bearer),
        Some(&key),
        Some(intent_request("MANUAL")),
    )
    .await;
    assert_problem(&reused, 409, "IDEMPOTENCY_KEY_REUSED");
    for key in [None, Some("short")] {
        let missing = send(
            &h.app,
            "POST",
            "/v1/payment-intents",
            Some(&bearer),
            key,
            Some(intent_request("AUTOMATIC")),
        )
        .await;
        assert_problem(&missing, 400, "INVALID_IDEMPOTENCY_KEY");
    }

    let id = first.body["payment_intent_id"]
        .as_str()
        .unwrap()
        .to_string();
    let confirm_key = new_key();
    let confirm = json!({ "tenant_id": TENANT, "payment_method_reference": "pm_tok_once" });
    let a = send(
        &h.app,
        "POST",
        &format!("/v1/payment-intents/{id}/confirm"),
        Some(&bearer),
        Some(&confirm_key),
        Some(confirm.clone()),
    )
    .await;
    let b = send(
        &h.app,
        "POST",
        &format!("/v1/payment-intents/{id}/confirm"),
        Some(&bearer),
        Some(&confirm_key),
        Some(confirm),
    )
    .await;
    assert_eq!(
        a.body, b.body,
        "a retried confirmation never authorises twice"
    );
    assert_eq!(h.service.events(TENANT).len(), 3);
}

#[tokio::test]
async fn the_payment_context_must_be_coherent_and_the_callers_own() {
    let h = harness();
    let bearer = token();
    let mut mismatch = intent_request("AUTOMATIC");
    mismatch["amount"]["currency"] = json!("USD");
    assert_problem(
        &send(
            &h.app,
            "POST",
            "/v1/payment-intents",
            Some(&bearer),
            Some(&new_key()),
            Some(mismatch),
        )
        .await,
        422,
        "CURRENCY_MISMATCH",
    );

    let mut foreign = intent_request("AUTOMATIC");
    foreign["context"]["source_engine"] = json!("baobab-trade");
    assert_problem(
        &send(
            &h.app,
            "POST",
            "/v1/payment-intents",
            Some(&bearer),
            Some(&new_key()),
            Some(foreign),
        )
        .await,
        403,
        "SOURCE_ENGINE_MISMATCH",
    );

    // Trade's own workload identity cannot move money for a Subscriptions
    // obligation either: the engine comes from configuration, not the body.
    let trade = token_with(trusted(), |c| c["azp"] = json!("baobab-trade-workload"));
    assert_problem(
        &send(
            &h.app,
            "POST",
            "/v1/payment-intents",
            Some(&trade),
            Some(&new_key()),
            Some(intent_request("AUTOMATIC")),
        )
        .await,
        403,
        "SOURCE_ENGINE_MISMATCH",
    );

    let mut card = intent_request("AUTOMATIC");
    card["card_number"] = json!("4111111111111111");
    assert_problem(
        &send(
            &h.app,
            "POST",
            "/v1/payment-intents",
            Some(&bearer),
            Some(&new_key()),
            Some(card),
        )
        .await,
        400,
        "VALIDATION_FAILED",
    );

    let mut no_context = intent_request("AUTOMATIC");
    no_context["context"]
        .as_object_mut()
        .unwrap()
        .remove("legal_entity_id");
    assert_problem(
        &send(
            &h.app,
            "POST",
            "/v1/payment-intents",
            Some(&bearer),
            Some(&new_key()),
            Some(no_context),
        )
        .await,
        400,
        "VALIDATION_FAILED",
    );
}

#[tokio::test]
async fn tenants_are_isolated() {
    let h = harness();
    let bearer = token();
    let created = send(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&bearer),
        Some(&new_key()),
        Some(intent_request("MANUAL")),
    )
    .await;
    let id = created.body["payment_intent_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_problem(
        &send(
            &h.app,
            "GET",
            &format!("/v1/tenants/{OTHER_TENANT}/payment-intents/{id}"),
            Some(&bearer),
            None,
            None,
        )
        .await,
        404,
        "PAYMENT_RESOURCE_NOT_FOUND",
    );
    assert_problem(
        &send(
            &h.app,
            "POST",
            &format!("/v1/payment-intents/{id}/confirm"),
            Some(&bearer),
            Some(&new_key()),
            Some(json!({ "tenant_id": OTHER_TENANT, "payment_method_reference": "pm_x" })),
        )
        .await,
        404,
        "PAYMENT_RESOURCE_NOT_FOUND",
    );
    assert_problem(
        &send(
            &h.app,
            "POST",
            &format!("/v1/payment-intents/{id}/cancel"),
            Some(&bearer),
            Some(&new_key()),
            Some(json!({ "tenant_id": OTHER_TENANT, "reason": "x" })),
        )
        .await,
        404,
        "PAYMENT_RESOURCE_NOT_FOUND",
    );
    let own = send(
        &h.app,
        "GET",
        &format!("/v1/tenants/{TENANT}/payment-intents/{id}"),
        Some(&bearer),
        None,
        None,
    )
    .await;
    assert_eq!(own.status, 200);
    assert!(h.service.events(OTHER_TENANT).is_empty());
}

#[tokio::test]
async fn only_workload_identity_is_accepted() {
    let h = harness();
    let body = || Some(intent_request("AUTOMATIC"));
    let path = "/v1/payment-intents";
    assert_problem(
        &send(&h.app, "POST", path, None, Some(&new_key()), body()).await,
        401,
        "AUTH_TOKEN_REQUIRED",
    );
    assert_problem(
        &send(
            &h.app,
            "POST",
            path,
            Some("static-shared-secret"),
            Some(&new_key()),
            body(),
        )
        .await,
        401,
        "AUTH_TOKEN_INVALID",
    );
    let cases: Vec<(String, u16, &str)> = vec![
        (token_with(untrusted(), |_| {}), 401, "AUTH_TOKEN_INVALID"),
        (
            token_with(trusted(), |c| {
                c["iss"] = json!("https://iam.other.test/realms/x")
            }),
            401,
            "AUTH_TOKEN_INVALID",
        ),
        (
            token_with(trusted(), |c| c["aud"] = json!("baobab-subscriptions")),
            401,
            "AUTH_TOKEN_INVALID",
        ),
        (
            token_with(trusted(), |c| {
                c["exp"] = json!(c["iat"].as_i64().unwrap() - 120)
            }),
            401,
            "AUTH_TOKEN_INVALID",
        ),
        (
            token_with(trusted(), |c| {
                c["exp"] = json!(c["iat"].as_i64().unwrap() + 3600)
            }),
            401,
            "AUTH_TOKEN_INVALID",
        ),
        (
            token_with(trusted(), |c| c["actor_type"] = json!("human")),
            403,
            "AUTHORIZATION_DENIED",
        ),
        (
            token_with(trusted(), |c| c["azp"] = json!("baobab-client-portal")),
            403,
            "AUTHORIZATION_DENIED",
        ),
        // The engine's name is not its workload client: only the registered
        // workload identity is allowed.
        (
            token_with(trusted(), |c| c["azp"] = json!("baobab-subscriptions")),
            403,
            "AUTHORIZATION_DENIED",
        ),
        (
            token_with(trusted(), |c| c["scope"] = json!("payment:read")),
            403,
            "AUTHORIZATION_DENIED",
        ),
    ];
    for (bearer, status, code) in cases {
        assert_problem(
            &send(
                &h.app,
                "POST",
                path,
                Some(&bearer),
                Some(&new_key()),
                body(),
            )
            .await,
            status,
            code,
        );
    }
    // Refunds need their own scope.
    let execute_only = token_with(trusted(), |c| {
        c["scope"] = json!("payment:execute payment:read")
    });
    assert_problem(
        &send(
            &h.app,
            "POST",
            "/v1/payments/pay_x/refunds",
            Some(&execute_only),
            Some(&new_key()),
            Some(json!({ "tenant_id": TENANT, "amount_minor": 1, "reason": "x" })),
        )
        .await,
        403,
        "AUTHORIZATION_DENIED",
    );
}

#[tokio::test]
async fn health_problems_and_correlation() {
    let h = harness();
    let live = send(&h.app, "GET", "/health/live", None, None, None).await;
    assert_eq!(live.status, 200);
    let ready = send(&h.app, "GET", "/health/ready", None, None, None).await;
    assert_eq!(ready.status, 200);
    assert_eq!(
        ready.body["real_payments"], "NOT_CONFIGURED",
        "a sandbox never claims real payments"
    );
    assert_eq!(ready.body["payment_provider"]["simulated"], true);

    let correlation = "0192a1b0-7c3e-7a10-8000-00000000abcd";
    let invalid = send_raw(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&token()),
        Some(&new_key()),
        Some(b"{}".to_vec()),
        &[("x-correlation-id", correlation)],
    )
    .await;
    assert_problem(&invalid, 400, "VALIDATION_FAILED");
    assert_eq!(invalid.headers["x-correlation-id"], correlation);
    assert_eq!(invalid.body["correlation_id"], correlation);
    assert_problem(
        &send(&h.app, "GET", "/v1/nothing", Some(&token()), None, None).await,
        404,
        "ROUTE_NOT_FOUND",
    );
    assert_problem(
        &send(
            &h.app,
            "DELETE",
            "/v1/payment-intents",
            Some(&token()),
            None,
            None,
        )
        .await,
        405,
        "METHOD_NOT_ALLOWED",
    );
    let large = send_raw(
        &h.app,
        "POST",
        "/v1/payment-intents",
        Some(&token()),
        Some(&new_key()),
        Some(vec![b' '; 70 * 1024]),
        &[],
    )
    .await;
    assert_problem(&large, 413, "PAYLOAD_TOO_LARGE");
    let _: Value = large.body;
}
