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
            Self::Confirm { payment_id } => {
                Ok(format!("payments/{}/confirm", record_id(payment_id)?))
            }
            Self::Capture { payment_id } => {
                Ok(format!("payments/{}/capture", record_id(payment_id)?))
            }
            Self::Cancel { payment_id } => {
                Ok(format!("payments/{}/cancel", record_id(payment_id)?))
            }
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
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        Ok(id)
    } else {
        Err(HyperSwitchError::InvalidOperation)
    }
}

fn idempotency_key(value: &str) -> Result<&str, HyperSwitchError> {
    if (16..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    {
        Ok(value)
    } else {
        Err(HyperSwitchError::InvalidOperation)
    }
}

/// Reject un-tokenised card/bank details even if a caller accidentally places
/// them in a provider request. Consumers pass only PSP-issued method tokens.
/// Only tokenised scalar provider fields are allowed in this transport
/// increment. Unknown or nested fields (including IBAN, direct-debit mandates,
/// raw bank accounts and PCI card blocks) are deliberately denied.
fn reject_sensitive_data(value: &Value) -> Result<(), HyperSwitchError> {
    const TOKEN_ONLY_FIELDS: &[&str] = &[
        "amount",
        "amount_to_capture",
        "currency",
        "capture_method",
        "confirm",
        "profile_id",
        "payment_id",
        "refund_id",
        "payment_token",
        "payment_method",
        "payment_method_type",
        "reason",
    ];
    let fields = value
        .as_object()
        .ok_or(HyperSwitchError::InvalidOperation)?;
    for (key, value) in fields {
        if !TOKEN_ONLY_FIELDS.contains(&key.as_str())
            || !(value.is_string() || value.is_boolean() || value.is_number())
        {
            return Err(HyperSwitchError::InvalidOperation);
        }
    }
    Ok(())
}

impl HyperSwitchTransport {
    /// Public construction never permits plaintext provider traffic.
    pub fn new(origin: &str, api_key: String) -> Result<Self, HyperSwitchError> {
        Self::with_policy(origin, api_key, false)
    }

    #[cfg(test)]
    pub(crate) fn loopback_test_client(origin: &str) -> Result<Self, HyperSwitchError> {
        Self::with_policy(origin, "synthetic-only-secret".into(), true)
    }

    fn with_policy(
        origin: &str,
        api_key: String,
        test_loopback_http: bool,
    ) -> Result<Self, HyperSwitchError> {
        let url = Url::parse(origin).map_err(|_| HyperSwitchError::Configuration)?;
        let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !((url.scheme() == "https")
            || (test_loopback_http && loopback && url.scheme() == "http"))
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
        Ok(Self {
            client,
            origin: url,
            api_key,
        })
    }

    /// Validate exact token-only shape before a durable operation is reserved.
    /// Prevent local request rejection from consuming an idempotency key.
    pub(crate) fn validate_operation(
        operation: &NativeOperation<'_>,
        body: &Value,
        stable_key: &str,
    ) -> Result<(), HyperSwitchError> {
        operation.path()?;
        idempotency_key(stable_key)?;
        reject_sensitive_data(body)?;
        Ok(())
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
        Self::validate_operation(&operation, governed_body, stable_key)?;
        let url = self
            .origin
            .join(&path)
            .map_err(|_| HyperSwitchError::Configuration)?;
        let mut response = self
            .client
            .request(Method::POST, url)
            .header("api-key", self.api_key.as_str())
            .header("idempotency-key", key)
            .header("accept", "application/json")
            .json(governed_body)
            .send()
            .await
            .map_err(|_| HyperSwitchError::Unavailable)?;
        let status = response.status();
        if !status.is_success() {
            return Err(if status.is_server_error() || status.as_u16() == 429 {
                HyperSwitchError::Unavailable
            } else {
                HyperSwitchError::Rejected
            });
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_PROVIDER_RESPONSE_BYTES as u64)
        {
            return Err(HyperSwitchError::InvalidResponse);
        }
        // Response::bytes() buffers an unbounded body when Content-Length
        // is absent or false. Stop reading at the policy cap even for chunked
        // or deliberately misreported provider responses.
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| HyperSwitchError::Unavailable)?
        {
            if chunk.len() > MAX_PROVIDER_RESPONSE_BYTES.saturating_sub(body.len()) {
                return Err(HyperSwitchError::InvalidResponse);
            }
            body.extend_from_slice(&chunk);
        }
        let value: Value =
            serde_json::from_slice(&body).map_err(|_| HyperSwitchError::InvalidResponse)?;
        let field = if operation == NativeOperation::Refund {
            "refund_id"
        } else {
            "payment_id"
        };
        let native_reference = value
            .get(field)
            .and_then(Value::as_str)
            .filter(|s| {
                (3..=120).contains(&s.len())
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            })
            .ok_or(HyperSwitchError::InvalidResponse)?;
        let provider_status = value
            .get("status")
            .and_then(Value::as_str)
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 64
                    && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
            })
            .ok_or(HyperSwitchError::InvalidResponse)?;
        Ok(ProviderObservation {
            native_reference: native_reference.into(),
            provider_status: provider_status.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::extract::Json;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;
    use serde_json::json;

    fn intent() -> Value {
        json!({
            "amount": 1200, "currency": "ZAR", "capture_method": "manual",
            "confirm": false, "profile_id": "prof_synthetic_not_certified"
        })
    }

    #[test]
    fn rejects_insecure_and_untrusted_configuration_without_exposing_keys() {
        for origin in [
            "http://payments.example.com/",
            "***payments.example.com/",
            "https://payments.example.com/path/",
            "https://payments.example.com/?token=foo",
            "http://127.0.0.1:8080/",
        ] {
            assert_eq!(
                HyperSwitchTransport::new(origin, "synthetic".into()).unwrap_err(),
                HyperSwitchError::Configuration
            );
        }
        let client =
            HyperSwitchTransport::new("https://payments.example.com/", "synthetic-secret".into())
                .unwrap();
        let debug = format!("{client:?}");
        assert!(!debug.contains("synthetic-secret"));
        assert!(debug.contains("<redacted>"));
    }

    #[tokio::test]
    async fn sends_real_http_with_server_only_key_and_stable_idempotency_header() {
        let app = Router::new().route(
            "/payments",
            post(|headers: HeaderMap, Json(body): Json<Value>| async move {
                assert_eq!(headers["api-key"], "synthetic-only-secret");
                assert_eq!(headers["idempotency-key"], "stable-test-key-0001");
                assert_eq!(body["amount"], 1200);
                assert_eq!(body["confirm"], false);
                Json(json!({"payment_id":"pay_synthetic_001","status":"requires_payment_method"}))
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let transport =
            HyperSwitchTransport::loopback_test_client(&format!("http://{addr}/")).unwrap();
        let response = transport
            .execute(NativeOperation::Create, &intent(), "stable-test-key-0001")
            .await
            .unwrap();
        assert_eq!(response.native_reference, "pay_synthetic_001");
        assert_eq!(response.provider_status, "requires_payment_method");
        task.abort();
    }

    #[tokio::test]
    async fn refuses_card_data_path_traversal_bad_key_and_unsafe_provider_response() {
        let client =
            HyperSwitchTransport::new("https://payments.example.com/", "synthetic".into()).unwrap();
        for bad in [
            json!({"payment_method_data":{"card":{"card_number":"4242424242424242"}}}),
            json!({"account_number":"12345"}),
            json!({"iban":"DE89370400440532013000"}),
            json!({"payment_method_data":{"sepa":{"iban":"DE89370400440532013000"}}}),
            json!({"payment_method_data":{"bank":{"routing_number":"123"}}}),
            json!({"card_cvc":"001"}),
        ] {
            assert_eq!(
                client
                    .execute(NativeOperation::Create, &bad, "stable-test-key-0001")
                    .await
                    .unwrap_err(),
                HyperSwitchError::InvalidOperation
            );
        }
        assert_eq!(
            client
                .execute(
                    NativeOperation::Confirm {
                        payment_id: "../other"
                    },
                    &intent(),
                    "stable-test-key-0001"
                )
                .await
                .unwrap_err(),
            HyperSwitchError::InvalidOperation
        );
        assert_eq!(
            client
                .execute(NativeOperation::Create, &intent(), "short")
                .await
                .unwrap_err(),
            HyperSwitchError::InvalidOperation
        );
        let app = Router::new().route(
            "/payments",
            post(|| async {
                Json(json!({"payment_id":"pay_synthetic_001", "status": "unknown;success"}))
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client =
            HyperSwitchTransport::loopback_test_client(&format!("http://{addr}/")).unwrap();
        assert_eq!(
            client
                .execute(NativeOperation::Create, &intent(), "stable-test-key-0001")
                .await
                .unwrap_err(),
            HyperSwitchError::InvalidResponse
        );
        task.abort();
    }

    #[tokio::test]
    async fn refuses_oversized_chunked_response_without_buffering_whole_body() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 8192];
            let _ = socket.read(&mut request).await;
            let _ = socket.write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: application/json\r\n\r\n",
            ).await;
            for _ in 0..75 {
                if socket.write_all(b"400\r\n").await.is_err() {
                    break;
                }
                if socket.write_all(&vec![b'x'; 1024]).await.is_err() {
                    break;
                }
                if socket.write_all(b"\r\n").await.is_err() {
                    break;
                }
            }
            let _ = socket.write_all(b"0\r\n\r\n").await;
        });
        let client =
            HyperSwitchTransport::loopback_test_client(&format!("http://{addr}/")).unwrap();
        assert_eq!(
            client
                .execute(NativeOperation::Create, &intent(), "stable-test-key-0001")
                .await
                .unwrap_err(),
            HyperSwitchError::InvalidResponse
        );
        task.abort();
    }

    #[tokio::test]
    async fn rejects_provider_error_without_automatic_money_retry() {
        let app = Router::new().route(
            "/payments",
            post(|| async {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({"error":"provider down"})),
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client =
            HyperSwitchTransport::loopback_test_client(&format!("http://{addr}/")).unwrap();
        assert_eq!(
            client
                .execute(NativeOperation::Create, &intent(), "stable-test-key-0001")
                .await
                .unwrap_err(),
            HyperSwitchError::Unavailable
        );
        task.abort();
    }
}
