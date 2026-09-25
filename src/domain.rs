//! The Baobab Payment API resources (Shared `payments/v1`). HyperSwitch
//! objects are never part of this model.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Money {
    pub currency: String,
    pub amount_minor: i64,
}

/// Where and for whom money moves, asserted by the calling engine from
/// Control Plane-resolved context. This engine never decides it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentContext {
    pub tenant_id: String,
    pub legal_entity_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_account_id: Option<String>,
    pub currency: String,
    pub source_engine: String,
    pub source_reference: String,
    pub correlation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderResult {
    pub kind: String,
    pub simulated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_reference: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CaptureMethod {
    Automatic,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntentStatus {
    RequiresPaymentMethod,
    RequiresCapture,
    Succeeded,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Authorized,
    Captured,
    Failed,
    Cancelled,
    PartiallyRefunded,
    Refunded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntent {
    pub payment_intent_id: String,
    pub context: PaymentContext,
    pub amount: Money,
    pub capture_method: CaptureMethod,
    pub status: IntentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<String>,
    pub provider: ProviderResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub version: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payment {
    pub payment_id: String,
    pub payment_intent_id: String,
    pub tenant_id: String,
    pub legal_entity_id: String,
    pub amount: Money,
    pub captured_amount_minor: i64,
    pub refunded_amount_minor: i64,
    pub status: PaymentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
    pub provider: ProviderResult,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refund {
    pub refund_id: String,
    pub payment_id: String,
    pub tenant_id: String,
    pub amount: Money,
    pub status: RefundStatus,
    pub reason: String,
    pub provider: ProviderResult,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

// --- Requests (validated against the contract before they are decoded) -----

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePaymentIntentRequest {
    pub context: PaymentContext,
    pub amount: Money,
    pub capture_method: CaptureMethod,
    #[serde(default)]
    pub description: Option<String>,
}

/// The payment method reference is opaque and already tokenised. It is used
/// for the provider call only: never stored, logged or returned.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmPaymentIntentRequest {
    pub tenant_id: String,
    pub payment_method_reference: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturePaymentRequest {
    pub tenant_id: String,
    #[serde(default)]
    pub amount_minor: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelPaymentRequest {
    pub tenant_id: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRefundRequest {
    pub tenant_id: String,
    pub amount_minor: i64,
    pub reason: String,
}

/// Engine-minted opaque identifiers: a prefix and a UUIDv7 body.
pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::now_v7().simple())
}
