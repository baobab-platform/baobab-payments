//! P0-04B Payments (ADR-SHARED-035 §5): the single enforcement point for every
//! consequential money movement. Payments and a provider cannot share one
//! database transaction, so this proves a protocol instead of claiming
//! distributed atomicity:
//!
//! 1. resolve current CP authority for the exact instruction (attempt 1);
//! 2. clear the documentary veto;
//! 3. durably reserve the movement with that decision (the journal checks the
//!    decision, provider readiness and Payments' own financial limits);
//! 4. resolve CP authority again at the execution point (attempt n >= 2);
//! 5. durably record the dispatch with that decision, then send exactly once;
//! 6. record what the provider said, or that nobody knows yet;
//! 7. settle an unknown outcome by reading the provider back, never by
//!    re-sending; re-send only after the provider proved it has no record.
//!
//! Every step fails closed. Payouts are refused before any I/O.

use std::future::Future;

use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio_postgres::{Client, NoTls, Transaction};
use uuid::Uuid;

use crate::cp_authority::{CpAuthorityError, CpRuntimeContext, IamCpAuthority};
use crate::hyperswitch::{
    HyperSwitchError, HyperSwitchTransport, NativeOperation, ProviderRecord, RecordKind,
};
use crate::legal_actor::PaymentLegalOperation;
use crate::money_movement::{
    AuthorizedDecision, DecisionRefusal, Instruction, OperationClass, authority_request,
    fingerprint, require_authorized,
};

/// CP legal authority for one money movement (`payments/v2`).
pub trait MoneyMovementAuthority: Sync {
    fn resolve(
        &self,
        request: &Value,
    ) -> impl Future<Output = Result<Value, CpAuthorityError>> + Send;
}

impl MoneyMovementAuthority for IamCpAuthority {
    fn resolve(
        &self,
        request: &Value,
    ) -> impl Future<Output = Result<Value, CpAuthorityError>> + Send {
        self.resolve_money_movement(request)
    }
}

/// The negative-only documentary veto. Clearance is never a grant.
pub trait DocumentaryVeto: Sync {
    fn clear(
        &self,
        class: OperationClass,
        operation_reference: &str,
    ) -> impl Future<Output = Result<(), CpAuthorityError>> + Send;
}

/// The live CP documentary assessment for the movement's own RUNTIME context.
pub struct LiveDocumentaryVeto<'a> {
    pub cp: &'a IamCpAuthority,
    pub context: &'a CpRuntimeContext,
    pub market: &'a str,
}

impl DocumentaryVeto for LiveDocumentaryVeto<'_> {
    fn clear(
        &self,
        class: OperationClass,
        operation_reference: &str,
    ) -> impl Future<Output = Result<(), CpAuthorityError>> + Send {
        let context_id = self.context.context_id.to_string();
        let operation_reference = operation_reference.to_owned();
        async move {
            let operation = PaymentLegalOperation {
                context_id: &context_id,
                expected_legal_entity_id: &self.context.expected_legal_entity_id,
                role: "PAYMENT_BENEFICIARY",
                activity: class.activity(),
                market: self.market,
                capability: class.capability().ok_or(CpAuthorityError::NotAuthorised)?,
                operation_reference: &operation_reference,
            };
            self.cp.verify_restriction(self.context, &operation).await
        }
    }
}

/// One money movement as Payments computed it. The instruction is built from
/// server-verified values; the payment token is an opaque PSP token, never
/// card or bank data.
#[derive(Debug, Clone)]
pub struct MoneyMovementCommand {
    pub class: OperationClass,
    pub idempotency_key: String,
    pub operation_reference: String,
    pub instruction: Instruction,
    pub responsible_legal_entity_id: String,
    pub operating_organisation_id: Uuid,
    pub correlation_id: Uuid,
    pub payment_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoneyMovementOutcome {
    /// The provider confirmed this movement in this call.
    Executed {
        execution_id: Uuid,
        provider_reference: String,
        provider_status: String,
    },
    /// The same instruction was already confirmed; nothing was sent again.
    Replayed {
        execution_id: Uuid,
        provider_reference: String,
        provider_status: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoneyMovementError {
    /// The class is catalogued but not activated (payout).
    NotActivated,
    InvalidInstruction,
    /// CP answered, and not with a usable AUTHORIZED decision.
    AuthorityDenied(String),
    /// CP could not be reached or its answer could not be trusted.
    AuthorityUnavailable,
    DocumentaryHold,
    ProviderNotReady,
    /// Payments' own financial approval refused (balance or limit).
    FinancialLimit,
    /// The payment's state forbids this movement.
    StateConflict,
    IdempotencyConflict,
    JournalUnavailable,
    /// Dispatched, and nobody knows yet whether money moved. Never re-send:
    /// reconcile.
    OutcomeUnknown {
        execution_id: Uuid,
    },
    ProviderRejected {
        execution_id: Uuid,
    },
}

fn journal_error(err: &tokio_postgres::Error) -> MoneyMovementError {
    match err.code().map(|c| c.code()) {
        Some("BPA01") => MoneyMovementError::AuthorityDenied("EXPIRED_BEFORE_COMMIT".into()),
        Some("BPR01") => MoneyMovementError::ProviderNotReady,
        Some("BPF01") => MoneyMovementError::FinancialLimit,
        Some("BPS01" | "BPS02") => MoneyMovementError::StateConflict,
        Some("BPP01") => MoneyMovementError::NotActivated,
        Some("23505") => MoneyMovementError::IdempotencyConflict,
        _ => MoneyMovementError::JournalUnavailable,
    }
}

/// What a provider response or read-back means for the journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Confirmed,
    Rejected,
    Unknown,
    Absent,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "CONFIRMED",
            Self::Rejected => "REJECTED",
            Self::Unknown => "UNKNOWN",
            Self::Absent => "ABSENT",
        }
    }
}

/// A dispatch response: a confirmed status for this class, an explicit
/// failure, or (anything else, including pending) unknown.
fn classify_response(class: OperationClass, status: &str) -> Outcome {
    match (class, status) {
        (_, "failed") => Outcome::Rejected,
        (OperationClass::Authorize, "requires_capture" | "succeeded" | "partially_captured") => {
            Outcome::Confirmed
        }
        (OperationClass::Authorize, "requires_payment_method" | "cancelled") => Outcome::Rejected,
        (
            OperationClass::Capture,
            "succeeded" | "partially_captured" | "partially_captured_and_capturable",
        ) => Outcome::Confirmed,
        (OperationClass::Void, "cancelled") => Outcome::Confirmed,
        (OperationClass::Refund, "succeeded" | "pending" | "review") => Outcome::Confirmed,
        _ => Outcome::Unknown,
    }
}

struct Existing {
    execution_id: Uuid,
    fingerprint: Option<String>,
    request_hash: String,
}

/// Binds everything the provider will be sent beyond the instruction (the
/// payment token) to the durable reservation, so a replay or a re-send after
/// ABSENT can never charge another payment source under the same key.
fn request_hash(fingerprint: &str, payment_token: Option<&str>) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(fingerprint.as_bytes());
    hasher.update(b"|payment_token|");
    if let Some(token) = payment_token {
        hasher.update(Sha256::digest(token.as_bytes()));
    }
    hex::encode(hasher.finalize())
}

pub struct MoneyMovementExecutor {
    database_url: String,
    transport: HyperSwitchTransport,
    engine_instance_id: String,
    provider_environment: &'static str,
}

fn key_is_valid(key: &str) -> bool {
    (16..=128).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

fn record_reference_is_valid(id: &str) -> bool {
    (3..=120).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

impl MoneyMovementExecutor {
    /// Local synthetic integration only: PostgreSQL on loopback. Protected
    /// environments need a verified TLS connector and an approved activation
    /// change, never this constructor.
    pub fn local_integration(
        database_url: String,
        transport: HyperSwitchTransport,
        environment: &str,
        engine_instance_id: String,
    ) -> Result<Self, MoneyMovementError> {
        if !matches!(environment, "test" | "development" | "integration") {
            return Err(MoneyMovementError::NotActivated);
        }
        let address: tokio_postgres::Config = database_url
            .parse()
            .map_err(|_| MoneyMovementError::InvalidInstruction)?;
        if address.get_hosts().is_empty()
            || address.get_hosts().iter().any(|h| {
                !matches!(h, tokio_postgres::config::Host::Tcp(host)
                    if host == "127.0.0.1" || host == "localhost" || host == "::1")
            })
            || !engine_instance_id.starts_with("ei_")
        {
            return Err(MoneyMovementError::InvalidInstruction);
        }
        Ok(Self {
            database_url,
            transport,
            engine_instance_id,
            provider_environment: "SANDBOX",
        })
    }

    pub async fn execute<A: MoneyMovementAuthority, D: DocumentaryVeto>(
        &self,
        authority: &A,
        documentary: &D,
        context_id: Uuid,
        command: &MoneyMovementCommand,
    ) -> Result<MoneyMovementOutcome, MoneyMovementError> {
        // Collection authority is never disbursement authority: refused
        // before CP, the journal or the provider is touched.
        if !command.class.activatable() {
            return Err(MoneyMovementError::NotActivated);
        }
        self.validate(command)?;
        let fp = fingerprint(
            command.class,
            &command.operation_reference,
            &command.instruction,
        );
        let mut client = self.connect().await?;
        if let Some(existing) = self.lookup(&mut client, command).await? {
            return self
                .resume(
                    authority,
                    documentary,
                    context_id,
                    command,
                    &fp,
                    existing,
                    &mut client,
                )
                .await;
        }
        let first = self.decide(authority, context_id, command, 1).await?;
        documentary
            .clear(command.class, &command.operation_reference)
            .await
            .map_err(|_| MoneyMovementError::DocumentaryHold)?;
        let execution_id = match self.reserve(&mut client, command, &fp, &first).await? {
            Some(id) => id,
            // A concurrent worker reserved the same key first.
            None => {
                let existing = self
                    .lookup(&mut client, command)
                    .await?
                    .ok_or(MoneyMovementError::JournalUnavailable)?;
                return self
                    .resume(
                        authority,
                        documentary,
                        context_id,
                        command,
                        &fp,
                        existing,
                        &mut client,
                    )
                    .await;
            }
        };
        self.dispatch(
            authority,
            documentary,
            context_id,
            command,
            execution_id,
            2,
            &mut client,
        )
        .await
    }

    fn validate(&self, c: &MoneyMovementCommand) -> Result<(), MoneyMovementError> {
        let i = &c.instruction;
        let tenant_ok = i.tenant_id.starts_with("tn_")
            && i.tenant_id.len() <= 63
            && i.tenant_id[3..].len() >= 3
            && i.tenant_id[3..]
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
        let references_ok = record_reference_is_valid(&i.payment_reference)
            && i.parent_reference
                .as_deref()
                .is_none_or(record_reference_is_valid);
        let token_ok = match (c.class, c.payment_token.as_deref()) {
            (OperationClass::Authorize, Some(t)) => {
                (3..=256).contains(&t.len())
                    && t.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-.:".contains(&b))
            }
            (OperationClass::Authorize, None) => true,
            (_, token) => token.is_none(),
        };
        if !i.shape_is_valid(c.class)
            || !tenant_ok
            || !references_ok
            || !token_ok
            || !key_is_valid(&c.idempotency_key)
            || i.operating_organisation_id != c.operating_organisation_id.to_string()
            || !(3..=160).contains(&c.responsible_legal_entity_id.trim().len())
        {
            return Err(MoneyMovementError::InvalidInstruction);
        }
        Ok(())
    }

    async fn connect(&self) -> Result<Client, MoneyMovementError> {
        let (client, connection) = tokio_postgres::connect(&self.database_url, NoTls)
            .await
            .map_err(|_| MoneyMovementError::JournalUnavailable)?;
        tokio::spawn(async move {
            if connection.await.is_err() {
                tracing::warn!(event = "payment.pg.money_movement_connection_lost");
            }
        });
        Ok(client)
    }

    async fn begin<'c>(
        client: &'c mut Client,
        c: &MoneyMovementCommand,
    ) -> Result<Transaction<'c>, MoneyMovementError> {
        let trx = client
            .transaction()
            .await
            .map_err(|_| MoneyMovementError::JournalUnavailable)?;
        // Trusted, transaction-local actor scope; FORCE RLS binds every row.
        trx.execute(
            "SELECT set_config('baobab.tenant_id',$1,true), set_config('baobab.responsible_legal_entity_id',$2,true)",
            &[&c.instruction.tenant_id, &c.responsible_legal_entity_id],
        )
        .await
        .map_err(|_| MoneyMovementError::JournalUnavailable)?;
        Ok(trx)
    }

    async fn lookup(
        &self,
        client: &mut Client,
        c: &MoneyMovementCommand,
    ) -> Result<Option<Existing>, MoneyMovementError> {
        let trx = Self::begin(client, c).await?;
        let row = trx
            .query_opt(
                "SELECT execution_id, instruction_fingerprint, request_sha256 FROM payment_ops.native_execution
                  WHERE tenant_id=$1 AND responsible_legal_entity_id=$2
                    AND operation_kind=$3 AND idempotency_key=$4",
                &[
                    &c.instruction.tenant_id,
                    &c.responsible_legal_entity_id,
                    &c.class.journal_kind(),
                    &c.idempotency_key,
                ],
            )
            .await
            .map_err(|_| MoneyMovementError::JournalUnavailable)?;
        trx.commit()
            .await
            .map_err(|_| MoneyMovementError::JournalUnavailable)?;
        Ok(row.map(|r| Existing {
            execution_id: r.get(0),
            fingerprint: r.get(1),
            request_hash: r.get(2),
        }))
    }

    async fn decide<A: MoneyMovementAuthority>(
        &self,
        authority: &A,
        context_id: Uuid,
        c: &MoneyMovementCommand,
        attempt: u16,
    ) -> Result<AuthorizedDecision, MoneyMovementError> {
        let request = authority_request(
            context_id,
            &self.engine_instance_id,
            c.class,
            &c.operation_reference,
            attempt,
            &c.instruction,
        )
        .ok_or(MoneyMovementError::InvalidInstruction)?;
        let response = authority.resolve(&request).await.map_err(|e| match e {
            CpAuthorityError::NotAuthorised => {
                MoneyMovementError::AuthorityDenied("CP_REFUSED".into())
            }
            _ => MoneyMovementError::AuthorityUnavailable,
        })?;
        require_authorized(
            &request,
            &response,
            &c.responsible_legal_entity_id,
            OffsetDateTime::now_utc(),
        )
        .map_err(|refusal| match refusal {
            DecisionRefusal::Denied(outcome) => MoneyMovementError::AuthorityDenied(outcome),
            DecisionRefusal::Invalid => MoneyMovementError::AuthorityUnavailable,
        })
    }

    async fn record_decision(
        trx: &Transaction<'_>,
        c: &MoneyMovementCommand,
        d: &AuthorizedDecision,
    ) -> Result<(), MoneyMovementError> {
        trx.execute(
            "INSERT INTO payment_ops.authority_decision (decision_id, tenant_id, responsible_legal_entity_id,
                 operation_class, operation_reference, attempt, instruction_fingerprint, mandate_id,
                 evaluated_at, expires_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9::text::timestamptz,$10::text::timestamptz)",
            &[
                &d.decision_id,
                &c.instruction.tenant_id,
                &c.responsible_legal_entity_id,
                &c.class.as_str(),
                &c.operation_reference,
                &i32::from(d.attempt),
                &d.instruction_fingerprint,
                &d.mandate_id,
                &d.evaluated_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
                &d.expires_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
            ],
        )
        .await
        .map_err(|e| journal_error(&e))?;
        Ok(())
    }

    /// Transaction 1: decision + reservation + outbox. Returns None when a
    /// concurrent worker already reserved this key.
    async fn reserve(
        &self,
        client: &mut Client,
        c: &MoneyMovementCommand,
        fp: &str,
        d: &AuthorizedDecision,
    ) -> Result<Option<Uuid>, MoneyMovementError> {
        let trx = Self::begin(client, c).await?;
        trx.execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!(
                "{}|{}|{}|{}",
                c.instruction.tenant_id,
                c.responsible_legal_entity_id,
                c.class.journal_kind(),
                c.idempotency_key
            )],
        )
        .await
        .map_err(|_| MoneyMovementError::JournalUnavailable)?;
        let taken = trx
            .query_opt(
                "SELECT 1 FROM payment_ops.native_execution WHERE tenant_id=$1
                   AND responsible_legal_entity_id=$2 AND operation_kind=$3 AND idempotency_key=$4",
                &[
                    &c.instruction.tenant_id,
                    &c.responsible_legal_entity_id,
                    &c.class.journal_kind(),
                    &c.idempotency_key,
                ],
            )
            .await
            .map_err(|_| MoneyMovementError::JournalUnavailable)?;
        if taken.is_some() {
            return Ok(None);
        }
        Self::record_decision(&trx, c, d).await?;
        let i = &c.instruction;
        let organisation = c.operating_organisation_id;
        let row = trx
            .query_one(
                "SELECT payment_ops.reserve_money_movement($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,
                     $15,$16,$17,$18,$19,$20,$21)",
                &[
                    &i.tenant_id,
                    &c.class.journal_kind(),
                    &c.idempotency_key,
                    &request_hash(fp, c.payment_token.as_deref()),
                    &i.payment_reference,
                    &organisation,
                    &c.responsible_legal_entity_id,
                    &d.mandate_id,
                    &format!("cp/money-movement-decision/{}", d.decision_id),
                    // Readiness references are recorded from the activation
                    // the journal itself verified, at dispatch.
                    &"journal/provider-activation",
                    &"journal/provider-activation",
                    &i.market,
                    &i.currency,
                    &i.amount_minor,
                    &c.class.as_str(),
                    &c.operation_reference,
                    &fp,
                    &i.parent_reference,
                    &i.merchant_profile_reference,
                    &i.destination_reference,
                    &self.provider_environment,
                ],
            )
            .await
            .map_err(|e| journal_error(&e))?;
        let execution_id: Uuid = row.get(0);
        trx.execute(
            "INSERT INTO payment_ops.transaction_outbox
               (tenant_id, responsible_legal_entity_id, execution_id, event_type, event_payload, correlation_id)
             VALUES ($1,$2,$3,'payments.native-operation.reserved.v1',
                     jsonb_build_object('operation_class',$4::text,'operation_reference',$5::text,
                                        'decision_id',$6::text,'instruction_fingerprint',$7::text),$8)",
            &[
                &i.tenant_id,
                &c.responsible_legal_entity_id,
                &execution_id,
                &c.class.as_str(),
                &c.operation_reference,
                &d.decision_id.to_string(),
                &fp,
                &c.correlation_id,
            ],
        )
        .await
        .map_err(|e| journal_error(&e))?;
        trx.commit().await.map_err(|e| journal_error(&e))?;
        Ok(Some(execution_id))
    }

    fn provider_call(
        c: &MoneyMovementCommand,
    ) -> Result<(NativeOperation<'_>, Value), MoneyMovementError> {
        let i = &c.instruction;
        let parent = i.parent_reference.as_deref();
        Ok(match c.class {
            OperationClass::Authorize => {
                let mut body = json!({});
                if let Some(token) = &c.payment_token {
                    body["payment_token"] = json!(token);
                }
                (
                    NativeOperation::Confirm {
                        payment_id: &i.payment_reference,
                    },
                    body,
                )
            }
            OperationClass::Capture => (
                NativeOperation::Capture {
                    payment_id: parent.ok_or(MoneyMovementError::InvalidInstruction)?,
                },
                json!({"amount_to_capture": i.amount_minor}),
            ),
            OperationClass::Void => (
                NativeOperation::Cancel {
                    payment_id: parent.ok_or(MoneyMovementError::InvalidInstruction)?,
                },
                json!({}),
            ),
            OperationClass::Refund => (
                NativeOperation::Refund,
                // The refund id is ours, so a lost response can be read back.
                json!({"payment_id": parent, "refund_id": i.payment_reference, "amount": i.amount_minor}),
            ),
            OperationClass::Payout => return Err(MoneyMovementError::NotActivated),
        })
    }

    /// The provider record that proves this movement's own effect.
    fn provider_reference(c: &MoneyMovementCommand) -> (RecordKind, &str) {
        match c.class {
            OperationClass::Refund => (RecordKind::Refund, &c.instruction.payment_reference),
            OperationClass::Authorize => (RecordKind::Payment, &c.instruction.payment_reference),
            _ => (
                RecordKind::Payment,
                c.instruction
                    .parent_reference
                    .as_deref()
                    .unwrap_or_default(),
            ),
        }
    }

    /// Transaction 2 (fresh decision + dispatch record), the single send, and
    /// transaction 3 (observation + outbox).
    #[allow(clippy::too_many_arguments)]
    async fn dispatch<A: MoneyMovementAuthority, D: DocumentaryVeto>(
        &self,
        authority: &A,
        documentary: &D,
        context_id: Uuid,
        c: &MoneyMovementCommand,
        execution_id: Uuid,
        attempt: u16,
        client: &mut Client,
    ) -> Result<MoneyMovementOutcome, MoneyMovementError> {
        let (operation, body) = Self::provider_call(c)?;
        HyperSwitchTransport::validate_operation(&operation, &body, &c.idempotency_key)
            .map_err(|_| MoneyMovementError::InvalidInstruction)?;
        // A reservation is not a send permission: the execution point needs
        // its own current decision, documentary clearance and readiness.
        let decision = self.decide(authority, context_id, c, attempt).await?;
        documentary
            .clear(c.class, &c.operation_reference)
            .await
            .map_err(|_| MoneyMovementError::DocumentaryHold)?;
        let trx = Self::begin(client, c).await?;
        Self::record_decision(&trx, c, &decision).await?;
        let i = &c.instruction;
        let activation: Option<Uuid> = trx
            .query_one(
                "SELECT payment_ops.current_activation($1,$2,$3,$4,$5,$6,$7)",
                &[
                    &i.tenant_id,
                    &c.responsible_legal_entity_id,
                    &i.merchant_profile_reference,
                    &i.market,
                    &i.currency,
                    &c.class.journal_kind(),
                    &self.provider_environment,
                ],
            )
            .await
            .map_err(|e| journal_error(&e))?
            .get(0);
        let activation = activation.ok_or(MoneyMovementError::ProviderNotReady)?;
        let dispatch_id: Uuid = trx
            .query_one(
                "INSERT INTO payment_ops.dispatch_attempt
                   (tenant_id, responsible_legal_entity_id, execution_id, decision_id, activation_id)
                 VALUES ($1,$2,$3,$4,$5) RETURNING dispatch_id",
                &[
                    &i.tenant_id,
                    &c.responsible_legal_entity_id,
                    &execution_id,
                    &decision.decision_id,
                    &activation,
                ],
            )
            .await
            .map_err(|e| journal_error(&e))?
            .get(0);
        trx.execute(
            "INSERT INTO payment_ops.transaction_outbox
               (tenant_id, responsible_legal_entity_id, execution_id, event_type, event_payload, correlation_id)
             VALUES ($1,$2,$3,'payments.money-movement.dispatched.v1',
                     jsonb_build_object('decision_id',$4::text,'attempt',$5::int,'activation_id',$6::text),$7)",
            &[
                &i.tenant_id,
                &c.responsible_legal_entity_id,
                &execution_id,
                &decision.decision_id.to_string(),
                &i32::from(decision.attempt),
                &activation.to_string(),
                &c.correlation_id,
            ],
        )
        .await
        .map_err(|e| journal_error(&e))?;
        trx.commit().await.map_err(|e| journal_error(&e))?;

        // The irreversible step. A timeout can mean the money moved: it is
        // recorded as UNKNOWN and reconciled, never silently re-sent.
        let (_, expected) = Self::provider_reference(c);
        let observed = self
            .transport
            .execute(operation, &body, &c.idempotency_key)
            .await;
        let (outcome, reference, status) = match &observed {
            Ok(o) if o.native_reference == expected => (
                classify_response(c.class, &o.provider_status),
                Some(o.native_reference.clone()),
                Some(o.provider_status.clone()),
            ),
            Ok(_) | Err(HyperSwitchError::InvalidResponse) | Err(HyperSwitchError::Unavailable) => {
                (Outcome::Unknown, None, None)
            }
            Err(HyperSwitchError::Rejected) => (Outcome::Rejected, None, None),
            Err(_) => (Outcome::Unknown, None, None),
        };
        // A confirmation always names the provider record.
        let (outcome, reference, status) = match (outcome, reference, status) {
            (Outcome::Confirmed, Some(r), Some(s)) => (Outcome::Confirmed, Some(r), Some(s)),
            (Outcome::Confirmed, _, _) => (Outcome::Unknown, None, None),
            other => other,
        };
        // A capture status alone does not say how much moved. Refunds are
        // approved against confirmed captures, so a capture is confirmed only
        // when the provider's own captured total covers it; otherwise it stays
        // UNKNOWN and is settled by reconciliation.
        let (outcome, reference, status) =
            if c.class == OperationClass::Capture && outcome == Outcome::Confirmed {
                let prior = self
                    .prior_confirmed_captures(client, c, execution_id)
                    .await?;
                match self.reconcile(c, prior).await {
                    (Outcome::Confirmed, Some(record)) => (
                        Outcome::Confirmed,
                        Some(record.reference),
                        Some(record.status),
                    ),
                    _ => (Outcome::Unknown, None, None),
                }
            } else {
                (outcome, reference, status)
            };
        self.observe(
            client,
            c,
            execution_id,
            dispatch_id,
            outcome,
            "DISPATCH_RESPONSE",
            reference.as_deref(),
            status.as_deref(),
        )
        .await?;
        match outcome {
            Outcome::Confirmed => Ok(MoneyMovementOutcome::Executed {
                execution_id,
                provider_reference: reference.unwrap_or_default(),
                provider_status: status.unwrap_or_default(),
            }),
            Outcome::Rejected => Err(MoneyMovementError::ProviderRejected { execution_id }),
            Outcome::Unknown | Outcome::Absent => {
                Err(MoneyMovementError::OutcomeUnknown { execution_id })
            }
        }
    }

    /// The amount this payment's other confirmed captures already account for.
    async fn prior_confirmed_captures(
        &self,
        client: &mut Client,
        c: &MoneyMovementCommand,
        execution_id: Uuid,
    ) -> Result<i64, MoneyMovementError> {
        let trx = Self::begin(client, c).await?;
        let prior: i64 = trx
            .query_one(
                "SELECT coalesce(sum(e.amount_minor), 0)::bigint FROM payment_ops.native_execution e
                  WHERE e.parent_reference=$1 AND e.operation_kind='CAPTURE' AND e.execution_id<>$2
                    AND payment_ops.execution_outcome(e.execution_id)='CONFIRMED'",
                &[&c.instruction.parent_reference, &execution_id],
            )
            .await
            .map_err(|_| MoneyMovementError::JournalUnavailable)?
            .get(0);
        trx.commit()
            .await
            .map_err(|_| MoneyMovementError::JournalUnavailable)?;
        Ok(prior)
    }

    #[allow(clippy::too_many_arguments)]
    async fn observe(
        &self,
        client: &mut Client,
        c: &MoneyMovementCommand,
        execution_id: Uuid,
        dispatch_id: Uuid,
        outcome: Outcome,
        source: &str,
        reference: Option<&str>,
        status: Option<&str>,
    ) -> Result<(), MoneyMovementError> {
        let trx = Self::begin(client, c).await?;
        trx.execute(
            "INSERT INTO payment_ops.provider_observation (tenant_id, responsible_legal_entity_id, execution_id,
                 dispatch_id, outcome, source, provider_reference, provider_status, provider_environment)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            &[
                &c.instruction.tenant_id,
                &c.responsible_legal_entity_id,
                &execution_id,
                &dispatch_id,
                &outcome.as_str(),
                &source,
                &reference,
                &status,
                &self.provider_environment,
            ],
        )
        .await
        .map_err(|e| journal_error(&e))?;
        let event = if source == "RECONCILIATION" {
            "payments.money-movement.reconciled.v1"
        } else {
            "payments.provider-response.observed.v1"
        };
        trx.execute(
            "INSERT INTO payment_ops.transaction_outbox
               (tenant_id, responsible_legal_entity_id, execution_id, event_type, event_payload, correlation_id)
             VALUES ($1,$2,$3,$4,jsonb_build_object('outcome',$5::text,'provider_reference',$6::text,
                     'provider_status',$7::text,'provider_environment',$8::text),$9)",
            &[
                &c.instruction.tenant_id,
                &c.responsible_legal_entity_id,
                &execution_id,
                &event,
                &outcome.as_str(),
                &reference,
                &status,
                &self.provider_environment,
                &c.correlation_id,
            ],
        )
        .await
        .map_err(|e| journal_error(&e))?;
        trx.commit().await.map_err(|e| journal_error(&e))?;
        Ok(())
    }

    /// A key seen before: replay a confirmed movement, settle an unknown one
    /// by reading the provider back, and re-send only what the provider
    /// proved it never received, each time on a fresh CP decision.
    #[allow(clippy::too_many_arguments)]
    async fn resume<A: MoneyMovementAuthority, D: DocumentaryVeto>(
        &self,
        authority: &A,
        documentary: &D,
        context_id: Uuid,
        c: &MoneyMovementCommand,
        fp: &str,
        existing: Existing,
        client: &mut Client,
    ) -> Result<MoneyMovementOutcome, MoneyMovementError> {
        if existing.fingerprint.as_deref() != Some(fp)
            || existing.request_hash != request_hash(fp, c.payment_token.as_deref())
        {
            return Err(MoneyMovementError::IdempotencyConflict);
        }
        let execution_id = existing.execution_id;
        let trx = Self::begin(client, c).await?;
        let state = trx
            .query_one(
                "SELECT payment_ops.execution_outcome($1),
                        (SELECT dispatch_id FROM payment_ops.dispatch_attempt WHERE execution_id=$1
                          ORDER BY dispatched_at DESC LIMIT 1),
                        (SELECT provider_reference FROM payment_ops.provider_observation
                          WHERE execution_id=$1 AND outcome='CONFIRMED' ORDER BY observed_at DESC LIMIT 1),
                        (SELECT provider_status FROM payment_ops.provider_observation
                          WHERE execution_id=$1 AND outcome='CONFIRMED' ORDER BY observed_at DESC LIMIT 1),
                        (SELECT coalesce(max(attempt), 1) FROM payment_ops.authority_decision
                          WHERE operation_reference=$2),
                        (SELECT coalesce(sum(e.amount_minor), 0)::bigint FROM payment_ops.native_execution e
                          WHERE e.parent_reference=$3 AND e.operation_kind='CAPTURE' AND e.execution_id<>$1
                            AND payment_ops.execution_outcome(e.execution_id)='CONFIRMED')",
                &[&execution_id, &c.operation_reference, &c.instruction.parent_reference],
            )
            .await
            .map_err(|_| MoneyMovementError::JournalUnavailable)?;
        trx.commit()
            .await
            .map_err(|_| MoneyMovementError::JournalUnavailable)?;
        let decisive: Option<String> = state.get(0);
        let dispatch: Option<Uuid> = state.get(1);
        let last_attempt: i32 = state.get(4);
        let prior_captured: i64 = state.get(5);
        let next =
            u16::try_from(last_attempt + 1).map_err(|_| MoneyMovementError::StateConflict)?;
        match (decisive.as_deref(), dispatch) {
            (Some("CONFIRMED"), _) => Ok(MoneyMovementOutcome::Replayed {
                execution_id,
                provider_reference: state.get::<_, Option<String>>(2).unwrap_or_default(),
                provider_status: state.get::<_, Option<String>>(3).unwrap_or_default(),
            }),
            (Some("REJECTED"), _) => Err(MoneyMovementError::ProviderRejected { execution_id }),
            // Reserved but never dispatched (a crash between transactions),
            // or proven absent at the provider: dispatch on a fresh decision.
            (_, None) | (Some("ABSENT"), _) => {
                self.dispatch(
                    authority,
                    documentary,
                    context_id,
                    c,
                    execution_id,
                    next.max(2),
                    client,
                )
                .await
            }
            (_, Some(dispatch_id)) => {
                let outcome = self.reconcile(c, prior_captured).await;
                let (reference, status) = match &outcome {
                    (Outcome::Confirmed, Some(record)) => {
                        (Some(record.reference.clone()), Some(record.status.clone()))
                    }
                    _ => (None, None),
                };
                let decided = if reference.is_some() || outcome.0 != Outcome::Confirmed {
                    outcome.0
                } else {
                    Outcome::Unknown
                };
                if decided != Outcome::Unknown {
                    self.observe(
                        client,
                        c,
                        execution_id,
                        dispatch_id,
                        decided,
                        "RECONCILIATION",
                        reference.as_deref(),
                        status.as_deref(),
                    )
                    .await?;
                }
                match decided {
                    Outcome::Confirmed => Ok(MoneyMovementOutcome::Replayed {
                        execution_id,
                        provider_reference: reference.unwrap_or_default(),
                        provider_status: status.unwrap_or_default(),
                    }),
                    Outcome::Rejected => Err(MoneyMovementError::ProviderRejected { execution_id }),
                    Outcome::Absent => {
                        self.dispatch(
                            authority,
                            documentary,
                            context_id,
                            c,
                            execution_id,
                            next.max(2),
                            client,
                        )
                        .await
                    }
                    Outcome::Unknown => Err(MoneyMovementError::OutcomeUnknown { execution_id }),
                }
            }
        }
    }

    /// Read the provider's own record back and decide what it proves for this
    /// movement. Anything ambiguous stays UNKNOWN.
    async fn reconcile(
        &self,
        c: &MoneyMovementCommand,
        prior_captured: i64,
    ) -> (Outcome, Option<ProviderRecord>) {
        let (kind, reference) = Self::provider_reference(c);
        let record = match self.transport.retrieve(kind, reference).await {
            Ok(record) => record,
            Err(HyperSwitchError::NotFound) if c.class == OperationClass::Refund => {
                return (Outcome::Absent, None);
            }
            Err(_) => return (Outcome::Unknown, None),
        };
        let outcome = match c.class {
            OperationClass::Authorize => match record.status.as_str() {
                "requires_capture" | "succeeded" | "partially_captured" => Outcome::Confirmed,
                "failed" => Outcome::Rejected,
                "requires_confirmation" | "requires_payment_method" => Outcome::Absent,
                _ => Outcome::Unknown,
            },
            OperationClass::Capture => match record.amount_captured {
                Some(captured) if captured >= prior_captured + c.instruction.amount_minor => {
                    Outcome::Confirmed
                }
                Some(captured)
                    if captured == prior_captured && record.status == "requires_capture" =>
                {
                    Outcome::Absent
                }
                _ => Outcome::Unknown,
            },
            OperationClass::Void => match record.status.as_str() {
                "cancelled" => Outcome::Confirmed,
                "requires_capture" => Outcome::Absent,
                _ => Outcome::Unknown,
            },
            OperationClass::Refund => match record.status.as_str() {
                "succeeded" | "pending" | "review" => Outcome::Confirmed,
                "failure" | "failed" => Outcome::Rejected,
                _ => Outcome::Unknown,
            },
            OperationClass::Payout => Outcome::Unknown,
        };
        (outcome, Some(record))
    }
}

#[cfg(test)]
mod tests;
