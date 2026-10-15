//! Gates A–C for the money-movement PEP: a loopback CP and a loopback provider
//! that count every call, against the real PostgreSQL 17 journal when
//! LA05_NATIVE_PG_TEST_URL (runtime role) and LA05_NATIVE_PG_ADMIN_URL
//! (operator, for synthetic provider activation) are set. CI sets both.

use super::*;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use time::format_description::well_known::Rfc3339;

const LE: &str = "LE-SYNTHETIC-PAY";
const ORG: &str = "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CpMode {
    Authorize,
    Deny,
    Unavailable,
    WrongFingerprint,
}

#[derive(Default)]
struct CpState {
    calls: Mutex<Vec<(String, u64)>>,
    modes: Mutex<Vec<CpMode>>,
}

async fn cp_resolve(
    State(s): State<Arc<CpState>>,
    Json(request): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let attempt = request["attempt"].as_u64().unwrap();
    s.calls.lock().unwrap().push((
        request["operation_class"].as_str().unwrap().to_owned(),
        attempt,
    ));
    let mode = {
        let mut modes = s.modes.lock().unwrap();
        if modes.len() > 1 {
            modes.remove(0)
        } else {
            modes.first().copied().unwrap_or(CpMode::Authorize)
        }
    };
    if mode == CpMode::Unavailable {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({})));
    }
    let class = OperationClass::parse(request["operation_class"].as_str().unwrap()).unwrap();
    let now = OffsetDateTime::now_utc();
    let mut decision = json!({
        "decision_id": Uuid::now_v7().to_string(),
        "context_id": request["context_id"], "engine_instance_id": request["engine_instance_id"],
        "operation_class": request["operation_class"], "operation_reference": request["operation_reference"],
        "attempt": request["attempt"], "instruction_fingerprint": request["instruction_fingerprint"],
        "outcome": "AUTHORIZED", "tenant_id": request["instruction"]["tenant_id"],
        "operating_organisation_id": request["instruction"]["operating_organisation_id"],
        "market": request["instruction"]["market"], "legal_actor_role": "PAYMENT_BENEFICIARY",
        "activity": class.activity(), "capability_key": class.capability(),
        "mandate_id": "0199a1b2-0000-7000-8000-0000000000dd", "responsible_legal_entity_id": LE,
        "evidence_references": ["synthetic:mandate-evidence/pay"],
        "mandate_effective_from": "2026-10-01T00:00:00Z",
        "evaluated_at": now.format(&Rfc3339).unwrap(),
        "expires_at": (now + time::Duration::seconds(25)).format(&Rfc3339).unwrap(),
        "policy_reference": "ADR-SHARED-035/payments-money-movement-authority:v1",
        "provider_permissions_granted": false
    });
    match mode {
        CpMode::Deny => {
            let d = decision.as_object_mut().unwrap();
            for key in [
                "mandate_id",
                "responsible_legal_entity_id",
                "evidence_references",
                "mandate_effective_from",
                "expires_at",
                "capability_key",
            ] {
                d.remove(key);
            }
            d.insert("outcome".into(), json!("REVOKED_OR_EXPIRED"));
        }
        CpMode::WrongFingerprint => decision["instruction_fingerprint"] = json!("0".repeat(64)),
        _ => {}
    }
    (StatusCode::OK, Json(decision))
}

/// A provider that records every instruction and remembers what moved.
#[derive(Default)]
struct PspState {
    posts: AtomicUsize,
    gets: AtomicUsize,
    captured: AtomicI64,
    authorized: AtomicBool,
    cancelled: AtomicBool,
    refunds: Mutex<Vec<String>>,
    /// Apply the effect but lose the response (a timeout after the money moved).
    lose_next_response: AtomicBool,
    /// Lose the request entirely (it never landed).
    drop_next_request: AtomicBool,
    /// Capture one minor unit less than asked, yet still answer success.
    short_capture: AtomicBool,
}

fn psp_fault(s: &PspState) -> Option<StatusCode> {
    s.posts.fetch_add(1, Ordering::SeqCst);
    if s.drop_next_request.swap(false, Ordering::SeqCst) {
        return Some(StatusCode::BAD_GATEWAY);
    }
    None
}

fn psp_reply(s: &PspState, body: Value) -> (StatusCode, Json<Value>) {
    if s.lose_next_response.swap(false, Ordering::SeqCst) {
        return (StatusCode::GATEWAY_TIMEOUT, Json(json!({})));
    }
    (StatusCode::OK, Json(body))
}

fn payment_status(s: &PspState) -> &'static str {
    if s.cancelled.load(Ordering::SeqCst) {
        "cancelled"
    } else if s.captured.load(Ordering::SeqCst) > 0 {
        "partially_captured"
    } else if s.authorized.load(Ordering::SeqCst) {
        "requires_capture"
    } else {
        "requires_confirmation"
    }
}

fn provider(state: Arc<PspState>) -> Router {
    Router::new()
        .route("/payments/{id}/confirm", post(|State(s): State<Arc<PspState>>, Path(id): Path<String>| async move {
            if let Some(code) = psp_fault(&s) { return (code, Json(json!({}))); }
            s.authorized.store(true, Ordering::SeqCst);
            psp_reply(&s, json!({"payment_id": id, "status": "requires_capture"}))
        }))
        .route("/payments/{id}/capture", post(|State(s): State<Arc<PspState>>, Path(id): Path<String>, Json(b): Json<Value>| async move {
            if let Some(code) = psp_fault(&s) { return (code, Json(json!({}))); }
            let short = i64::from(s.short_capture.swap(false, Ordering::SeqCst));
            s.captured.fetch_add(b["amount_to_capture"].as_i64().unwrap() - short, Ordering::SeqCst);
            psp_reply(&s, json!({"payment_id": id, "status": "partially_captured"}))
        }))
        .route("/payments/{id}/cancel", post(|State(s): State<Arc<PspState>>, Path(id): Path<String>| async move {
            if let Some(code) = psp_fault(&s) { return (code, Json(json!({}))); }
            s.cancelled.store(true, Ordering::SeqCst);
            psp_reply(&s, json!({"payment_id": id, "status": "cancelled"}))
        }))
        .route("/refunds", post(|State(s): State<Arc<PspState>>, Json(b): Json<Value>| async move {
            if let Some(code) = psp_fault(&s) { return (code, Json(json!({}))); }
            let id = b["refund_id"].as_str().unwrap().to_owned();
            s.refunds.lock().unwrap().push(id.clone());
            psp_reply(&s, json!({"refund_id": id, "status": "succeeded"}))
        }))
        .route("/payments/{id}", get(|State(s): State<Arc<PspState>>, Path(id): Path<String>| async move {
            s.gets.fetch_add(1, Ordering::SeqCst);
            Json(json!({"payment_id": id, "status": payment_status(&s),
                        "amount_captured": s.captured.load(Ordering::SeqCst)}))
        }))
        .route("/refunds/{id}", get(|State(s): State<Arc<PspState>>, Path(id): Path<String>| async move {
            s.gets.fetch_add(1, Ordering::SeqCst);
            if s.refunds.lock().unwrap().contains(&id) {
                (StatusCode::OK, Json(json!({"refund_id": id, "status": "succeeded"})))
            } else {
                (StatusCode::NOT_FOUND, Json(json!({})))
            }
        }))
        .with_state(state)
}

struct Clear(bool);
impl DocumentaryVeto for Clear {
    fn clear(
        &self,
        _: OperationClass,
        _: &str,
    ) -> impl Future<Output = Result<(), CpAuthorityError>> + Send {
        let ok = self.0;
        async move {
            if ok {
                Ok(())
            } else {
                Err(CpAuthorityError::NotAuthorised)
            }
        }
    }
}

struct Harness {
    executor: MoneyMovementExecutor,
    cp: IamCpAuthority,
    cp_state: Arc<CpState>,
    psp: Arc<PspState>,
    tenant: String,
    admin: String,
    runtime: String,
}

async fn serve(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/")
}

async fn harness() -> Option<Harness> {
    let urls = (
        std::env::var("LA05_NATIVE_PG_TEST_URL"),
        std::env::var("LA05_NATIVE_PG_ADMIN_URL"),
    );
    // CI sets LA05_REQUIRE_PG so a missing database fails rather than skips.
    if std::env::var("LA05_REQUIRE_PG").is_ok() {
        assert!(
            urls.0.is_ok() && urls.1.is_ok(),
            "PostgreSQL 17 is required for the money-movement PEP tests"
        );
    }
    let (runtime, admin) = (urls.0.ok()?, urls.1.ok()?);
    let cp_state = Arc::new(CpState::default());
    let cp_origin = serve(
        Router::new()
            .route(
                "/token",
                post(|| async { Json(json!({"access_token": "synthetic-workload-token"})) }),
            )
            .route(
                "/internal/payments/v1/money-movement-authority/resolve",
                post(cp_resolve),
            )
            .with_state(cp_state.clone()),
    )
    .await;
    let psp = Arc::new(PspState::default());
    let psp_origin = serve(provider(psp.clone())).await;
    let cp = IamCpAuthority::loopback(
        &cp_origin,
        &format!("{cp_origin}token"),
        "workload".into(),
        "synthetic".into(),
    )
    .unwrap();
    let transport = HyperSwitchTransport::loopback_test_client(&psp_origin).unwrap();
    let executor = MoneyMovementExecutor::local_integration(
        runtime.clone(),
        transport,
        "test",
        "ei_paytest01".into(),
    )
    .unwrap();
    let tenant = format!("tn_mm{}", &Uuid::now_v7().simple().to_string()[20..]);
    let h = Harness {
        executor,
        cp,
        cp_state,
        psp,
        tenant,
        admin,
        runtime,
    };
    h.activate("mprof_synthetic").await;
    Some(h)
}

impl Harness {
    async fn admin(&self) -> Client {
        let (client, connection) = tokio_postgres::connect(&self.admin, NoTls).await.unwrap();
        tokio::spawn(connection);
        client
    }

    async fn activate(&self, profile: &str) {
        self.activate_in(profile, "SANDBOX").await;
    }

    async fn activate_in(&self, profile: &str, environment: &str) {
        self.admin().await.execute(
            "INSERT INTO payment_ops.provider_activation (tenant_id, responsible_legal_entity_id,
                merchant_profile_reference, market_code, currency_code, operation_kinds, provider_environment,
                provider_certification_reference, merchant_activation_reference, valid_from, valid_until)
             VALUES ($1,$2,$3,'UG','UGX',ARRAY['AUTHORIZE','CAPTURE','CANCEL','REFUND'],$4,
                     'synthetic/cert','synthetic/activation',now()-interval '1 hour',now()+interval '1 hour')",
            &[&self.tenant, &LE, &profile, &environment],
        ).await.unwrap();
    }

    async fn revoke_all(&self) {
        self.admin().await.execute(
            "INSERT INTO payment_ops.provider_activation_revocation (activation_id, tenant_id, responsible_legal_entity_id, reason)
             SELECT activation_id, tenant_id, responsible_legal_entity_id, 'synthetic withdrawal'
               FROM payment_ops.provider_activation WHERE tenant_id=$1
                AND activation_id NOT IN (SELECT activation_id FROM payment_ops.provider_activation_revocation)",
            &[&self.tenant],
        ).await.unwrap();
    }

    fn command(
        &self,
        class: OperationClass,
        key: &str,
        payment: &str,
        amount: i64,
        parent: Option<&str>,
    ) -> MoneyMovementCommand {
        MoneyMovementCommand {
            class,
            idempotency_key: format!("{key}-{}", &self.tenant[5..]),
            operation_reference: format!("payments/{key}/{}", &self.tenant[5..]),
            instruction: Instruction {
                tenant_id: self.tenant.clone(),
                operating_organisation_id: ORG.into(),
                market: "UG".into(),
                currency: "UGX".into(),
                amount_minor: amount,
                payment_reference: payment.into(),
                parent_reference: parent.map(str::to_owned),
                merchant_profile_reference: "mprof_synthetic".into(),
                destination_reference: (class == OperationClass::Refund)
                    .then(|| crate::money_movement::ORIGINAL_PAYMENT_METHOD.into()),
            },
            responsible_legal_entity_id: LE.into(),
            operating_organisation_id: Uuid::parse_str(ORG).unwrap(),
            correlation_id: Uuid::now_v7(),
            payment_token: (class == OperationClass::Authorize)
                .then(|| "tok_synthetic_visa".into()),
        }
    }

    async fn run(
        &self,
        command: &MoneyMovementCommand,
    ) -> Result<MoneyMovementOutcome, MoneyMovementError> {
        self.executor
            .execute(&self.cp, &Clear(true), Uuid::now_v7(), command)
            .await
    }

    fn modes(&self, modes: &[CpMode]) {
        *self.cp_state.modes.lock().unwrap() = modes.to_vec();
    }

    fn cp_calls(&self) -> usize {
        self.cp_state.calls.lock().unwrap().len()
    }

    fn posts(&self) -> usize {
        self.psp.posts.load(Ordering::SeqCst)
    }

    async fn count(&self, sql: &str) -> i64 {
        let mut client = {
            let (client, connection) = tokio_postgres::connect(&self.runtime, NoTls).await.unwrap();
            tokio::spawn(connection);
            client
        };
        let trx = client.transaction().await.unwrap();
        trx.execute("SELECT set_config('baobab.tenant_id',$1,true), set_config('baobab.responsible_legal_entity_id',$2,true)",
            &[&self.tenant, &LE]).await.unwrap();
        trx.query_one(sql, &[]).await.unwrap().get(0)
    }

    async fn authorize(&self, payment: &str, amount: i64) {
        let out = self
            .run(&self.command(
                OperationClass::Authorize,
                &format!("auth-{payment}"),
                payment,
                amount,
                None,
            ))
            .await;
        assert!(
            matches!(out, Ok(MoneyMovementOutcome::Executed { .. })),
            "{out:?}"
        );
    }
}

fn unique(prefix: &str) -> String {
    format!("{prefix}_{}", &Uuid::now_v7().simple().to_string()[16..])
}

#[tokio::test]
async fn payout_is_refused_before_any_io() {
    // Nothing listens on these ports: any I/O would surface as unavailable.
    let transport = HyperSwitchTransport::loopback_test_client("http://127.0.0.1:1/").unwrap();
    let executor = MoneyMovementExecutor::local_integration(
        "host=127.0.0.1 port=1 user=none".into(),
        transport,
        "test",
        "ei_paytest01".into(),
    )
    .unwrap();
    let cp = IamCpAuthority::loopback(
        "http://127.0.0.1:1/",
        "http://127.0.0.1:1/token",
        "w".into(),
        "s".into(),
    )
    .unwrap();
    let command = MoneyMovementCommand {
        class: OperationClass::Payout,
        idempotency_key: "payout-key-000000001".into(),
        operation_reference: "payments/payout/1".into(),
        instruction: Instruction {
            tenant_id: "tn_payout".into(),
            operating_organisation_id: ORG.into(),
            market: "UG".into(),
            currency: "UGX".into(),
            amount_minor: 100,
            payment_reference: "payout_1".into(),
            parent_reference: None,
            merchant_profile_reference: "mprof_synthetic".into(),
            destination_reference: Some("bnf_synthetic".into()),
        },
        responsible_legal_entity_id: LE.into(),
        operating_organisation_id: Uuid::parse_str(ORG).unwrap(),
        correlation_id: Uuid::now_v7(),
        payment_token: None,
    };
    assert_eq!(
        executor
            .execute(&cp, &Clear(true), Uuid::now_v7(), &command)
            .await,
        Err(MoneyMovementError::NotActivated)
    );
}

#[tokio::test]
async fn protected_environments_and_remote_databases_cannot_construct_the_executor() {
    let transport = HyperSwitchTransport::loopback_test_client("http://127.0.0.1:1/").unwrap();
    for (env, url) in [
        ("staging", "host=127.0.0.1 user=x"),
        ("production", "host=127.0.0.1 user=x"),
        ("test", "host=db.remote user=x"),
    ] {
        assert!(
            MoneyMovementExecutor::local_integration(
                url.into(),
                transport.clone(),
                env,
                "ei_x01".into()
            )
            .is_err()
        );
    }
}

#[tokio::test]
async fn invalid_instruction_never_reaches_cp() {
    let transport = HyperSwitchTransport::loopback_test_client("http://127.0.0.1:1/").unwrap();
    let executor = MoneyMovementExecutor::local_integration(
        "host=127.0.0.1 port=1 user=none".into(),
        transport,
        "test",
        "ei_paytest01".into(),
    )
    .unwrap();
    let cp = IamCpAuthority::loopback(
        "http://127.0.0.1:1/",
        "http://127.0.0.1:1/token",
        "w".into(),
        "s".into(),
    )
    .unwrap();
    let mut command = MoneyMovementCommand {
        class: OperationClass::Refund,
        idempotency_key: "refund-key-00000001".into(),
        operation_reference: "payments/refund/1".into(),
        instruction: Instruction {
            tenant_id: "tn_invalid".into(),
            operating_organisation_id: ORG.into(),
            market: "UG".into(),
            currency: "UGX".into(),
            amount_minor: 100,
            payment_reference: "refund_1".into(),
            parent_reference: Some("pay_1".into()),
            merchant_profile_reference: "mprof_synthetic".into(),
            destination_reference: Some("bnf_elsewhere".into()),
        },
        responsible_legal_entity_id: LE.into(),
        operating_organisation_id: Uuid::parse_str(ORG).unwrap(),
        correlation_id: Uuid::now_v7(),
        payment_token: None,
    };
    assert_eq!(
        executor
            .execute(&cp, &Clear(true), Uuid::now_v7(), &command)
            .await,
        Err(MoneyMovementError::InvalidInstruction)
    );
    command.instruction.destination_reference =
        Some(crate::money_movement::ORIGINAL_PAYMENT_METHOD.into());
    command.payment_token = Some("tok_not_for_refunds".into());
    assert_eq!(
        executor
            .execute(&cp, &Clear(true), Uuid::now_v7(), &command)
            .await,
        Err(MoneyMovementError::InvalidInstruction)
    );
}

#[tokio::test]
async fn postgres17_lifecycle_needs_two_decisions_and_sends_each_movement_once() {
    let Some(h) = harness().await else { return };
    let payment = unique("pay");
    h.authorize(&payment, 1000).await;
    let capture = h.command(
        OperationClass::Capture,
        "capture-1",
        &payment,
        600,
        Some(&payment),
    );
    assert!(matches!(
        h.run(&capture).await,
        Ok(MoneyMovementOutcome::Executed { .. })
    ));
    let refund = h.command(
        OperationClass::Refund,
        "refund-1",
        &unique("refund"),
        600,
        Some(&payment),
    );
    assert!(matches!(
        h.run(&refund).await,
        Ok(MoneyMovementOutcome::Executed { .. })
    ));
    assert_eq!(
        h.posts(),
        3,
        "exactly one provider instruction per movement"
    );
    assert_eq!(h.cp_calls(), 6, "two CP decisions per movement");
    let attempts: Vec<u64> = h
        .cp_state
        .calls
        .lock()
        .unwrap()
        .iter()
        .map(|c| c.1)
        .collect();
    assert_eq!(attempts, vec![1, 2, 1, 2, 1, 2]);
    assert_eq!(
        h.count("SELECT count(*) FROM payment_ops.authority_decision")
            .await,
        6
    );
    assert_eq!(
        h.count("SELECT count(*) FROM payment_ops.dispatch_attempt")
            .await,
        3
    );
    assert_eq!(
        h.count("SELECT count(*) FROM payment_ops.provider_observation WHERE outcome='CONFIRMED'")
            .await,
        3
    );
    // Replaying a confirmed movement sends nothing and asks CP nothing.
    assert!(matches!(
        h.run(&capture).await,
        Ok(MoneyMovementOutcome::Replayed { .. })
    ));
    assert_eq!((h.posts(), h.cp_calls()), (3, 6));
    // The same key for another amount is a conflict, not a second capture.
    let mut changed = capture.clone();
    changed.instruction.amount_minor = 601;
    assert_eq!(
        h.run(&changed).await,
        Err(MoneyMovementError::IdempotencyConflict)
    );
    assert_eq!(h.posts(), 3);
}

#[tokio::test]
async fn postgres17_payments_financial_approval_is_independent_of_cp() {
    let Some(h) = harness().await else { return };
    let payment = unique("pay");
    h.authorize(&payment, 500).await;
    let posts = h.posts();
    // CP authorises every one of these; Payments' own limits refuse them.
    let over_capture = h.command(
        OperationClass::Capture,
        "capture-over",
        &payment,
        501,
        Some(&payment),
    );
    assert_eq!(
        h.run(&over_capture).await,
        Err(MoneyMovementError::FinancialLimit)
    );
    let early_refund = h.command(
        OperationClass::Refund,
        "refund-early",
        &unique("refund"),
        1,
        Some(&payment),
    );
    assert_eq!(
        h.run(&early_refund).await,
        Err(MoneyMovementError::FinancialLimit)
    );
    let orphan = h.command(
        OperationClass::Capture,
        "capture-orphan",
        &payment,
        1,
        Some("pay_does_not_exist"),
    );
    assert_eq!(h.run(&orphan).await, Err(MoneyMovementError::StateConflict));
    assert_eq!(h.posts(), posts, "no refused movement reached the provider");
    assert_eq!(
        h.count(
            "SELECT count(*) FROM payment_ops.native_execution WHERE operation_kind<>'AUTHORIZE'"
        )
        .await,
        0
    );
}

#[tokio::test]
async fn postgres17_cp_denial_unavailability_and_documentary_hold_send_nothing() {
    let Some(h) = harness().await else { return };
    let payment = unique("pay");
    h.authorize(&payment, 900).await;
    let posts = h.posts();
    h.modes(&[CpMode::Deny]);
    let denied = h.command(
        OperationClass::Capture,
        "capture-denied",
        &payment,
        100,
        Some(&payment),
    );
    assert_eq!(
        h.run(&denied).await,
        Err(MoneyMovementError::AuthorityDenied(
            "REVOKED_OR_EXPIRED".into()
        ))
    );
    h.modes(&[CpMode::Unavailable]);
    let unavailable = h.command(
        OperationClass::Capture,
        "capture-cpdown",
        &payment,
        100,
        Some(&payment),
    );
    assert_eq!(
        h.run(&unavailable).await,
        Err(MoneyMovementError::AuthorityUnavailable)
    );
    h.modes(&[CpMode::WrongFingerprint]);
    let drift = h.command(
        OperationClass::Capture,
        "capture-drift",
        &payment,
        100,
        Some(&payment),
    );
    assert_eq!(
        h.run(&drift).await,
        Err(MoneyMovementError::AuthorityUnavailable)
    );
    h.modes(&[CpMode::Authorize]);
    let held = h.command(
        OperationClass::Capture,
        "capture-held",
        &payment,
        100,
        Some(&payment),
    );
    assert_eq!(
        h.executor
            .execute(&h.cp, &Clear(false), Uuid::now_v7(), &held)
            .await,
        Err(MoneyMovementError::DocumentaryHold)
    );
    assert_eq!(h.posts(), posts);
    assert_eq!(
        h.count("SELECT count(*) FROM payment_ops.native_execution WHERE operation_kind='CAPTURE'")
            .await,
        0
    );
}

#[tokio::test]
async fn postgres17_revocation_between_reservation_and_dispatch_blocks_the_send() {
    let Some(h) = harness().await else { return };
    let payment = unique("pay");
    h.authorize(&payment, 700).await;
    let posts = h.posts();
    // Attempt 1 authorised, attempt 2 (the execution point) revoked.
    h.modes(&[CpMode::Authorize, CpMode::Deny]);
    let capture = h.command(
        OperationClass::Capture,
        "capture-revoked",
        &payment,
        700,
        Some(&payment),
    );
    assert_eq!(
        h.run(&capture).await,
        Err(MoneyMovementError::AuthorityDenied(
            "REVOKED_OR_EXPIRED".into()
        ))
    );
    assert_eq!(
        h.posts(),
        posts,
        "a revoked movement never reaches the provider"
    );
    assert_eq!(
        h.count("SELECT count(*) FROM payment_ops.dispatch_attempt")
            .await,
        1,
        "only the authorisation"
    );
    // Authority restored: the held reservation dispatches on a fresh decision.
    h.modes(&[CpMode::Authorize]);
    assert!(matches!(
        h.run(&capture).await,
        Ok(MoneyMovementOutcome::Executed { .. })
    ));
    assert_eq!(h.posts(), posts + 1);
}

#[tokio::test]
async fn postgres17_withdrawn_readiness_blocks_even_with_cp_authority() {
    let Some(h) = harness().await else { return };
    let payment = unique("pay");
    h.authorize(&payment, 300).await;
    h.revoke_all().await;
    let posts = h.posts();
    let capture = h.command(
        OperationClass::Capture,
        "capture-unready",
        &payment,
        300,
        Some(&payment),
    );
    assert_eq!(
        h.run(&capture).await,
        Err(MoneyMovementError::ProviderNotReady)
    );
    assert_eq!(h.posts(), posts);
}

#[tokio::test]
async fn postgres17_unknown_outcome_is_reconciled_never_resent() {
    let Some(h) = harness().await else { return };
    let payment = unique("pay");
    h.authorize(&payment, 800).await;
    // The capture lands at the provider, but its response is lost.
    h.psp.lose_next_response.store(true, Ordering::SeqCst);
    let capture = h.command(
        OperationClass::Capture,
        "capture-lost",
        &payment,
        800,
        Some(&payment),
    );
    let first = h.run(&capture).await;
    assert!(
        matches!(first, Err(MoneyMovementError::OutcomeUnknown { .. })),
        "{first:?}"
    );
    assert_eq!(h.psp.captured.load(Ordering::SeqCst), 800);
    let posts = h.posts();
    // The retry reads the provider back and confirms; it sends nothing.
    assert!(matches!(
        h.run(&capture).await,
        Ok(MoneyMovementOutcome::Replayed { .. })
    ));
    assert_eq!(h.posts(), posts, "no second capture");
    assert_eq!(h.psp.captured.load(Ordering::SeqCst), 800);
    assert!(h.psp.gets.load(Ordering::SeqCst) >= 1);
    // An unknown capture still counted against the balance before it settled.
    let refund = h.command(
        OperationClass::Refund,
        "refund-after",
        &unique("refund"),
        800,
        Some(&payment),
    );
    assert!(matches!(
        h.run(&refund).await,
        Ok(MoneyMovementOutcome::Executed { .. })
    ));
}

#[tokio::test]
async fn postgres17_a_request_proven_absent_is_resent_once_on_a_fresh_decision() {
    let Some(h) = harness().await else { return };
    let payment = unique("pay");
    h.authorize(&payment, 400).await;
    h.psp.drop_next_request.store(true, Ordering::SeqCst);
    let refund_ref = unique("refund");
    // Nothing captured yet, so capture first; then lose a refund request.
    let capture = h.command(
        OperationClass::Capture,
        "capture-ok",
        &payment,
        400,
        Some(&payment),
    );
    let lost = h.run(&capture).await;
    assert!(
        matches!(lost, Err(MoneyMovementError::OutcomeUnknown { .. })),
        "{lost:?}"
    );
    assert_eq!(
        h.psp.captured.load(Ordering::SeqCst),
        0,
        "the request never landed"
    );
    let calls = h.cp_calls();
    assert!(matches!(
        h.run(&capture).await,
        Ok(MoneyMovementOutcome::Executed { .. })
    ));
    assert_eq!(
        h.psp.captured.load(Ordering::SeqCst),
        400,
        "captured exactly once"
    );
    let attempts: Vec<u64> = h.cp_state.calls.lock().unwrap()[calls..]
        .iter()
        .map(|c| c.1)
        .collect();
    assert_eq!(
        attempts,
        vec![3],
        "the re-send needed its own fresh decision"
    );
    h.psp.drop_next_request.store(true, Ordering::SeqCst);
    let refund = h.command(
        OperationClass::Refund,
        "refund-lost",
        &refund_ref,
        100,
        Some(&payment),
    );
    assert!(matches!(
        h.run(&refund).await,
        Err(MoneyMovementError::OutcomeUnknown { .. })
    ));
    assert!(matches!(
        h.run(&refund).await,
        Ok(MoneyMovementOutcome::Executed { .. })
    ));
    assert_eq!(
        h.psp
            .refunds
            .lock()
            .unwrap()
            .iter()
            .filter(|r| **r == refund_ref)
            .count(),
        1
    );
}

#[tokio::test]
async fn postgres17_void_only_before_capture() {
    let Some(h) = harness().await else { return };
    let voided = unique("pay");
    h.authorize(&voided, 250).await;
    let void = h.command(OperationClass::Void, "void-1", &voided, 250, Some(&voided));
    assert!(matches!(
        h.run(&void).await,
        Ok(MoneyMovementOutcome::Executed { .. })
    ));
    let after = h.command(
        OperationClass::Capture,
        "capture-after-void",
        &voided,
        250,
        Some(&voided),
    );
    assert_eq!(h.run(&after).await, Err(MoneyMovementError::StateConflict));
}

#[tokio::test]
async fn postgres17_a_replay_can_never_substitute_another_payment_token() {
    let Some(h) = harness().await else { return };
    let payment = unique("pay");
    let mut authorize = h.command(OperationClass::Authorize, "auth-token", &payment, 500, None);
    h.psp.drop_next_request.store(true, Ordering::SeqCst);
    assert!(matches!(
        h.run(&authorize).await,
        Err(MoneyMovementError::OutcomeUnknown { .. })
    ));
    let posts = h.posts();
    authorize.payment_token = Some("tok_someone_elses_card".into());
    assert_eq!(
        h.run(&authorize).await,
        Err(MoneyMovementError::IdempotencyConflict)
    );
    assert_eq!(h.posts(), posts, "the substituted token was never sent");
    authorize.payment_token = Some("tok_synthetic_visa".into());
    assert!(matches!(
        h.run(&authorize).await,
        Ok(MoneyMovementOutcome::Executed { .. })
    ));
}

#[tokio::test]
async fn postgres17_a_short_capture_is_never_confirmed() {
    let Some(h) = harness().await else { return };
    let payment = unique("pay");
    h.authorize(&payment, 600).await;
    h.psp.short_capture.store(true, Ordering::SeqCst);
    let capture = h.command(
        OperationClass::Capture,
        "capture-short",
        &payment,
        600,
        Some(&payment),
    );
    let out = h.run(&capture).await;
    assert!(
        matches!(out, Err(MoneyMovementError::OutcomeUnknown { .. })),
        "{out:?}"
    );
    assert_eq!(h.psp.captured.load(Ordering::SeqCst), 599);
    // Funds that were never captured can never be refunded.
    let refund = h.command(
        OperationClass::Refund,
        "refund-short",
        &unique("refund"),
        600,
        Some(&payment),
    );
    assert_eq!(
        h.run(&refund).await,
        Err(MoneyMovementError::FinancialLimit)
    );
    assert_eq!(h.count("SELECT count(*) FROM payment_ops.provider_observation o JOIN payment_ops.native_execution e USING (execution_id) WHERE e.operation_kind='CAPTURE' AND o.outcome='CONFIRMED'").await, 0);
}

#[tokio::test]
async fn postgres17_an_activation_for_another_environment_never_counts() {
    let Some(h) = harness().await else { return };
    h.activate_in("mprof_staging_only", "STAGING").await;
    let payment = unique("pay");
    let mut authorize = h.command(
        OperationClass::Authorize,
        "auth-staging",
        &payment,
        500,
        None,
    );
    authorize.instruction.merchant_profile_reference = "mprof_staging_only".into();
    let posts = h.posts();
    assert_eq!(
        h.run(&authorize).await,
        Err(MoneyMovementError::ProviderNotReady)
    );
    assert_eq!(h.posts(), posts);
}
