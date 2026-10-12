//! LA-05 PAY-HT01 — HTTP transport for a separately governed HyperSwitch instance.
//!
//! This is REAL outbound HTTP code, not a simulated PaymentProvider. It is NOT
//! installed in PaymentService: in-memory state, uncertified merchants, absent
//! legal-actor evidence and incomplete settlement reconciliation still forbid
//! protected money movement. Admission is owned by the future durable
//! financial transaction boundary, never by a client-provided body.
//!
//! HyperSwitch public API v1: /payments, /payments/{id}/confirm,
//! /payments/{id}/capture and /refunds. We deliberately expose only
//! transport and provider-observation semantics, not Baobab payment success.

use std::fmt;
use std::time::Duration;

use reqwest::{Client, Method, Url};
use serde_json::Value;

const MAX_PROVIDER_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperSwitchError {
    Configuration,
    InvalidOperation,
    Unavailable,
    Rejected,
    InvalidResponse,
}

#[derive(Clone)]
pub struct HyperSwitchTransport {
    client: Client,
    origin: Url,
    api_key: String,
}

impl fmt::Debug for HyperSwitchTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HyperSwitchTransport")
            .field("origin", &self.origin.as_str())
            .field("api_key", &"<redacted>")
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOperation<'a> {
    Create,
    Confirm { payment_id: &'a str },
    Capture { payment_id: &'a str },
    Cancel { payment_id: &'a str },
    Refund,
}

impl NativeOperation<'_> {
    fn path(&self) -> Result<String, HyperSwitchError> {
        match self {
            Self::Create => Ok("payments".into()),
            Self::Confirm { payment_id } => Ok(format!("payments/{}/confirm", record_id(payment_id)?)),
            Self::Capture { payment_id } => Ok(format!("payments/{}/capture", record_id(payment_id)?)),
            Self::Cancel { payment_id } => Ok(format!("payments/{}/cancel", record_id(payment_id)?)),
            Self::Refund => Ok("refunds".into()),
        }
    }
}

/// A provider response is *observed*, not a financially settled Baobab fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderObservation {
    pub native_reference: String,
    pub provider_status: String,
}

fn record_id(id: &str) -> Result<&str, HyperSwitchError> {
    if (3..=120).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        Ok(id)
    } else {
        Err(HyperSwitchError::InvalidOperation)
    }
}

fn idempotency_key(value: &str) -> Result<&str, HyperSwitchError> {
    if (16..=128).contains(&value.len())
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    {
        Ok(value)
    } else {
        Err(HyperSwitchError::InvalidOperation)
    }
}

/// Reject un-tokenised card/bank details even if a caller accidentally places
/// them in a provider request. Consumers pass only PSP-issued method tokens.
fn reject_sensitive_data(value: &Value) -> Result<(), HyperSwitchError> {
    match value {
        Value::Object(fields) => {
            for (key, nested) in fields {
                let normalised = key.to_ascii_lowercase();
                if matches!(
                    normalised.as_str(),
                    "card_number" | "card_cvc" | "cvc" | "cvv" | "pan"
                        | "account_number" | "bank_account_number" | "card_exp_month" | "card_exp_year"
                ) {
                    return Err(HyperSwitchError::InvalidOperation);
                }
                reject_sensitive_data(nested)?;
            }
        }
        Value::Array(items) => {
            for nested in items {
                reject_sensitive_data(nested)?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl HyperSwitchTransport {
    /// Loopback HTTP exists strictly to test the actual HTTP wire protocol.
    /// HTTPS is mandatory for any configured non-loopback HyperSwitch host.
    pub fn new(origin: &str, api_key: String, test_loopback_http: bool) -> Result<Self, HyperSwitchError> {
        let url = Url::parse(origin).map_err(|_| HyperSwitchError::Configuration)?;
        let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !((url.scheme() == "https") || (test_loopback_http && loopback && url.scheme() == "http"))
            || url.username() != ""
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || api_key.trim().is_empty()
            || api_key.len() > 4096
            || api_key.bytes().any(|b| b.is_ascii_control())
        {
            return Err(HyperSwitchError::Configuration);
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| HyperSwitchError::Configuration)?;
        Ok(Self { client, origin: url, api_key })
    }

    /// A caller MUST already hold CP-issued, operation-scoped legal actor,
    /// documentary, independently-certified merchant and durable idempotency
    /// evidence. This method does not create or accept such evidence itself.
    ///
    /// A timeout/5xx returns Unavailable: never treat it as a decline nor
    /// retry automatically (money may already have moved).
    pub async fn execute(
        &self,
        operation: NativeOperation<'_>,
        governed_body: &Value,
        stable_key: &str,
    ) -> Result<ProviderObservation, HyperSwitchError> {
        let path = operation.path()?;
        let key = idempotency_key(stable_key)?;
        if !governed_body.is_object() {
            return Err(HyperSwitchError::InvalidOperation);
        }
        reject_sensitive_data(governed_body)?;
        let url = self.origin.join(&path).map_err(|_| HyperSwitchError::Configuration)?;
        let response = self.client.request(Method::POST, url)
            .header("api-key", &self.api_key)
            .header("idempotency-key", key)
            .header("accept", "application/json")
            .json(governed_body)
            .send().await.map_err(|_| HyperSwitchError::Unavailable)?;
        let status = response.status();
        if !status.is_success() {
            return Err(if status.is_server_error() || status.as_u16() == 429 {
                HyperSwitchError::Unavailable
            } else {
                HyperSwitchError::Rejected
            });
        }
        if response.content_length().is_some_and(|len| len > MAX_PROVIDER_RESPONSE_BYTES as u64) {
            return Err(HyperSwitchError::InvalidResponse);
        }
        let bytes = response.bytes().await.map_err(|_| HyperSwitchError::Unavailable)?;
        if bytes.len() > MAX_PROVIDER_RESPONSE_BYTES {
            return Err(HyperSwitchError::InvalidResponse);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| HyperSwitchError::InvalidResponse)?;
        let field = if operation == NativeOperation::Refund { "refund_id" } else { "payment_id" };
        let native_reference = value.get(field).and_then(Value::as_str)
            .filter(|s| (3..=120).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
            .ok_or(HyperSwitchError::InvalidResponse)?;
        let provider_status = value.get("status").and_then(Value::as_str)
            .filter(|s| !s.is_empty() && s.len() <= 64
                && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))
            .ok_or(HyperSwitchError::InvalidResponse)?;
        Ok(ProviderObservation {
            native_reference: native_reference.into(),
            provider_status: provider_status.into(),
        })
    }
}
