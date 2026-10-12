//! LA-05E5: durable PostgreSQL 17 -> HyperSwitch operation composition.
//!
//! Explicitly synthetic/integration-only until the platform has a certified
//! merchant/settlement mapping, IAM-authenticated CP decisions, database TLS,
//! durable reconciliation and protected acceptance. No HTTP client can supply
//! a privileged NativePaymentContext directly. A native outcome is OBSERVED,
//! never converted into a canonical captured/settled finance fact here.

use sha2::{Digest, Sha256};
use serde_json::{json, Value};
use tokio_postgres::{NoTls, Error as DbError};
use uuid::Uuid;

use crate::documentary_restriction::execute_with_current_documentary_payment_authority;
use crate::hyperswitch::{HyperSwitchError, HyperSwitchTransport, NativeOperation, ProviderObservation};
use crate::legal_actor::{LegalActorGateError, PaymentLegalOperation, PaymentProviderLegalReadiness};
use time::OffsetDateTime;

/// Trusted, independently reconciled provider coordinates. Never take them
/// from a browser, payment Create body, webhook or mutable tenant display name.
#[derive(Debug, Clone)]
pub struct ProviderBinding {
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
    ProviderObserved { execution_id: Uuid, native_reference: String, status: String },
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
        let address: tokio_postgres::Config =
            database_url.parse().map_err(|_| ExecutionError::InvalidScope)?;
        // Reject remote/plaintext DB connections; protected environments
        // need a real verified TLS connector instead of this local harness.
        if address.get_hosts().is_empty()
            || address.get_hosts().iter().any(|h| !matches!(h,
                tokio_postgres::config::Host::Tcp(host)
                if host == "127.0.0.1" || host == "localhost" || host == "::1"))
        {
            return Err(ExecutionError::InvalidScope);
        }
        Ok(Self { database_url, transport })
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
        mut fresh_cp_actor: L,
        mut fresh_documentary: D,
        readiness: &dyn PaymentProviderLegalReadiness,
        now: N,
    ) -> Result<ExecutionResult, ExecutionError>
    where
        L: FnMut() -> Result<Value, LegalActorGateError>,
        D: FnMut() -> Result<Value, LegalActorGateError>,
        N: Fn() -> OffsetDateTime,
    {
        self.validate(context, operation, binding, stable_idempotency_key, &native_operation, body)?;
        // The authority seam rechecks both CP actor and negative-only
        // documentary clearance immediately before any DB/PSP mutation.
        let mandate = crate::legal_actor::require_current_payment_actor(
            operation, &fresh_cp_actor().map_err(|_| ExecutionError::GovernanceDenied)?,
            now(),
        ).map_err(|_| ExecutionError::GovernanceDenied)?;
        let result = execute_with_current_documentary_payment_authority(
            operation, &context.organisation_id.to_string(),
            || fresh_cp_actor(),
            || fresh_documentary(),
            readiness,
            now,
            || async {
                self.execute_after_authority(
                    context, binding, &mandate, stable_idempotency_key,
                    native_operation, body,
                ).await
            },
        ).await.map_err(|_| ExecutionError::GovernanceDenied)?;
        result
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
        if !context.tenant_id.starts_with("tn_")
            || context.tenant_id.len() > 63
            || context.tenant_id[3..].is_empty()
            || !context.tenant_id[3..].bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            || context.legal_entity_id != operation.expected_legal_entity_id
            || context.payment_reference != operation.operation_reference
            || context.market != operation.market
            || !(context.market.len() == 2 && context.market.bytes().all(|b| b.is_ascii_uppercase()))
            || !(context.currency.len() == 3 && context.currency.bytes().all(|b| b.is_ascii_uppercase()))
            || context.amount_minor <= 0
            || binding.certification_reference.len() < 3
            || binding.merchant_activation_reference.len() < 3
            || binding.provider_engine_instance_id.len() < 3
            || binding.profile_id.len() < 3
            || stable_key.len() < 16
            || stable_key.len() > 128
            || !stable_key.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
            || !body.is_object()
        {
            return Err(ExecutionError::InvalidScope);
        }
        // Stage one supports intent creation ONLY; financial capture/reversal
        // requires its own certification, amount and beneficiary invariants.
        if *native != NativeOperation::Create
            || body.get("amount").and_then(Value::as_i64) != Some(context.amount_minor)
            || body.get("currency").and_then(Value::as_str) != Some(&context.currency)
            || body.get("profile_id").and_then(Value::as_str) != Some(&binding.profile_id)
            || body.get("confirm").and_then(Value::as_bool) != Some(false)
        {
            return Err(ExecutionError::InvalidScope);
        }
        Ok(())
    }

    async fn execute_after_authority(
        &self,
        scope: &NativePaymentContext,
        binding: &ProviderBinding,
        mandate_id: &str,
        key: &str,
        operation: NativeOperation<'_>,
        body: &Value,
    ) -> Result<ExecutionResult, ExecutionError> {
        let mandate = Uuid::parse_str(mandate_id).map_err(|_| ExecutionError::GovernanceDenied)?;
        let (mut client, connection) = tokio_postgres::connect(&self.database_url, NoTls)
            .await.map_err(|_| ExecutionError::JournalUnavailable)?;
        tokio::spawn(async move {
            if let Err(err) = connection.await {
                tracing::warn!(event = "payment.pg.connection_lost", error_category = %err.code().map(|c| c.code()).unwrap_or("transport"));
            }
        });
        let trx = client.transaction().await.map_err(|_| ExecutionError::JournalUnavailable)?;
        // Session-local trusted context. Never pass it via request-supplied
        // headers or provider callbacks; FORCE RLS binds every child row.
        trx.execute("SELECT set_config('baobab.tenant_id',$1,true)", &[&scope.tenant_id])
            .await.map_err(|_| ExecutionError::JournalUnavailable)?;
        trx.execute("SELECT set_config('baobab.responsible_legal_entity_id',$1,true)", &[&scope.legal_entity_id])
            .await.map_err(|_| ExecutionError::JournalUnavailable)?;
        let body_hash = hex::encode(Sha256::digest(body.to_string().as_bytes()));
        // Serialize competing workers at the exact tenant/actor/operation/key
        // boundary. A replay never initiates another native provider call.
        trx.execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!("{}|{}|CREATE_INTENT|{}", scope.tenant_id, scope.legal_entity_id, key)],
        ).await.map_err(|_| ExecutionError::JournalUnavailable)?;
        let old = trx.query_opt(
            "SELECT execution_id FROM payment_ops.native_execution
             WHERE tenant_id=$1 AND responsible_legal_entity_id=$2
               AND operation_kind='CREATE_INTENT' AND idempotency_key=$3",
            &[&scope.tenant_id, &scope.legal_entity_id, &key],
        ).await.map_err(|_| ExecutionError::JournalUnavailable)?;
        if old.is_some() {
            return Err(ExecutionError::AlreadyReservedNeedsReconciliation);
        }
        let row = trx.query_one(
            "SELECT payment_ops.reserve_native_execution(
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
            &[
                &scope.tenant_id, &"CREATE_INTENT", &key, &body_hash,
                &scope.payment_reference, &scope.organisation_id,
                &scope.legal_entity_id, &mandate,
                &format!("cp/live-mandate/{mandate_id}"),
                &binding.certification_reference, &binding.merchant_activation_reference,
                &scope.market, &scope.currency, &scope.amount_minor,
            ],
        ).await.map_err(|_| ExecutionError::JournalUnavailable)?;
        let execution_id: Uuid = row.get(0);
        trx.execute(
            "INSERT INTO payment_ops.transaction_outbox
               (tenant_id, responsible_legal_entity_id, execution_id,
                event_type, event_payload, correlation_id)
             VALUES ($1,$2,$3,'payments.native-operation.reserved.v1',
                jsonb_build_object('payment_reference',$4,'kind','CREATE_INTENT'),$5)",
            &[
                &scope.tenant_id, &scope.legal_entity_id, &execution_id,
                &scope.payment_reference, &scope.correlation_id,
            ],
        ).await.map_err(|_| ExecutionError::JournalUnavailable)?;
        trx.commit().await.map_err(|_| ExecutionError::JournalUnavailable)?;
        // The irreversible send happens only AFTER durable commit. A timeout
        // can mean remote success; do not silently resend on retry.
        let observed = self.transport.execute(operation, body, key).await;
        self.persist_observation(scope, execution_id, &observed).await?;
        match observed {
            Ok(ProviderObservation { native_reference, provider_status }) =>
                Ok(ExecutionResult::ProviderObserved {
                    execution_id, native_reference, status: provider_status,
                }),
            Err(HyperSwitchError::Unavailable) =>
                Err(ExecutionError::ProviderUnavailableNeedsReconciliation),
            Err(HyperSwitchError::Rejected) =>
                Err(ExecutionError::ProviderRejectedNeedsReconciliation),
            Err(_) => Err(ExecutionError::InvalidProviderResponseNeedsReconciliation),
        }
    }

    async fn persist_observation(
        &self,
        scope: &NativePaymentContext,
        execution: Uuid,
        observed: &Result<ProviderObservation, HyperSwitchError>,
    ) -> Result<(), ExecutionError> {
        let (mut client, connection) = tokio_postgres::connect(&self.database_url, NoTls)
            .await.map_err(|_| ExecutionError::JournalUnavailable)?;
        tokio::spawn(async move {
            if connection.await.is_err() {
                tracing::warn!(event = "payment.pg.observation_connection_lost");
            }
        });
        let trx = client.transaction().await.map_err(|_| ExecutionError::JournalUnavailable)?;
        trx.execute("SELECT set_config('baobab.tenant_id',$1,true)", &[&scope.tenant_id])
            .await.map_err(|_| ExecutionError::JournalUnavailable)?;
        trx.execute("SELECT set_config('baobab.responsible_legal_entity_id',$1,true)", &[&scope.legal_entity_id])
            .await.map_err(|_| ExecutionError::JournalUnavailable)?;
        let (kind, evidence) = match observed {
            Ok(ref state) => (
                "payments.provider-response.observed.v1",
                json!({"provider_reference":state.native_reference, "provider_status":state.provider_status}),
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
                &scope.tenant_id, &scope.legal_entity_id, &execution,
                &kind, &evidence, &scope.correlation_id,
            ],
        ).await.map_err(|_| ExecutionError::JournalUnavailable)?;
        trx.commit().await.map_err(|_| ExecutionError::JournalUnavailable)?;
        Ok(())
    }
}
