//! LA-05E5: durable PostgreSQL 17 -> HyperSwitch operation composition.
//!
//! Explicitly synthetic/integration-only until the platform has a certified
//! merchant/settlement mapping, IAM-authenticated CP decisions, database TLS,
//! durable reconciliation and protected acceptance. No HTTP client can supply
//! a privileged NativePaymentContext directly. A native outcome is OBSERVED,
//! never converted into a canonical captured/settled finance fact here.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio_postgres::NoTls;
use uuid::Uuid;

use crate::documentary_restriction::{
    execute_with_current_documentary_payment_mandate, require_current_documentary_clearance,
};
use crate::hyperswitch::{
    HyperSwitchError, HyperSwitchTransport, NativeOperation, ProviderObservation,
};
use crate::legal_actor::require_current_payment_actor;
use crate::legal_actor::{
    LegalActorGateError, PaymentLegalOperation, PaymentProviderLegalReadiness,
};
use std::cell::RefCell;
use time::OffsetDateTime;

/// Trusted, independently reconciled provider coordinates. Never take them
/// from a browser, payment Create body, webhook or mutable tenant display name.
#[derive(Debug, Clone)]
pub struct ProviderBinding {
    pub merchant_id: String,
    pub certification_reference: String,
    pub merchant_activation_reference: String,
    pub provider_engine_instance_id: String,
    pub profile_id: String,
}

#[derive(Debug, Clone)]
pub struct NativePaymentContext {
    pub tenant_id: String,
    pub legal_entity_id: String,
    pub organisation_id: Uuid,
    pub payment_reference: String,
    pub market: String,
    pub currency: String,
    pub amount_minor: i64,
    pub correlation_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    GovernanceDenied,
    InvalidScope,
    JournalUnavailable,
    AlreadyReservedNeedsReconciliation,
    ProviderUnavailableNeedsReconciliation,
    ProviderRejectedNeedsReconciliation,
    InvalidProviderResponseNeedsReconciliation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionResult {
    ProviderObserved {
        execution_id: Uuid,
        native_reference: String,
        status: String,
    },
}

/// The only runnable composition at this stage is local synthetic integration:
/// PostgreSQL 17 NoTls on loopback. Real TLS/certified runtime wiring must be
/// independently reviewed before enabling any protected deployment.
pub struct NativeTransactionService {
    database_url: String,
    transport: HyperSwitchTransport,
}

impl NativeTransactionService {
    pub fn local_integration(
        database_url: String,
        transport: HyperSwitchTransport,
        environment: &str,
    ) -> Result<Self, ExecutionError> {
        if !matches!(environment, "test" | "development" | "integration") {
            return Err(ExecutionError::GovernanceDenied);
        }
        let address: tokio_postgres::Config = database_url
            .parse()
            .map_err(|_| ExecutionError::InvalidScope)?;
        // Reject remote/plaintext DB connections; protected environments
        // need a real verified TLS connector instead of this local harness.
        if address.get_hosts().is_empty()
            || address.get_hosts().iter().any(|h| {
                !matches!(h,
                tokio_postgres::config::Host::Tcp(host)
                if host == "127.0.0.1" || host == "localhost" || host == "::1")
            })
        {
            return Err(ExecutionError::InvalidScope);
        }
        Ok(Self {
            database_url,
            transport,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn execute<'a, L, D, N>(
        &self,
        context: &NativePaymentContext,
        operation: &PaymentLegalOperation<'a>,
        binding: &ProviderBinding,
        stable_idempotency_key: &str,
        native_operation: NativeOperation<'_>,
        body: &Value,
        fresh_cp_actor: L,
        fresh_documentary: D,
        readiness: &dyn PaymentProviderLegalReadiness,
        now: N,
    ) -> Result<ExecutionResult, ExecutionError>
    where
        L: FnMut() -> Result<Value, LegalActorGateError>,
        D: FnMut() -> Result<Value, LegalActorGateError>,
        N: Fn() -> OffsetDateTime,
    {
        self.validate(
            context,
            operation,
            binding,
            stable_idempotency_key,
            &native_operation,
            body,
        )?;
        HyperSwitchTransport::validate_operation(&native_operation, body, stable_idempotency_key)
            .map_err(|_| ExecutionError::InvalidScope)?;
        // Shared references permit the authoritative sources to be re-queried
        // after the journal transaction, not just before it. RefCell borrows
        // are released synchronously before each async DB/network boundary.
        let actor = RefCell::new(fresh_cp_actor);
        let documentary = RefCell::new(fresh_documentary);
        execute_with_current_documentary_payment_mandate(
            operation,
            &context.organisation_id.to_string(),
            || (actor.borrow_mut())(),
            || (documentary.borrow_mut())(),
            readiness,
            &now,
            |mandate| {
                let actor_ref = &actor;
                let documentary_ref = &documentary;
                let now_ref = &now;
                async move {
                    self.execute_after_authority(
                        context,
                        binding,
                        &mandate,
                        stable_idempotency_key,
                        native_operation,
                        body,
                        || {
                            // A DB connection/lock/commit may have consumed the
                            // original negative-only two-second documentary lease.
                            let checked = require_current_payment_actor(
                                operation,
                                &(actor_ref.borrow_mut())()
                                    .map_err(|_| ExecutionError::GovernanceDenied)?,
                                now_ref(),
                            )
                            .map_err(|_| ExecutionError::GovernanceDenied)?;
                            if checked != mandate {
                                return Err(ExecutionError::GovernanceDenied);
                            }
                            require_current_documentary_clearance(
                                operation,
                                &context.organisation_id.to_string(),
                                &(documentary_ref.borrow_mut())()
                                    .map_err(|_| ExecutionError::GovernanceDenied)?,
                                now_ref(),
                            )
                            .map_err(|_| ExecutionError::GovernanceDenied)?;
                            readiness
                                .require_ready(operation, &mandate)
                                .map_err(|_| ExecutionError::GovernanceDenied)?;
                            Ok(())
                        },
                    )
                    .await
                }
            },
        )
        .await
        .map_err(|_| ExecutionError::GovernanceDenied)?
    }

    fn validate(
        &self,
        context: &NativePaymentContext,
        operation: &PaymentLegalOperation<'_>,
        binding: &ProviderBinding,
        stable_key: &str,
        native: &NativeOperation<'_>,
        body: &Value,
    ) -> Result<(), ExecutionError> {
        if binding.merchant_id.len() < 3
            || binding.merchant_id.len() > 160
            || !binding
                .merchant_id
                .bytes()
                .all(|x| x.is_ascii_alphanumeric() || b"._:-".contains(&x))
            || !context.tenant_id.starts_with("tn_")
            || context.tenant_id.len() > 63
            || context.tenant_id[3..].is_empty()
            || !context.tenant_id[3..]
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            || context.legal_entity_id != operation.expected_legal_entity_id
            || context.payment_reference != operation.operation_reference
            || context.market != operation.market
            || !(context.market.len() == 2
                && context.market.bytes().all(|b| b.is_ascii_uppercase()))
            || !(context.currency.len() == 3
                && context.currency.bytes().all(|b| b.is_ascii_uppercase()))
            || context.amount_minor <= 0
            || binding.certification_reference.len() < 3
            || binding.merchant_activation_reference.len() < 3
            || binding.provider_engine_instance_id.len() < 3
            || binding.profile_id.len() < 3
            || stable_key.len() < 16
            || stable_key.len() > 128
            || !stable_key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
            || !body.is_object()
        {
            return Err(ExecutionError::InvalidScope);
        }
        // Stage one supports intent creation ONLY; financial capture/reversal
        // requires its own certification, amount and beneficiary invariants.
        if *native != NativeOperation::Create
            || body.get("amount").and_then(Value::as_i64) != Some(context.amount_minor)
            || body.get("currency").and_then(Value::as_str) != Some(context.currency.as_str())
            || body.get("profile_id").and_then(Value::as_str) != Some(binding.profile_id.as_str())
            || body.get("confirm").and_then(Value::as_bool) != Some(false)
        {
            return Err(ExecutionError::InvalidScope);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_after_authority<F>(
        &self,
        scope: &NativePaymentContext,
        binding: &ProviderBinding,
        mandate_id: &str,
        key: &str,
        operation: NativeOperation<'_>,
        body: &Value,
        mut verify_again_at_send: F,
    ) -> Result<ExecutionResult, ExecutionError>
    where
        F: FnMut() -> Result<(), ExecutionError>,
    {
        let mandate = Uuid::parse_str(mandate_id).map_err(|_| ExecutionError::GovernanceDenied)?;
        let (mut client, connection) = tokio_postgres::connect(&self.database_url, NoTls)
            .await
            .map_err(|_| ExecutionError::JournalUnavailable)?;
        tokio::spawn(async move {
            if let Err(err) = connection.await {
                tracing::warn!(event = "payment.pg.connection_lost", error_category = %err.code().map(|c| c.code()).unwrap_or("transport"));
            }
        });
        let trx = client
            .transaction()
            .await
            .map_err(|_| ExecutionError::JournalUnavailable)?;
        // Session-local trusted context. Never pass it via request-supplied
        // headers or provider callbacks; FORCE RLS binds every child row.
        trx.query_one(
            "SELECT set_config('baobab.tenant_id',$1,true)",
            &[&scope.tenant_id],
        )
        .await
        .map_err(|_| ExecutionError::JournalUnavailable)?;
        trx.execute(
            "SELECT set_config('baobab.responsible_legal_entity_id',$1,true)",
            &[&scope.legal_entity_id],
        )
        .await
        .map_err(|_| ExecutionError::JournalUnavailable)?;
        let body_hash = hex::encode(Sha256::digest(body.to_string().as_bytes()));
        // Serialize competing workers at the exact tenant/actor/operation/key
        // boundary. A replay never initiates another native provider call.
        trx.query_one(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!(
                "{}|{}|CREATE_INTENT|{}",
                scope.tenant_id, scope.legal_entity_id, key
            )],
        )
        .await
        .map_err(|_| ExecutionError::JournalUnavailable)?;
        let old = trx
            .query_opt(
                "SELECT execution_id FROM payment_ops.native_execution
             WHERE tenant_id=$1 AND responsible_legal_entity_id=$2
               AND operation_kind='CREATE_INTENT' AND idempotency_key=$3",
                &[&scope.tenant_id, &scope.legal_entity_id, &key],
            )
            .await
            .map_err(|_| ExecutionError::JournalUnavailable)?;
        if old.is_some() {
            return Err(ExecutionError::AlreadyReservedNeedsReconciliation);
        }
        let row = trx
            .query_one(
                "SELECT payment_ops.reserve_native_execution(
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
                &[
                    &scope.tenant_id,
                    &"CREATE_INTENT",
                    &key,
                    &body_hash,
                    &scope.payment_reference,
                    &scope.organisation_id,
                    &scope.legal_entity_id,
                    &mandate,
                    &format!("cp/live-mandate/{mandate_id}"),
                    &binding.certification_reference,
                    &binding.merchant_activation_reference,
                    &scope.market,
                    &scope.currency,
                    &scope.amount_minor,
                ],
            )
            .await
            .map_err(|_| ExecutionError::JournalUnavailable)?;
        let execution_id: Uuid = row.get(0);
        trx.execute(
            "INSERT INTO payment_ops.transaction_outbox
               (tenant_id, responsible_legal_entity_id, execution_id,
                event_type, event_payload, correlation_id)
             VALUES ($1,$2,$3,'payments.native-operation.reserved.v1',
                jsonb_build_object('payment_reference',$4::text,'kind','CREATE_INTENT'),$5)",
            &[
                &scope.tenant_id,
                &scope.legal_entity_id,
                &execution_id,
                &scope.payment_reference,
                &scope.correlation_id,
            ],
        )
        .await
        .map_err(|_| ExecutionError::JournalUnavailable)?;
        trx.commit()
            .await
            .map_err(|_| ExecutionError::JournalUnavailable)?;
        // No PSP instruction can cross the durable DB commit boundary without
        // a fresh second CP+documentary+provider readiness decision.
        // A denial leaves the reservation held, never silently replays money.
        verify_again_at_send()?;
        // The irreversible send happens only AFTER durable commit. A timeout
        // can mean remote success; do not silently resend on retry.
        let observed = self.transport.execute(operation, body, key).await;
        self.persist_observation(scope, binding, execution_id, &observed)
            .await?;
        match observed {
            Ok(ProviderObservation {
                native_reference,
                provider_status,
            }) => Ok(ExecutionResult::ProviderObserved {
                execution_id,
                native_reference,
                status: provider_status,
            }),
            Err(HyperSwitchError::Unavailable) => {
                Err(ExecutionError::ProviderUnavailableNeedsReconciliation)
            }
            Err(HyperSwitchError::Rejected) => {
                Err(ExecutionError::ProviderRejectedNeedsReconciliation)
            }
            Err(_) => Err(ExecutionError::InvalidProviderResponseNeedsReconciliation),
        }
    }

    async fn persist_observation(
        &self,
        scope: &NativePaymentContext,
        binding: &ProviderBinding,
        execution: Uuid,
        observed: &Result<ProviderObservation, HyperSwitchError>,
    ) -> Result<(), ExecutionError> {
        let (mut client, connection) = tokio_postgres::connect(&self.database_url, NoTls)
            .await
            .map_err(|_| ExecutionError::JournalUnavailable)?;
        tokio::spawn(async move {
            if connection.await.is_err() {
                tracing::warn!(event = "payment.pg.observation_connection_lost");
            }
        });
        let trx = client
            .transaction()
            .await
            .map_err(|_| ExecutionError::JournalUnavailable)?;
        trx.execute(
            "SELECT set_config('baobab.tenant_id',$1,true)",
            &[&scope.tenant_id],
        )
        .await
        .map_err(|_| ExecutionError::JournalUnavailable)?;
        trx.execute(
            "SELECT set_config('baobab.responsible_legal_entity_id',$1,true)",
            &[&scope.legal_entity_id],
        )
        .await
        .map_err(|_| ExecutionError::JournalUnavailable)?;
        let (kind, evidence) = match observed {
            Ok(state) => (
                "payments.provider-response.observed.v1",
                json!({"provider_reference":state.native_reference, "provider_status":state.provider_status,
                    "provider_engine_instance_id": binding.provider_engine_instance_id,
                    "provider_environment": "SANDBOX",
                    "merchant_id": binding.merchant_id}),
            ),
            Err(_) => (
                "payments.settlement.reconciliation-requested.v1",
                json!({"provider_observation":"UNCERTAIN","manual_reconciliation_required":true}),
            ),
        };
        trx.execute(
            "INSERT INTO payment_ops.transaction_outbox
               (tenant_id, responsible_legal_entity_id, execution_id, event_type,
                event_payload, correlation_id) VALUES ($1,$2,$3,$4,$5,$6)",
            &[
                &scope.tenant_id,
                &scope.legal_entity_id,
                &execution,
                &kind,
                &evidence,
                &scope.correlation_id,
            ],
        )
        .await
        .map_err(|_| ExecutionError::JournalUnavailable)?;
        trx.commit()
            .await
            .map_err(|_| ExecutionError::JournalUnavailable)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, routing::post};
    use serde_json::json;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    fn scope() -> NativePaymentContext {
        NativePaymentContext {
            tenant_id: "tn_syntheticza".into(),
            legal_entity_id: "LE-SYNTHETIC-ZA".into(),
            organisation_id: Uuid::parse_str("0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c").unwrap(),
            payment_reference: "payment-local-synthetic-123".into(),
            market: "ZA".into(),
            currency: "ZAR".into(),
            amount_minor: 1200,
            correlation_id: Uuid::now_v7(),
        }
    }

    fn operation<'a>() -> PaymentLegalOperation<'a> {
        PaymentLegalOperation {
            context_id: "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6b",
            expected_legal_entity_id: "LE-SYNTHETIC-ZA",
            role: "PAYMENT_BENEFICIARY",
            activity: "B2B_RECEIPT",
            market: "ZA",
            capability: "payments.intent.create",
            operation_reference: "payment-local-synthetic-123",
        }
    }

    fn binding() -> ProviderBinding {
        ProviderBinding {
            merchant_id: "merchant_synthetic".into(),
            certification_reference: "synthetic-certification-not-valid".into(),
            merchant_activation_reference: "synthetic-merchant-not-valid".into(),
            provider_engine_instance_id: "synthetic-hyperswitch".into(),
            profile_id: "prof_synthetic_test".into(),
        }
    }

    fn body() -> Value {
        json!({"amount":1200,"currency":"ZAR","profile_id":"prof_synthetic_test",
               "capture_method":"manual","confirm":false})
    }

    #[test]
    fn protected_environment_and_unauthorized_native_body_are_rejected() {
        let transport =
            HyperSwitchTransport::new("https://payments.example.com/", "synthetic".into()).unwrap();
        assert!(matches!(
            NativeTransactionService::local_integration(
                "host=127.0.0.1 user=none".into(),
                transport.clone(),
                "production"
            ),
            Err(ExecutionError::GovernanceDenied)
        ));
        assert!(matches!(
            NativeTransactionService::local_integration(
                "host=payments-db.remote user=none".into(),
                transport.clone(),
                "test"
            ),
            Err(ExecutionError::InvalidScope)
        ));
        let service = NativeTransactionService::local_integration(
            "host=127.0.0.1 user=none".into(),
            transport,
            "test",
        )
        .unwrap();
        assert!(
            service
                .validate(
                    &scope(),
                    &operation(),
                    &binding(),
                    "stable-synthetic-key-001",
                    &NativeOperation::Create,
                    &json!({"amount":1,"currency":"UGX","confirm":true})
                )
                .is_err()
        );
        assert!(
            service
                .validate(
                    &scope(),
                    &operation(),
                    &binding(),
                    "stable-synthetic-key-001",
                    &NativeOperation::Capture {
                        payment_id: "pay_test"
                    },
                    &body()
                )
                .is_err()
        );
    }

    #[tokio::test]
    async fn never_reaches_journal_or_provider_on_cp_refusal() {
        let transport =
            HyperSwitchTransport::new("https://payments.example.com/", "synthetic".into()).unwrap();
        let service = NativeTransactionService::local_integration(
            "host=127.0.0.1 user=none".into(),
            transport,
            "test",
        )
        .unwrap();
        let result = service
            .execute(
                &scope(),
                &operation(),
                &binding(),
                "stable-synthetic-key-001",
                NativeOperation::Create,
                &body(),
                || Err(LegalActorGateError::NotAuthorised),
                || Err(LegalActorGateError::NotAuthorised),
                &NeverReady,
                OffsetDateTime::now_utc,
            )
            .await;
        assert_eq!(result, Err(ExecutionError::GovernanceDenied));
    }

    struct NeverReady;
    impl PaymentProviderLegalReadiness for NeverReady {
        fn require_ready(
            &self,
            _: &PaymentLegalOperation<'_>,
            _: &str,
        ) -> Result<(), LegalActorGateError> {
            Err(LegalActorGateError::ProviderNotReady)
        }
    }

    #[tokio::test]
    async fn postgres17_commits_intent_before_exactly_one_synthetic_provider_send() {
        let Ok(database_url) = std::env::var("LA05_NATIVE_PG_TEST_URL") else {
            return;
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let app = Router::new().route("/payments", post(move |Json(payload): Json<Value>| {
            let observed = observed.clone();
            async move {
                assert_eq!(payload["confirm"], false);
                observed.fetch_add(1, Ordering::SeqCst);
                Json(json!({"payment_id":"pay_synthetic_001","status":"requires_payment_method"}))
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let transport =
            HyperSwitchTransport::loopback_test_client(&format!("http://{addr}/")).unwrap();
        let service =
            NativeTransactionService::local_integration(database_url.clone(), transport, "test")
                .unwrap();
        let ctx = scope();
        let key = format!("synthetic-{}", Uuid::now_v7()).replace("-", "");
        let first = service
            .execute_after_authority(
                &ctx,
                &binding(),
                "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d",
                &key,
                NativeOperation::Create,
                &body(),
                || Ok(()),
            )
            .await
            .unwrap();
        assert!(matches!(first, ExecutionResult::ProviderObserved { .. }));
        let again = service
            .execute_after_authority(
                &ctx,
                &binding(),
                "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d",
                &key,
                NativeOperation::Create,
                &body(),
                || Ok(()),
            )
            .await;
        assert_eq!(
            again,
            Err(ExecutionError::AlreadyReservedNeedsReconciliation)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // Validate both immutable journal facts commit durably, with actor RLS.
        let (mut db, conn) = tokio_postgres::connect(&database_url, NoTls).await.unwrap();
        tokio::spawn(async move {
            let _ = conn.await;
        });
        let tx = db.transaction().await.unwrap();
        tx.query_one(
            "SELECT set_config('baobab.tenant_id',$1,true)",
            &[&ctx.tenant_id],
        )
        .await
        .unwrap();
        tx.query_one(
            "SELECT set_config('baobab.responsible_legal_entity_id',$1,true)",
            &[&ctx.legal_entity_id],
        )
        .await
        .unwrap();
        let rows = tx
            .query(
                "SELECT event_type FROM payment_ops.transaction_outbox
             WHERE execution_id=(SELECT execution_id FROM payment_ops.native_execution
             WHERE idempotency_key=$1 AND tenant_id=$2 AND responsible_legal_entity_id=$3)",
                &[&key, &ctx.tenant_id, &ctx.legal_entity_id],
            )
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        task.abort();
    }
}
