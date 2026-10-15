//! P0-04B Gate D1: the public Payment API (Shared `payments/v1`) on the native
//! backend. Every consequential money movement goes through the single
//! enforcement point in `money_movement_execution`; this module only turns a
//! contract request into the instruction Payments itself computes, and the
//! journal back into contract resources.
//!
//! The instruction is built from Payments' own records (the intent row) and the
//! tenant's merchant binding, never from the request body: a caller chooses
//! only which of its own payments to act on, the amount where the contract
//! allows it, and the opaque payment method token. Payment and refund state is
//! never stored; it is read from the movements the provider confirmed.

use std::sync::Arc;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tokio_postgres::{Client, NoTls, Transaction};
use uuid::Uuid;

use crate::cp_authority::{CpAuthorityError, CpRuntimeContext, IamCpAuthority};
use crate::domain::{
    CancelPaymentRequest, CaptureMethod, CapturePaymentRequest, ConfirmPaymentIntentRequest,
    CreatePaymentIntentRequest, CreateRefundRequest,
};
use crate::hyperswitch::NativeOperation;
use crate::legal_actor::{
    LegalActorGateError, PaymentLegalOperation, PaymentProviderLegalReadiness,
};
use crate::merchant_binding::{MerchantBinding, MerchantBindings};
use crate::money_movement::{Instruction, ORIGINAL_PAYMENT_METHOD, OperationClass};
use crate::money_movement_execution::{
    LiveDocumentaryVeto, MoneyMovementCommand, MoneyMovementError, MoneyMovementExecutor,
    MoneyMovementOutcome,
};
use crate::native_execution::{
    ExecutionError, NativePaymentContext, NativeTransactionService, ProviderBinding,
};
use crate::service::{Call, Outcome, Refusal, decode, hash, parse_body, timestamp};

/// The provider environment every native movement is journalled under.
const PROVIDER_ENVIRONMENT: &str = "SANDBOX";

pub struct NativePaymentApi {
    database_url: String,
    executor: MoneyMovementExecutor,
    intents: NativeTransactionService,
    cp: IamCpAuthority,
    bindings: MerchantBindings,
}

/// One intent as Payments recorded it.
#[derive(Debug, Clone)]
struct IntentRow {
    payment_intent_id: String,
    payment_reference: String,
    operating_organisation_id: Uuid,
    market: String,
    currency: String,
    amount_minor: i64,
    merchant_profile_reference: String,
    customer_reference: String,
    market_id: Option<String>,
    source_engine: String,
    source_reference: String,
    correlation_id: String,
    description: Option<String>,
    created_at: OffsetDateTime,
}

/// What the journal says happened to an intent's payment.
#[derive(Debug, Clone, Default)]
struct PaymentState {
    created: bool,
    cancelled_unauthorised: bool,
    authorisation: Option<String>,
    authorised_at: Option<OffsetDateTime>,
    captured: i64,
    refunded: i64,
    voided: bool,
    events: i64,
    updated_at: Option<OffsetDateTime>,
}

fn digest(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"|");
    }
    hex::encode(hasher.finalize())
}

/// Deterministic identifiers: a retry with the same Idempotency-Key computes
/// the same intent, payment, refund and operation references, so the same
/// instruction and fingerprint, and the enforcement point replays or
/// reconciles instead of moving money twice.
fn minted(prefix: &str, parts: &[&str]) -> String {
    format!("{prefix}_{}", &digest(parts)[..32])
}

fn refusal(status: u16, code: &'static str, detail: &str) -> Refusal {
    Refusal::new(status, code, detail)
}

fn retryable(mut r: Refusal) -> Refusal {
    r.retryable = true;
    r
}

fn not_found(what: &str) -> Refusal {
    refusal(
        404,
        "PAYMENT_RESOURCE_NOT_FOUND",
        &format!("no such {what} for this tenant"),
    )
}

fn journal_unavailable() -> Refusal {
    retryable(refusal(
        503,
        "PAYMENTS_UNAVAILABLE",
        "the payment journal is unavailable",
    ))
}

/// Provider readiness for intent creation: a current activation for this
/// merchant to authorise, which the journal itself checked a moment before.
struct ActivationReadiness(bool);

impl PaymentProviderLegalReadiness for ActivationReadiness {
    fn require_ready(
        &self,
        _operation: &PaymentLegalOperation<'_>,
        _mandate_id: &str,
    ) -> Result<(), LegalActorGateError> {
        if self.0 {
            Ok(())
        } else {
            Err(LegalActorGateError::NotAuthorised)
        }
    }
}

fn movement_refusal(class: OperationClass, error: &MoneyMovementError) -> Refusal {
    match error {
        MoneyMovementError::NotActivated => refusal(
            503,
            "PAYMENT_OPERATION_NOT_ACTIVATED",
            "this money movement is not activated",
        ),
        MoneyMovementError::InvalidInstruction => refusal(
            422,
            "INVALID_PAYMENT_INSTRUCTION",
            "the payment instruction is not valid",
        ),
        MoneyMovementError::AuthorityDenied(_) => refusal(
            403,
            "PAYMENT_AUTHORITY_DENIED",
            "the Control Plane did not authorise this money movement",
        ),
        MoneyMovementError::AuthorityUnavailable => retryable(refusal(
            503,
            "PAYMENT_AUTHORITY_UNAVAILABLE",
            "legal authority for this money movement could not be established",
        )),
        MoneyMovementError::DocumentaryHold => refusal(
            403,
            "PAYMENT_DOCUMENTARY_HOLD",
            "a documentary restriction holds this money movement",
        ),
        MoneyMovementError::ProviderNotReady => refusal(
            503,
            "PAYMENT_PROVIDER_NOT_READY",
            "the merchant has no current provider activation for this movement",
        ),
        MoneyMovementError::FinancialLimit => match class {
            OperationClass::Capture => refusal(
                422,
                "CAPTURE_EXCEEDS_AUTHORIZED",
                "the capture exceeds the authorised amount",
            ),
            _ => refusal(
                422,
                "REFUND_EXCEEDS_CAPTURED",
                "the refund exceeds what remains captured",
            ),
        },
        MoneyMovementError::StateConflict => refusal(
            409,
            "INVALID_PAYMENT_STATE",
            "the payment's state does not allow this movement",
        ),
        MoneyMovementError::IdempotencyConflict => refusal(
            409,
            "IDEMPOTENCY_KEY_REUSED",
            "the Idempotency-Key was used for a different request",
        ),
        MoneyMovementError::JournalUnavailable => journal_unavailable(),
        MoneyMovementError::OutcomeUnknown { .. } => retryable(refusal(
            202,
            "PAYMENT_OUTCOME_PENDING",
            "the provider's answer is not known yet; retry with the same Idempotency-Key to reconcile",
        )),
        MoneyMovementError::ProviderRejected { .. } => refusal(
            422,
            "PAYMENT_PROVIDER_REJECTED",
            "the payment provider rejected this movement",
        ),
    }
}

impl NativePaymentApi {
    pub fn new(
        database_url: String,
        executor: MoneyMovementExecutor,
        intents: NativeTransactionService,
        cp: IamCpAuthority,
        bindings: MerchantBindings,
    ) -> Self {
        Self {
            database_url,
            executor,
            intents,
            cp,
            bindings,
        }
    }

    fn binding(&self, tenant_id: &str) -> Result<&MerchantBinding, Refusal> {
        self.bindings.get(tenant_id).ok_or_else(|| {
            refusal(
                503,
                "MERCHANT_NOT_ACTIVATED",
                "this tenant has no activated merchant on the native backend",
            )
        })
    }

    async fn connect(&self) -> Result<Client, Refusal> {
        let (client, connection) = tokio_postgres::connect(&self.database_url, NoTls)
            .await
            .map_err(|_| journal_unavailable())?;
        tokio::spawn(async move {
            if connection.await.is_err() {
                tracing::warn!(event = "payment.pg.native_api_connection_lost");
            }
        });
        Ok(client)
    }

    async fn begin<'c>(
        client: &'c mut Client,
        b: &MerchantBinding,
    ) -> Result<Transaction<'c>, Refusal> {
        let trx = client
            .transaction()
            .await
            .map_err(|_| journal_unavailable())?;
        trx.execute(
            "SELECT set_config('baobab.tenant_id',$1,true), set_config('baobab.responsible_legal_entity_id',$2,true)",
            &[&b.tenant_id, &b.responsible_legal_entity_id],
        )
        .await
        .map_err(|_| journal_unavailable())?;
        Ok(trx)
    }

    /// Records the request under its key, or checks a repeat is identical.
    /// Returns the resource the first request named.
    #[allow(clippy::too_many_arguments)]
    async fn claim(
        &self,
        client: &mut Client,
        b: &MerchantBinding,
        operation: &str,
        call: &Call<'_>,
        request: &Value,
        resource_id: &str,
        reason: Option<&str>,
    ) -> Result<String, Refusal> {
        let trx = Self::begin(client, b).await?;
        let request_hash = hash(request);
        let row = trx
            .query_one(
                "WITH ins AS (
                   INSERT INTO payment_ops.native_api_request (tenant_id, responsible_legal_entity_id,
                       operation, idempotency_key, request_sha256, resource_id, reason)
                   VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING RETURNING resource_id, request_sha256)
                 SELECT resource_id, request_sha256 FROM ins
                 UNION ALL
                 SELECT resource_id, request_sha256 FROM payment_ops.native_api_request
                  WHERE tenant_id=$1 AND responsible_legal_entity_id=$2 AND operation=$3 AND idempotency_key=$4
                 LIMIT 1",
                &[
                    &b.tenant_id,
                    &b.responsible_legal_entity_id,
                    &operation,
                    &call.idempotency_key,
                    &request_hash,
                    &resource_id,
                    &reason,
                ],
            )
            .await
            .map_err(|_| journal_unavailable())?;
        trx.commit().await.map_err(|_| journal_unavailable())?;
        if row.get::<_, String>(1) != request_hash {
            return Err(refusal(
                409,
                "IDEMPOTENCY_KEY_REUSED",
                "the Idempotency-Key was used for a different request",
            ));
        }
        Ok(row.get(0))
    }

    async fn load_intent(
        &self,
        client: &mut Client,
        b: &MerchantBinding,
        payment_intent_id: Option<&str>,
        payment_reference: Option<&str>,
    ) -> Result<Option<IntentRow>, Refusal> {
        let trx = Self::begin(client, b).await?;
        let row = trx
            .query_opt(
                "SELECT payment_intent_id, payment_reference, operating_organisation_id, market_code,
                        currency_code, amount_minor, merchant_profile_reference, customer_reference, market_id,
                        source_engine, source_reference, correlation_id, description, created_at
                   FROM payment_ops.native_intent
                  WHERE ($1::text IS NULL OR payment_intent_id=$1) AND ($2::text IS NULL OR payment_reference=$2)",
                &[&payment_intent_id, &payment_reference],
            )
            .await
            .map_err(|_| journal_unavailable())?;
        trx.commit().await.map_err(|_| journal_unavailable())?;
        Ok(row.map(|r| IntentRow {
            payment_intent_id: r.get(0),
            payment_reference: r.get(1),
            operating_organisation_id: r.get(2),
            market: r.get(3),
            currency: r.get(4),
            amount_minor: r.get(5),
            merchant_profile_reference: r.get(6),
            customer_reference: r.get(7),
            market_id: r.get(8),
            source_engine: r.get(9),
            source_reference: r.get(10),
            correlation_id: r.get(11),
            description: r.get(12),
            created_at: r.get(13),
        }))
    }

    async fn state(
        &self,
        client: &mut Client,
        b: &MerchantBinding,
        intent: &IntentRow,
    ) -> Result<PaymentState, Refusal> {
        let trx = Self::begin(client, b).await?;
        let r = trx
            .query_one(
                "WITH moves AS (
                   SELECT e.execution_id, e.operation_kind, e.amount_minor,
                          payment_ops.execution_outcome(e.execution_id) AS outcome
                     FROM payment_ops.native_execution e
                    WHERE e.canonical_payment_reference=$1 OR e.parent_reference=$1),
                 seen AS (
                   SELECT o.observed_at, o.outcome FROM payment_ops.provider_observation o
                    WHERE o.execution_id IN (SELECT execution_id FROM moves))
                 SELECT
                   EXISTS (SELECT 1 FROM moves WHERE operation_kind='CREATE_INTENT'),
                   EXISTS (SELECT 1 FROM payment_ops.native_intent_cancellation c WHERE c.payment_intent_id=$2),
                   (SELECT outcome FROM moves WHERE operation_kind='AUTHORIZE' AND outcome IS NOT NULL
                     ORDER BY (outcome='CONFIRMED') DESC LIMIT 1),
                   (SELECT min(o.observed_at) FROM payment_ops.provider_observation o
                     JOIN moves m ON m.execution_id=o.execution_id
                    WHERE m.operation_kind='AUTHORIZE' AND o.outcome='CONFIRMED'),
                   coalesce((SELECT sum(amount_minor) FROM moves WHERE operation_kind='CAPTURE' AND outcome='CONFIRMED'), 0)::bigint,
                   coalesce((SELECT sum(amount_minor) FROM moves WHERE operation_kind='REFUND' AND outcome='CONFIRMED'), 0)::bigint,
                   EXISTS (SELECT 1 FROM moves WHERE operation_kind='CANCEL' AND outcome='CONFIRMED'),
                   (SELECT count(*) FROM seen WHERE outcome IN ('CONFIRMED','REJECTED')),
                   (SELECT max(observed_at) FROM seen)",
                &[&intent.payment_reference, &intent.payment_intent_id],
            )
            .await
            .map_err(|_| journal_unavailable())?;
        trx.commit().await.map_err(|_| journal_unavailable())?;
        Ok(PaymentState {
            created: r.get(0),
            cancelled_unauthorised: r.get(1),
            authorisation: r.get(2),
            authorised_at: r.get(3),
            captured: r.get(4),
            refunded: r.get(5),
            voided: r.get(6),
            events: r.get(7),
            updated_at: r.get(8),
        })
    }

    fn provider_result(reference: Option<&str>) -> Value {
        let mut provider = json!({"kind": "HYPERSWITCH", "simulated": false});
        if let Some(reference) = reference {
            provider["provider_reference"] = json!(reference);
        }
        provider
    }

    fn intent_body(b: &MerchantBinding, i: &IntentRow, s: &PaymentState) -> Value {
        let authorised = s.authorisation.as_deref() == Some("CONFIRMED");
        let status = if s.cancelled_unauthorised || s.voided {
            "CANCELLED"
        } else if s.authorisation.as_deref() == Some("REJECTED") {
            "FAILED"
        } else if authorised && s.captured > 0 {
            "SUCCEEDED"
        } else if authorised {
            "REQUIRES_CAPTURE"
        } else {
            "REQUIRES_PAYMENT_METHOD"
        };
        let mut context = json!({
            "tenant_id": b.tenant_id,
            "legal_entity_id": b.responsible_legal_entity_id,
            "platform_account_id": i.customer_reference,
            "currency": i.currency,
            "source_engine": i.source_engine,
            "source_reference": i.source_reference,
            "correlation_id": i.correlation_id,
        });
        if let Some(market_id) = &i.market_id {
            context["market_id"] = json!(market_id);
        }
        let mut body = json!({
            "payment_intent_id": i.payment_intent_id,
            "context": context,
            "amount": {"currency": i.currency, "amount_minor": i.amount_minor},
            "capture_method": "MANUAL",
            "status": status,
            "provider": Self::provider_result(Some(&i.payment_reference)),
            "version": 1 + s.events + i64::from(s.cancelled_unauthorised),
            "created_at": timestamp(i.created_at),
            "updated_at": timestamp(s.updated_at.unwrap_or(i.created_at).max(i.created_at)),
        });
        if authorised || s.authorisation.as_deref() == Some("REJECTED") {
            body["payment_id"] = json!(i.payment_reference);
        }
        if let Some(description) = &i.description {
            body["description"] = json!(description);
        }
        body
    }

    fn payment_body(b: &MerchantBinding, i: &IntentRow, s: &PaymentState) -> Value {
        let status = if s.voided {
            "CANCELLED"
        } else if s.authorisation.as_deref() == Some("REJECTED") {
            "FAILED"
        } else if s.refunded > 0 && s.refunded >= s.captured {
            "REFUNDED"
        } else if s.refunded > 0 {
            "PARTIALLY_REFUNDED"
        } else if s.captured > 0 {
            "CAPTURED"
        } else {
            "AUTHORIZED"
        };
        let created = s.authorised_at.unwrap_or(i.created_at);
        json!({
            "payment_id": i.payment_reference,
            "payment_intent_id": i.payment_intent_id,
            "tenant_id": b.tenant_id,
            "legal_entity_id": b.responsible_legal_entity_id,
            "amount": {"currency": i.currency, "amount_minor": i.amount_minor},
            "captured_amount_minor": s.captured,
            "refunded_amount_minor": s.refunded,
            "status": status,
            "provider": Self::provider_result(Some(&i.payment_reference)),
            "created_at": timestamp(created),
            "updated_at": timestamp(s.updated_at.unwrap_or(created).max(created)),
        })
    }

    /// The caller's own intent: another engine's, or another tenant's, is
    /// not found.
    async fn own_intent(
        &self,
        client: &mut Client,
        b: &MerchantBinding,
        call_engine: &str,
        payment_intent_id: Option<&str>,
        payment_reference: Option<&str>,
    ) -> Result<(IntentRow, PaymentState), Refusal> {
        let what = if payment_intent_id.is_some() {
            "payment intent"
        } else {
            "payment"
        };
        let intent = self
            .load_intent(client, b, payment_intent_id, payment_reference)
            .await?
            .filter(|i| i.source_engine == call_engine)
            .ok_or_else(|| not_found(what))?;
        let state = self.state(client, b, &intent).await?;
        if !state.created {
            return Err(not_found(what));
        }
        Ok((intent, state))
    }

    async fn runtime_context(&self, b: &MerchantBinding) -> Result<CpRuntimeContext, Refusal> {
        self.cp
            .issue_current_context(
                &b.tenant_id,
                b.operating_organisation_id,
                &b.market,
                &b.responsible_legal_entity_id,
            )
            .await
            .map_err(|e| match e {
                CpAuthorityError::NotAuthorised => refusal(
                    403,
                    "PAYMENT_AUTHORITY_DENIED",
                    "the Control Plane did not issue a context for this merchant",
                ),
                _ => retryable(refusal(
                    503,
                    "PAYMENT_AUTHORITY_UNAVAILABLE",
                    "the Control Plane could not be reached",
                )),
            })
    }

    fn correlation(call: &Call<'_>) -> Uuid {
        Uuid::parse_str(call.correlation_id).unwrap_or_else(|_| Uuid::now_v7())
    }

    #[allow(clippy::too_many_arguments)]
    async fn move_money(
        &self,
        b: &MerchantBinding,
        intent: &IntentRow,
        call: &Call<'_>,
        class: OperationClass,
        subject: &str,
        amount_minor: i64,
        payment_reference: String,
        payment_token: Option<String>,
    ) -> Result<MoneyMovementOutcome, Refusal> {
        let operation_reference = minted(
            "mm",
            &[
                class.as_str(),
                &b.tenant_id,
                &b.responsible_legal_entity_id,
                subject,
                call.idempotency_key,
            ],
        );
        let (parent_reference, destination_reference) = match class {
            OperationClass::Authorize => (None, None),
            OperationClass::Refund => (
                Some(intent.payment_reference.clone()),
                Some(ORIGINAL_PAYMENT_METHOD.to_string()),
            ),
            _ => (Some(intent.payment_reference.clone()), None),
        };
        let command = MoneyMovementCommand {
            class,
            idempotency_key: call.idempotency_key.to_string(),
            operation_reference,
            instruction: Instruction {
                tenant_id: b.tenant_id.clone(),
                operating_organisation_id: intent.operating_organisation_id.to_string(),
                market: intent.market.clone(),
                currency: intent.currency.clone(),
                amount_minor,
                payment_reference,
                parent_reference,
                merchant_profile_reference: intent.merchant_profile_reference.clone(),
                destination_reference,
            },
            responsible_legal_entity_id: b.responsible_legal_entity_id.clone(),
            operating_organisation_id: intent.operating_organisation_id,
            correlation_id: Self::correlation(call),
            payment_token,
        };
        let context = self.runtime_context(b).await?;
        let veto = LiveDocumentaryVeto {
            cp: &self.cp,
            context: &context,
            market: &intent.market,
        };
        self.executor
            .execute(&self.cp, &veto, context.context_id, &command)
            .await
            .map_err(|e| movement_refusal(class, &e))
    }

    async fn activation_is_current(
        &self,
        client: &mut Client,
        b: &MerchantBinding,
    ) -> Result<bool, Refusal> {
        let trx = Self::begin(client, b).await?;
        let activation: Option<Uuid> = trx
            .query_one(
                "SELECT payment_ops.current_activation($1,$2,$3,$4,$5,'AUTHORIZE',$6)",
                &[
                    &b.tenant_id,
                    &b.responsible_legal_entity_id,
                    &b.merchant_profile_reference,
                    &b.market,
                    &b.currency,
                    &PROVIDER_ENVIRONMENT,
                ],
            )
            .await
            .map_err(|_| journal_unavailable())?
            .get(0);
        trx.commit().await.map_err(|_| journal_unavailable())?;
        Ok(activation.is_some())
    }

    /// Creates an intent: Payments' own record plus the provider payment.
    /// No money moves; CP legal-actor and documentary checks still apply.
    pub async fn create_intent(&self, call: &Call<'_>, raw: &[u8]) -> Result<Outcome, Refusal> {
        let value = parse_body(raw)?;
        let req: CreatePaymentIntentRequest = decode(&value, "CreatePaymentIntentRequest")?;
        if req.context.source_engine != call.engine {
            return Err(refusal(
                403,
                "SOURCE_ENGINE_MISMATCH",
                "a workload moves money only for its own engine's obligations",
            ));
        }
        let b = self.binding(&req.context.tenant_id)?;
        let platform_account = req
            .context
            .platform_account_id
            .as_deref()
            .unwrap_or_default();
        if req.context.legal_entity_id != b.responsible_legal_entity_id
            || !b.allows_platform_account(platform_account)
        {
            return Err(refusal(
                403,
                "PAYMENT_CONTEXT_MISMATCH",
                "the legal entity or platform account is not this tenant's merchant",
            ));
        }
        if req.amount.currency != req.context.currency || req.amount.currency != b.currency {
            return Err(refusal(
                422,
                "CURRENCY_MISMATCH",
                "the amount's currency must be the context and merchant currency",
            ));
        }
        if req.capture_method != CaptureMethod::Manual {
            return Err(refusal(
                422,
                "CAPTURE_METHOD_NOT_SUPPORTED",
                "the native backend authorises and captures as separate movements: use MANUAL",
            ));
        }
        let parts = [
            b.tenant_id.as_str(),
            &b.responsible_legal_entity_id,
            call.idempotency_key,
        ];
        let intent_id = minted("payint", &[&["intent"][..], &parts[..]].concat());
        let payment_reference = minted("pay", &[&["payment"][..], &parts[..]].concat());
        let mut client = self.connect().await?;
        let claimed = self
            .claim(&mut client, b, "create:", call, &value, &intent_id, None)
            .await?;
        if claimed != intent_id {
            return Err(journal_unavailable());
        }
        {
            let trx = Self::begin(&mut client, b).await?;
            trx.execute(
                "INSERT INTO payment_ops.native_intent (payment_intent_id, tenant_id, responsible_legal_entity_id,
                     operating_organisation_id, payment_reference, market_code, currency_code, amount_minor,
                     capture_method, merchant_profile_reference, customer_reference, market_id, source_engine,
                     source_reference, correlation_id, description)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'MANUAL',$9,$10,$11,$12,$13,$14,$15)
                 ON CONFLICT (payment_intent_id) DO NOTHING",
                &[
                    &intent_id,
                    &b.tenant_id,
                    &b.responsible_legal_entity_id,
                    &b.operating_organisation_id,
                    &payment_reference,
                    &b.market,
                    &b.currency,
                    &req.amount.amount_minor,
                    &b.merchant_profile_reference,
                    &platform_account,
                    &req.context.market_id,
                    &req.context.source_engine,
                    &req.context.source_reference,
                    &req.context.correlation_id,
                    &req.description,
                ],
            )
            .await
            .map_err(|_| journal_unavailable())?;
            trx.commit().await.map_err(|_| journal_unavailable())?;
        }
        let intent = self
            .load_intent(&mut client, b, Some(&intent_id), None)
            .await?
            .ok_or_else(journal_unavailable)?;
        let before = self.state(&mut client, b, &intent).await?;
        if before.created {
            return Ok(Outcome {
                status: 201,
                body: Self::intent_body(b, &intent, &before),
                replayed: true,
            });
        }
        let ready = ActivationReadiness(self.activation_is_current(&mut client, b).await?);
        let context = NativePaymentContext {
            tenant_id: b.tenant_id.clone(),
            legal_entity_id: b.responsible_legal_entity_id.clone(),
            organisation_id: b.operating_organisation_id,
            payment_reference: payment_reference.clone(),
            market: b.market.clone(),
            currency: b.currency.clone(),
            amount_minor: req.amount.amount_minor,
            correlation_id: Self::correlation(call),
        };
        let binding = ProviderBinding {
            merchant_id: b.provider_merchant_id.clone(),
            certification_reference: b.certification_reference.clone(),
            merchant_activation_reference: b.merchant_activation_reference.clone(),
            provider_engine_instance_id: b.provider_engine_instance_id.clone(),
            profile_id: b.merchant_profile_reference.clone(),
        };
        let body = json!({
            "payment_id": payment_reference,
            "amount": req.amount.amount_minor,
            "currency": b.currency,
            "capture_method": "manual",
            "confirm": false,
            "profile_id": b.merchant_profile_reference,
            "customer_id": platform_account,
        });
        // The CP RUNTIME context is minted for this workload; the legal-actor
        // and documentary checks run against it twice, around the journal.
        let cp_context = self.runtime_context(b).await?;
        let handle = cp_context.context_id.to_string();
        let operation = PaymentLegalOperation {
            context_id: &handle,
            expected_legal_entity_id: &b.responsible_legal_entity_id,
            role: "PAYMENT_BENEFICIARY",
            activity: "PAYMENT_COLLECTION",
            market: &b.market,
            capability: "payment.intent.create",
            operation_reference: &payment_reference,
        };
        let created = self
            .intents
            .execute_with_live_cp(
                &context,
                &cp_context,
                &self.cp,
                &operation,
                &binding,
                call.idempotency_key,
                NativeOperation::Create,
                &body,
                &ready,
            )
            .await;
        match created {
            Ok(_) => {}
            Err(ExecutionError::GovernanceDenied) => {
                return Err(refusal(
                    403,
                    "PAYMENT_AUTHORITY_DENIED",
                    "current legal authority, documentary clearance or provider readiness was not established",
                ));
            }
            Err(ExecutionError::InvalidScope) => {
                return Err(refusal(
                    422,
                    "INVALID_PAYMENT_INSTRUCTION",
                    "the payment intent is not valid",
                ));
            }
            Err(ExecutionError::JournalUnavailable) => return Err(journal_unavailable()),
            Err(_) => {
                return Err(retryable(refusal(
                    503,
                    "PAYMENT_PROVIDER_UNAVAILABLE",
                    "the provider payment could not be confirmed as created",
                )));
            }
        }
        let after = self.state(&mut client, b, &intent).await?;
        Ok(Outcome {
            status: 201,
            body: Self::intent_body(b, &intent, &after),
            replayed: false,
        })
    }

    pub async fn confirm_intent(
        &self,
        call: &Call<'_>,
        intent_id: &str,
        raw: &[u8],
    ) -> Result<Outcome, Refusal> {
        let value = parse_body(raw)?;
        let req: ConfirmPaymentIntentRequest = decode(&value, "ConfirmPaymentIntentRequest")?;
        let b = self.binding(&req.tenant_id)?;
        let mut client = self.connect().await?;
        let (intent, _) = self
            .own_intent(&mut client, b, call.engine, Some(intent_id), None)
            .await?;
        self.claim(
            &mut client,
            b,
            &format!("confirm:{intent_id}"),
            call,
            &value,
            &intent.payment_intent_id,
            None,
        )
        .await?;
        let moved = self
            .move_money(
                b,
                &intent,
                call,
                OperationClass::Authorize,
                &intent.payment_intent_id,
                intent.amount_minor,
                intent.payment_reference.clone(),
                Some(req.payment_method_reference.clone()),
            )
            .await;
        let replayed = matches!(moved, Ok(MoneyMovementOutcome::Replayed { .. }));
        match moved {
            Ok(_) => {}
            // A decline is an answer about the intent, not a refusal.
            Err(r) if r.code == "PAYMENT_PROVIDER_REJECTED" => {}
            Err(r) => return Err(r),
        }
        let state = self.state(&mut client, b, &intent).await?;
        Ok(Outcome {
            status: 200,
            body: Self::intent_body(b, &intent, &state),
            replayed,
        })
    }

    pub async fn capture(
        &self,
        call: &Call<'_>,
        payment_id: &str,
        raw: &[u8],
    ) -> Result<Outcome, Refusal> {
        let value = parse_body(raw)?;
        let req: CapturePaymentRequest = decode(&value, "CapturePaymentRequest")?;
        let b = self.binding(&req.tenant_id)?;
        let mut client = self.connect().await?;
        let (intent, state) = self
            .own_intent(&mut client, b, call.engine, None, Some(payment_id))
            .await?;
        if state.authorisation.as_deref() != Some("CONFIRMED") {
            return Err(not_found("payment"));
        }
        self.claim(
            &mut client,
            b,
            &format!("capture:{payment_id}"),
            call,
            &value,
            &intent.payment_reference,
            None,
        )
        .await?;
        let capture_reference = minted(
            "cap",
            &[
                &b.tenant_id,
                &b.responsible_legal_entity_id,
                payment_id,
                call.idempotency_key,
            ],
        );
        let moved = self
            .move_money(
                b,
                &intent,
                call,
                OperationClass::Capture,
                payment_id,
                req.amount_minor.unwrap_or(intent.amount_minor),
                capture_reference,
                None,
            )
            .await?;
        let state = self.state(&mut client, b, &intent).await?;
        Ok(Outcome {
            status: 200,
            body: Self::payment_body(b, &intent, &state),
            replayed: matches!(moved, MoneyMovementOutcome::Replayed { .. }),
        })
    }

    /// Cancels an intent: a void of its authorisation when it has one (a
    /// money movement), otherwise a journal record that forbids authorising it.
    pub async fn cancel_intent(
        &self,
        call: &Call<'_>,
        intent_id: &str,
        raw: &[u8],
    ) -> Result<Outcome, Refusal> {
        let value = parse_body(raw)?;
        let req: CancelPaymentRequest = decode(&value, "CancelPaymentRequest")?;
        let b = self.binding(&req.tenant_id)?;
        let mut client = self.connect().await?;
        let (intent, state) = self
            .own_intent(&mut client, b, call.engine, Some(intent_id), None)
            .await?;
        self.claim(
            &mut client,
            b,
            &format!("cancel:{intent_id}"),
            call,
            &value,
            &intent.payment_intent_id,
            None,
        )
        .await?;
        let mut replayed = false;
        if state.authorisation.as_deref() == Some("CONFIRMED") {
            let void_reference = minted(
                "void",
                &[
                    &b.tenant_id,
                    &b.responsible_legal_entity_id,
                    intent_id,
                    call.idempotency_key,
                ],
            );
            let moved = self
                .move_money(
                    b,
                    &intent,
                    call,
                    OperationClass::Void,
                    intent_id,
                    intent.amount_minor,
                    void_reference,
                    None,
                )
                .await?;
            replayed = matches!(moved, MoneyMovementOutcome::Replayed { .. });
        } else if state.cancelled_unauthorised {
            replayed = true;
        } else {
            let trx = Self::begin(&mut client, b).await?;
            let inserted = trx
                .execute(
                    "INSERT INTO payment_ops.native_intent_cancellation
                       (payment_intent_id, tenant_id, responsible_legal_entity_id, reason)
                     VALUES ($1,$2,$3,$4)",
                    &[
                        &intent.payment_intent_id,
                        &b.tenant_id,
                        &b.responsible_legal_entity_id,
                        &req.reason,
                    ],
                )
                .await;
            match inserted {
                Ok(_) => trx.commit().await.map_err(|_| journal_unavailable())?,
                Err(e) if e.code().map(|c| c.code()) == Some("BPS01") => {
                    return Err(refusal(
                        409,
                        "INVALID_PAYMENT_STATE",
                        "the payment intent is being authorised: void it instead",
                    ));
                }
                Err(_) => return Err(journal_unavailable()),
            }
        }
        let state = self.state(&mut client, b, &intent).await?;
        Ok(Outcome {
            status: 200,
            body: Self::intent_body(b, &intent, &state),
            replayed,
        })
    }

    pub async fn refund(
        &self,
        call: &Call<'_>,
        payment_id: &str,
        raw: &[u8],
    ) -> Result<Outcome, Refusal> {
        let value = parse_body(raw)?;
        let req: CreateRefundRequest = decode(&value, "CreateRefundRequest")?;
        let b = self.binding(&req.tenant_id)?;
        let mut client = self.connect().await?;
        let (intent, state) = self
            .own_intent(&mut client, b, call.engine, None, Some(payment_id))
            .await?;
        if state.authorisation.as_deref() != Some("CONFIRMED") {
            return Err(not_found("payment"));
        }
        let refund_id = minted(
            "refund",
            &[
                &b.tenant_id,
                &b.responsible_legal_entity_id,
                payment_id,
                call.idempotency_key,
            ],
        );
        self.claim(
            &mut client,
            b,
            &format!("refund:{payment_id}"),
            call,
            &value,
            &refund_id,
            Some(&req.reason),
        )
        .await?;
        let moved = self
            .move_money(
                b,
                &intent,
                call,
                OperationClass::Refund,
                payment_id,
                req.amount_minor,
                refund_id.clone(),
                None,
            )
            .await?;
        let (replayed, provider_status) = match &moved {
            MoneyMovementOutcome::Executed {
                provider_status, ..
            } => (false, provider_status.clone()),
            MoneyMovementOutcome::Replayed {
                provider_status, ..
            } => (true, provider_status.clone()),
        };
        // A refund the provider accepted but has not completed is accepted
        // for processing, never reported as succeeded.
        if provider_status != "succeeded" {
            return Err(retryable(refusal(
                202,
                "REFUND_PENDING",
                "the provider accepted the refund and has not completed it yet",
            )));
        }
        let at = self.refund_recorded_at(&mut client, b, &refund_id).await?;
        Ok(Outcome {
            status: 201,
            body: json!({
                "refund_id": refund_id,
                "payment_id": intent.payment_reference,
                "tenant_id": b.tenant_id,
                "amount": {"currency": intent.currency, "amount_minor": req.amount_minor},
                "status": "SUCCEEDED",
                "reason": req.reason,
                "provider": Self::provider_result(Some(&refund_id)),
                "created_at": timestamp(at),
            }),
            replayed,
        })
    }

    async fn refund_recorded_at(
        &self,
        client: &mut Client,
        b: &MerchantBinding,
        refund_id: &str,
    ) -> Result<OffsetDateTime, Refusal> {
        let trx = Self::begin(client, b).await?;
        let at: OffsetDateTime = trx
            .query_one(
                "SELECT min(recorded_at) FROM payment_ops.native_execution
                  WHERE operation_kind='REFUND' AND canonical_payment_reference=$1",
                &[&refund_id],
            )
            .await
            .map_err(|_| journal_unavailable())?
            .try_get::<_, Option<OffsetDateTime>>(0)
            .ok()
            .flatten()
            .ok_or_else(journal_unavailable)?;
        trx.commit().await.map_err(|_| journal_unavailable())?;
        Ok(at)
    }

    pub async fn intent(
        &self,
        engine: &str,
        tenant_id: &str,
        intent_id: &str,
    ) -> Result<Value, Refusal> {
        let b = self
            .bindings
            .get(tenant_id)
            .ok_or_else(|| not_found("payment intent"))?;
        let mut client = self.connect().await?;
        let (intent, state) = self
            .own_intent(&mut client, b, engine, Some(intent_id), None)
            .await?;
        Ok(Self::intent_body(b, &intent, &state))
    }

    pub async fn payment(
        &self,
        engine: &str,
        tenant_id: &str,
        payment_id: &str,
    ) -> Result<Value, Refusal> {
        let b = self
            .bindings
            .get(tenant_id)
            .ok_or_else(|| not_found("payment"))?;
        let mut client = self.connect().await?;
        let (intent, state) = self
            .own_intent(&mut client, b, engine, None, Some(payment_id))
            .await?;
        if state.authorisation.is_none() {
            return Err(not_found("payment"));
        }
        Ok(Self::payment_body(b, &intent, &state))
    }
}

/// Shared ownership for the HTTP layer.
pub type SharedNativeApi = Arc<NativePaymentApi>;
