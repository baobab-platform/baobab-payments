//! The Baobab Payment API (ADR-PAY-0001, Shared `payments/v1`).
//!
//! Every money-moving request is idempotent on (tenant, operation,
//! Idempotency-Key). Every read and write is scoped by tenant, so another
//! tenant's identifiers are not found. The payment context is the caller's
//! (Control Plane-resolved); the engine only checks it is coherent: the
//! amount's currency is the context currency, and a workload can only move
//! money for obligations of its own engine. Events are recorded only for
//! state that occurred, and every one carries `simulated`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::contracts;
use crate::domain::*;
use crate::provider::{Authorization, PaymentProvider};

pub const EVENT_SOURCE: &str = "urn:baobab-platform:service:baobab-payments";

/// A registered payment event: its type and its payload definition.
#[derive(Clone, Copy)]
struct EventKind(&'static str, &'static str);

const CREATED: EventKind = EventKind(
    "com.baobab-platform.payments.payment.created.v1",
    "PaymentCreated",
);
const AUTHORIZED: EventKind = EventKind(
    "com.baobab-platform.payments.payment.authorized.v1",
    "PaymentAuthorized",
);
const CAPTURED: EventKind = EventKind(
    "com.baobab-platform.payments.payment.captured.v1",
    "PaymentCaptured",
);
const FAILED: EventKind = EventKind(
    "com.baobab-platform.payments.payment.failed.v1",
    "PaymentFailed",
);
const CANCELLED: EventKind = EventKind(
    "com.baobab-platform.payments.payment.cancelled.v1",
    "PaymentCancelled",
);
const REFUNDED: EventKind = EventKind(
    "com.baobab-platform.payments.payment.refunded.v1",
    "PaymentRefunded",
);

/// A refused request, in the shape of an RFC 9457 problem.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub status: u16,
    pub code: &'static str,
    pub detail: String,
    pub retryable: bool,
    pub problems: Vec<String>,
}

impl Refusal {
    pub fn new(status: u16, code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            status,
            code,
            detail: detail.into(),
            retryable: false,
            problems: Vec::new(),
        }
    }

    fn invalid(problems: Vec<String>) -> Self {
        Self {
            problems,
            ..Self::new(
                400,
                "VALIDATION_FAILED",
                "the request does not satisfy the contract",
            )
        }
    }

    fn not_found(what: &str) -> Self {
        Self::new(
            404,
            "PAYMENT_RESOURCE_NOT_FOUND",
            format!("no such {what} for this tenant"),
        )
    }

    fn state(detail: impl Into<String>) -> Self {
        Self::new(409, "INVALID_PAYMENT_STATE", detail)
    }
}

/// An outcome: the HTTP status, the contract body, and whether it replays an earlier request.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub status: u16,
    pub body: Value,
    pub replayed: bool,
}

/// A canonical event envelope recorded with the change it describes.
#[derive(Debug, Clone)]
pub struct RecordedEvent {
    pub event_type: String,
    pub tenant_id: String,
    pub envelope: Value,
}

struct Stored {
    request_hash: String,
    status: u16,
    body: Value,
}

type Key = (String, String);

#[derive(Default)]
struct State {
    intents: HashMap<Key, PaymentIntent>,
    payments: HashMap<Key, Payment>,
    refunds: HashMap<Key, Refund>,
    idempotency: HashMap<(String, String, String), Stored>,
    events: Vec<RecordedEvent>,
}

pub type Clock = Arc<dyn Fn() -> OffsetDateTime + Send + Sync>;

/// The payment service over an in-memory store. Durable persistence is not
/// implemented yet, so configuration refuses staging and production.
pub struct PaymentService {
    state: Mutex<State>,
    provider: Arc<dyn PaymentProvider>,
    clock: Clock,
}

/// Who is asking: the authenticated workload and the request's correlation.
pub struct Call<'a> {
    pub client_id: &'a str,
    /// The engine the authenticated client acts for.
    pub engine: &'a str,
    pub idempotency_key: &'a str,
    pub correlation_id: &'a str,
}

fn parse_body(body: &[u8]) -> Result<Value, Refusal> {
    match serde_json::from_slice::<Value>(body) {
        Ok(v) if v.is_object() => Ok(v),
        _ => Err(Refusal::invalid(vec![
            "$: a JSON object is required".into(),
        ])),
    }
}

fn decode<T: DeserializeOwned>(value: &Value, definition: &str) -> Result<T, Refusal> {
    let problems = contracts::problems(&contracts::def(contracts::PAYMENT, definition), value);
    if !problems.is_empty() {
        return Err(Refusal::invalid(problems));
    }
    serde_json::from_value(value.clone()).map_err(|e| Refusal::invalid(vec![format!("$: {e}")]))
}

fn hash(value: &Value) -> String {
    // serde_json orders object keys, so equal requests hash equally.
    hex::encode(Sha256::digest(value.to_string().as_bytes()))
}

fn timestamp(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).expect("a UTC time formats as RFC 3339")
}

fn body<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("resources serialise")
}

impl PaymentService {
    pub fn new(provider: Arc<dyn PaymentProvider>, clock: Clock) -> Self {
        Self {
            state: Mutex::new(State::default()),
            provider,
            clock,
        }
    }

    pub fn provider(&self) -> &dyn PaymentProvider {
        self.provider.as_ref()
    }

    fn now(&self) -> OffsetDateTime {
        (self.clock)()
    }

    fn provider_result(&self, reference: Option<String>) -> ProviderResult {
        ProviderResult {
            kind: self.provider.kind().into(),
            simulated: self.provider.simulated(),
            provider_reference: reference,
        }
    }

    /// Runs `work` atomically, replaying a stored response for a repeated
    /// Idempotency-Key and refusing the key's reuse for a different request.
    fn idempotent(
        &self,
        tenant_id: &str,
        operation: String,
        call: &Call<'_>,
        request: &Value,
        work: impl FnOnce(&mut State) -> Result<(u16, Value), Refusal>,
    ) -> Result<Outcome, Refusal> {
        let request_hash = hash(request);
        let key = (
            tenant_id.to_string(),
            operation,
            call.idempotency_key.to_string(),
        );
        let mut state = self
            .state
            .lock()
            .map_err(|_| Refusal::new(503, "PAYMENTS_UNAVAILABLE", "state is unavailable"))?;
        if let Some(prior) = state.idempotency.get(&key) {
            if prior.request_hash != request_hash {
                return Err(Refusal::new(
                    409,
                    "IDEMPOTENCY_KEY_REUSED",
                    "the Idempotency-Key was used for a different request",
                ));
            }
            return Ok(Outcome {
                status: prior.status,
                body: prior.body.clone(),
                replayed: true,
            });
        }
        // A refusal leaves no trace: work checks before it writes, and any
        // event it recorded before refusing is dropped.
        let events_before = state.events.len();
        let (status, body) = match work(&mut state) {
            Ok(result) => result,
            Err(refusal) => {
                state.events.truncate(events_before);
                return Err(refusal);
            }
        };
        state.idempotency.insert(
            key,
            Stored {
                request_hash,
                status,
                body: body.clone(),
            },
        );
        Ok(Outcome {
            status,
            body,
            replayed: false,
        })
    }

    fn event(
        &self,
        state: &mut State,
        kind: EventKind,
        intent: &PaymentIntent,
        call: &Call<'_>,
        data: Value,
        at: OffsetDateTime,
    ) {
        let mut data = data;
        let fields = data.as_object_mut().expect("event data is an object");
        fields.insert("payment_intent_id".into(), json!(intent.payment_intent_id));
        fields.insert("tenant_id".into(), json!(intent.context.tenant_id));
        fields.insert(
            "legal_entity_id".into(),
            json!(intent.context.legal_entity_id),
        );
        fields.insert("source_engine".into(), json!(intent.context.source_engine));
        fields.insert(
            "source_reference".into(),
            json!(intent.context.source_reference),
        );
        fields.insert("amount".into(), body(&intent.amount));
        fields.insert("simulated".into(), json!(self.provider.simulated()));
        fields.insert("occurred_at".into(), json!(timestamp(at)));
        let envelope = json!({
            "specversion": "1.0",
            "id": uuid::Uuid::now_v7().to_string(),
            "type": kind.0,
            "source": EVENT_SOURCE,
            "subject": format!("payment-intent:{}", intent.payment_intent_id),
            "time": timestamp(at),
            "datacontenttype": "application/json",
            "dataschema": format!("{}{}", contracts::BASE, contracts::def(contracts::EVENTS, kind.1)),
            "baobabscope": "tenant",
            "correlationid": call.correlation_id,
            "tenantid": intent.context.tenant_id,
            "idempotencykey": call.idempotency_key,
            "data": data,
        });
        state.events.push(RecordedEvent {
            event_type: kind.0.into(),
            tenant_id: intent.context.tenant_id.clone(),
            envelope,
        });
    }

    /// Creates a payment intent for an obligation of the calling engine.
    pub fn create_intent(&self, call: &Call<'_>, raw: &[u8]) -> Result<Outcome, Refusal> {
        let value = parse_body(raw)?;
        let req: CreatePaymentIntentRequest = decode(&value, "CreatePaymentIntentRequest")?;
        if req.amount.currency != req.context.currency {
            return Err(Refusal::new(
                422,
                "CURRENCY_MISMATCH",
                "the amount's currency must be the context currency",
            ));
        }
        if req.context.source_engine != call.engine {
            return Err(Refusal::new(
                403,
                "SOURCE_ENGINE_MISMATCH",
                "a workload moves money only for its own engine's obligations",
            ));
        }
        let tenant = req.context.tenant_id.clone();
        self.idempotent(&tenant, "create".into(), call, &value, |state| {
            let at = self.now();
            let intent = PaymentIntent {
                payment_intent_id: new_id("payint"),
                context: req.context,
                amount: req.amount,
                capture_method: req.capture_method,
                status: IntentStatus::RequiresPaymentMethod,
                payment_id: None,
                provider: self.provider_result(None),
                description: req.description,
                version: 1,
                created_at: at,
                updated_at: at,
            };
            self.event(state, CREATED, &intent, call, json!({}), at);
            let out = body(&intent);
            state
                .intents
                .insert((tenant.clone(), intent.payment_intent_id.clone()), intent);
            Ok((201, out))
        })
    }

    /// Authorises the intent with an opaque payment method reference, and
    /// captures at once when the capture method is AUTOMATIC.
    pub fn confirm_intent(
        &self,
        call: &Call<'_>,
        intent_id: &str,
        raw: &[u8],
    ) -> Result<Outcome, Refusal> {
        let value = parse_body(raw)?;
        let req: ConfirmPaymentIntentRequest = decode(&value, "ConfirmPaymentIntentRequest")?;
        let tenant = req.tenant_id.clone();
        self.idempotent(&tenant, format!("confirm:{intent_id}"), call, &value, |state| {
            let key = (tenant.clone(), intent_id.to_string());
            let mut intent = state.intents.get(&key).cloned().ok_or_else(|| Refusal::not_found("payment intent"))?;
            if intent.status != IntentStatus::RequiresPaymentMethod {
                return Err(Refusal::state("the payment intent is not awaiting a payment method"));
            }
            let at = self.now();
            intent.version += 1;
            intent.updated_at = at;
            match self.provider.authorize(&intent, &req.payment_method_reference) {
                Authorization::Declined { failure_code } => {
                    intent.status = IntentStatus::Failed;
                    self.event(state, FAILED, &intent, call,
                        json!({ "failure_code": failure_code }), at);
                }
                Authorization::Authorized { provider_reference } => {
                    let mut payment = Payment {
                        payment_id: new_id("pay"),
                        payment_intent_id: intent.payment_intent_id.clone(),
                        tenant_id: tenant.clone(),
                        legal_entity_id: intent.context.legal_entity_id.clone(),
                        amount: intent.amount.clone(),
                        captured_amount_minor: 0,
                        refunded_amount_minor: 0,
                        status: PaymentStatus::Authorized,
                        failure_code: None,
                        provider: self.provider_result(Some(provider_reference)),
                        created_at: at,
                        updated_at: at,
                    };
                    intent.payment_id = Some(payment.payment_id.clone());
                    self.event(state, AUTHORIZED, &intent, call,
                        json!({ "payment_id": payment.payment_id }), at);
                    if intent.capture_method == CaptureMethod::Automatic {
                        self.provider.capture(&payment, payment.amount.amount_minor);
                        payment.status = PaymentStatus::Captured;
                        payment.captured_amount_minor = payment.amount.amount_minor;
                        intent.status = IntentStatus::Succeeded;
                        self.event(state, CAPTURED, &intent, call,
                            json!({ "payment_id": payment.payment_id, "captured_amount_minor": payment.captured_amount_minor }), at);
                    } else {
                        intent.status = IntentStatus::RequiresCapture;
                    }
                    state.payments.insert((tenant.clone(), payment.payment_id.clone()), payment);
                }
            }
            let out = body(&intent);
            state.intents.insert(key, intent);
            Ok((200, out))
        })
    }

    /// Captures an authorised payment, in full or in part.
    pub fn capture(
        &self,
        call: &Call<'_>,
        payment_id: &str,
        raw: &[u8],
    ) -> Result<Outcome, Refusal> {
        let value = parse_body(raw)?;
        let req: CapturePaymentRequest = decode(&value, "CapturePaymentRequest")?;
        let tenant = req.tenant_id.clone();
        self.idempotent(
            &tenant,
            format!("capture:{payment_id}"),
            call,
            &value,
            |state| {
                let key = (tenant.clone(), payment_id.to_string());
                let mut payment = state
                    .payments
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| Refusal::not_found("payment"))?;
                if payment.status != PaymentStatus::Authorized {
                    return Err(Refusal::state("only an authorised payment can be captured"));
                }
                let amount = req.amount_minor.unwrap_or(payment.amount.amount_minor);
                if amount > payment.amount.amount_minor {
                    return Err(Refusal::new(
                        422,
                        "CAPTURE_EXCEEDS_AUTHORIZED",
                        "the capture exceeds the authorised amount",
                    ));
                }
                let intent_key = (tenant.clone(), payment.payment_intent_id.clone());
                let mut intent = state
                    .intents
                    .get(&intent_key)
                    .cloned()
                    .ok_or_else(|| Refusal::not_found("payment intent"))?;
                let at = self.now();
                self.provider.capture(&payment, amount);
                payment.status = PaymentStatus::Captured;
                payment.captured_amount_minor = amount;
                payment.updated_at = at;
                intent.status = IntentStatus::Succeeded;
                intent.version += 1;
                intent.updated_at = at;
                self.event(
                    state,
                    CAPTURED,
                    &intent,
                    call,
                    json!({ "payment_id": payment.payment_id, "captured_amount_minor": amount }),
                    at,
                );
                let out = body(&payment);
                state.payments.insert(key, payment);
                state.intents.insert(intent_key, intent);
                Ok((200, out))
            },
        )
    }

    /// Cancels an intent that has not been captured, voiding any authorisation.
    pub fn cancel_intent(
        &self,
        call: &Call<'_>,
        intent_id: &str,
        raw: &[u8],
    ) -> Result<Outcome, Refusal> {
        let value = parse_body(raw)?;
        let req: CancelPaymentRequest = decode(&value, "CancelPaymentRequest")?;
        let tenant = req.tenant_id.clone();
        self.idempotent(
            &tenant,
            format!("cancel:{intent_id}"),
            call,
            &value,
            |state| {
                let key = (tenant.clone(), intent_id.to_string());
                let mut intent = state
                    .intents
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| Refusal::not_found("payment intent"))?;
                if !matches!(
                    intent.status,
                    IntentStatus::RequiresPaymentMethod | IntentStatus::RequiresCapture
                ) {
                    return Err(Refusal::state(
                        "only an uncaptured payment intent can be cancelled",
                    ));
                }
                let at = self.now();
                if let Some(payment_id) = intent.payment_id.clone() {
                    let payment_key = (tenant.clone(), payment_id);
                    if let Some(mut payment) = state.payments.get(&payment_key).cloned() {
                        self.provider.void(&payment);
                        payment.status = PaymentStatus::Cancelled;
                        payment.updated_at = at;
                        state.payments.insert(payment_key, payment);
                    }
                }
                intent.status = IntentStatus::Cancelled;
                intent.version += 1;
                intent.updated_at = at;
                self.event(state, CANCELLED, &intent, call, json!({}), at);
                let out = body(&intent);
                state.intents.insert(key, intent);
                Ok((200, out))
            },
        )
    }

    /// Refunds part or all of what was captured.
    pub fn refund(
        &self,
        call: &Call<'_>,
        payment_id: &str,
        raw: &[u8],
    ) -> Result<Outcome, Refusal> {
        let value = parse_body(raw)?;
        let req: CreateRefundRequest = decode(&value, "CreateRefundRequest")?;
        let tenant = req.tenant_id.clone();
        self.idempotent(&tenant, format!("refund:{payment_id}"), call, &value, |state| {
            let key = (tenant.clone(), payment_id.to_string());
            let mut payment = state.payments.get(&key).cloned().ok_or_else(|| Refusal::not_found("payment"))?;
            if !matches!(payment.status, PaymentStatus::Captured | PaymentStatus::PartiallyRefunded) {
                return Err(Refusal::state("only a captured payment can be refunded"));
            }
            let refundable = payment.captured_amount_minor - payment.refunded_amount_minor;
            if req.amount_minor > refundable {
                return Err(Refusal::new(422, "REFUND_EXCEEDS_CAPTURED", "the refund exceeds what remains captured"));
            }
            let intent = state
                .intents
                .get(&(tenant.clone(), payment.payment_intent_id.clone()))
                .cloned()
                .ok_or_else(|| Refusal::not_found("payment intent"))?;
            let at = self.now();
            let reference = self.provider.refund(&payment, req.amount_minor);
            payment.refunded_amount_minor += req.amount_minor;
            payment.status = if payment.refunded_amount_minor == payment.captured_amount_minor {
                PaymentStatus::Refunded
            } else {
                PaymentStatus::PartiallyRefunded
            };
            payment.updated_at = at;
            let refund = Refund {
                refund_id: new_id("refund"),
                payment_id: payment.payment_id.clone(),
                tenant_id: tenant.clone(),
                amount: Money { currency: payment.amount.currency.clone(), amount_minor: req.amount_minor },
                status: RefundStatus::Succeeded,
                reason: req.reason,
                provider: self.provider_result(Some(reference)),
                created_at: at,
            };
            self.event(state, REFUNDED, &intent, call,
                json!({ "payment_id": payment.payment_id, "refund_id": refund.refund_id, "refunded_amount_minor": req.amount_minor }), at);
            let out = body(&refund);
            state.refunds.insert((tenant.clone(), refund.refund_id.clone()), refund);
            state.payments.insert(key, payment);
            Ok((201, out))
        })
    }

    pub fn intent(&self, tenant_id: &str, intent_id: &str) -> Result<Value, Refusal> {
        let state = self
            .state
            .lock()
            .map_err(|_| Refusal::new(503, "PAYMENTS_UNAVAILABLE", "state is unavailable"))?;
        state
            .intents
            .get(&(tenant_id.to_string(), intent_id.to_string()))
            .map(body)
            .ok_or_else(|| Refusal::not_found("payment intent"))
    }

    pub fn payment(&self, tenant_id: &str, payment_id: &str) -> Result<Value, Refusal> {
        let state = self
            .state
            .lock()
            .map_err(|_| Refusal::new(503, "PAYMENTS_UNAVAILABLE", "state is unavailable"))?;
        state
            .payments
            .get(&(tenant_id.to_string(), payment_id.to_string()))
            .map(body)
            .ok_or_else(|| Refusal::not_found("payment"))
    }

    /// Events recorded for a tenant, oldest first. No relay exists yet.
    pub fn events(&self, tenant_id: &str) -> Vec<RecordedEvent> {
        self.state
            .lock()
            .map(|s| {
                s.events
                    .iter()
                    .filter(|e| e.tenant_id == tenant_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}
