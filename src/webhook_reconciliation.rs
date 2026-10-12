//! LA-05E7: HyperSwitch HMAC-SHA512 webhook ingress and immutable receipts.
//! This is a trust boundary, NOT public HTTP route activation. The WebhookSource
//! is installed only through trusted deployment configuration. A signed PSP
//! event is not a bank settlement or an ERP invoice payment.
use aws_lc_rs::hmac;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio_postgres::NoTls;
use uuid::Uuid;

#[derive(Clone)]
pub struct WebhookSource {
    pub tenant_id: String,
    pub legal_entity_id: String,
    pub merchant_id: String,
    pub provider_engine_instance_id: String,
    pub provider_environment: String,
    /// From configured HyperSwitch business profile payment_response_hash_key.
    /// A webhook, tenant request or URL path can NEVER select its own key.
    pub signing_key: Vec<u8>,
}

impl std::fmt::Debug for WebhookSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookSource")
            .field("merchant_id", &self.merchant_id)
            .field(
                "provider_engine_instance_id",
                &self.provider_engine_instance_id,
            )
            .field("secret", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookError {
    SignatureDenied,
    UnrecognisedEvent,
    ScopeMismatch,
    Unavailable,
    ReconciliationRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookResult {
    Stored {
        execution_id: Uuid,
        receipt_id: Uuid,
    },
    Duplicate {
        execution_id: Uuid,
    },
}

pub struct NativeWebhookReconciler {
    database_url: String,
    trusted_source: WebhookSource,
}

fn limited_id(value: Option<&Value>, max: usize) -> Option<&str> {
    value.and_then(Value::as_str).filter(|s| {
        (3..=max).contains(&s.len())
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    })
}

fn status_for_event(value: &str) -> Option<&'static str> {
    match value {
        "payment_authorized" => Some("AUTHORIZED"),
        "payment_captured" | "payment_succeeded" => Some("CAPTURED"),
        "payment_failed" => Some("DECLINED"),
        "payment_processing" => Some("PENDING"),
        "payment_cancelled" => Some("VOIDED"),
        "refund_succeeded" => Some("REFUNDED"),
        _ => None,
    }
}

/// Validate RAW bytes rather than deserializing and re-serializing them.
/// HyperSwitch's outgoing webhook documentation specifies
/// x-webhook-signature-512 = hex(HMAC-SHA512(payment_response_hash_key,body)).
pub fn verify_hyperswitch_hmac512(
    body: &[u8],
    signature_hex: &str,
    configured_key: &[u8],
) -> Result<(), WebhookError> {
    if body.is_empty()
        || body.len() > 32 * 1024
        || configured_key.len() < 32
        || configured_key.len() > 4096
        || signature_hex.len() != 128
    {
        return Err(WebhookError::SignatureDenied);
    }
    let signature = hex::decode(signature_hex).map_err(|_| WebhookError::SignatureDenied)?;
    let key = hmac::Key::new(hmac::HMAC_SHA512, configured_key);
    hmac::verify(&key, body, &signature).map_err(|_| WebhookError::SignatureDenied)
}

impl NativeWebhookReconciler {
    /// Local integration only. Live webhook reception requires TLS termination,
    /// independently certified merchant profile and production DB TLS.
    pub fn local_integration(
        database_url: String,
        source: WebhookSource,
    ) -> Result<Self, WebhookError> {
        let address: tokio_postgres::Config = database_url
            .parse()
            .map_err(|_| WebhookError::ScopeMismatch)?;
        if address.get_hosts().is_empty()
            || address.get_hosts().iter().any(|host| {
                !matches!(host,
                tokio_postgres::config::Host::Tcp(h)
                if h == "127.0.0.1" || h == "localhost" || h == "::1")
            })
            || source.tenant_id.len() < 6
            || source.legal_entity_id.len() < 3
            || source.merchant_id.len() < 3
            || source.provider_engine_instance_id.len() < 3
            || !matches!(source.provider_environment.as_str(), "SANDBOX" | "STAGING")
            || source.signing_key.len() < 32
        {
            return Err(WebhookError::ScopeMismatch);
        }
        Ok(Self {
            database_url,
            trusted_source: source,
        })
    }

    pub async fn receive_signed(
        &self,
        raw: &[u8],
        signature_512: &str,
    ) -> Result<WebhookResult, WebhookError> {
        verify_hyperswitch_hmac512(raw, signature_512, &self.trusted_source.signing_key)?;
        let payload: Value =
            serde_json::from_slice(raw).map_err(|_| WebhookError::UnrecognisedEvent)?;
        let source = &self.trusted_source;
        let event_id =
            limited_id(payload.get("event_id"), 180).ok_or(WebhookError::UnrecognisedEvent)?;
        let merchant =
            limited_id(payload.get("merchant_id"), 160).ok_or(WebhookError::UnrecognisedEvent)?;
        if merchant != source.merchant_id {
            return Err(WebhookError::ScopeMismatch);
        }
        let event_type = payload
            .get("event_type")
            .and_then(Value::as_str)
            .ok_or(WebhookError::UnrecognisedEvent)?;
        let status = status_for_event(event_type).ok_or(WebhookError::UnrecognisedEvent)?;
        let content = payload
            .get("content")
            .ok_or(WebhookError::UnrecognisedEvent)?;
        let object = content
            .get("object")
            .ok_or(WebhookError::UnrecognisedEvent)?;
        let payment_id =
            limited_id(object.get("payment_id"), 160).ok_or(WebhookError::UnrecognisedEvent)?;
        if content.get("type").and_then(Value::as_str) != Some("payment_details") {
            return Err(WebhookError::UnrecognisedEvent);
        }
        let amount_minor = object
            .get("amount")
            .and_then(Value::as_i64)
            .filter(|a| *a > 0)
            .ok_or(WebhookError::UnrecognisedEvent)?;
        let currency = object
            .get("currency")
            .and_then(Value::as_str)
            .filter(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_uppercase()))
            .ok_or(WebhookError::UnrecognisedEvent)?;
        // Require a configured connector merchant, not a client-provided
        // account selector. Actual source identity is the trusted config slot.
        let hash = hex::encode(Sha256::digest(raw));
        let (mut db, connection) = tokio_postgres::connect(&self.database_url, NoTls)
            .await
            .map_err(|_| WebhookError::Unavailable)?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let tx = db
            .transaction()
            .await
            .map_err(|_| WebhookError::Unavailable)?;
        tx.query_one(
            "SELECT set_config('baobab.tenant_id',$1,true)",
            &[&source.tenant_id],
        )
        .await
        .map_err(|_| WebhookError::Unavailable)?;
        tx.query_one(
            "SELECT set_config('baobab.responsible_legal_entity_id',$1,true)",
            &[&source.legal_entity_id],
        )
        .await
        .map_err(|_| WebhookError::Unavailable)?;
        // A webhook MUST resolve a uniquely recorded PSP intent observation.
        // Never accept a payment_id without a known canonical execution.
        let rows = tx
            .query(
                "SELECT DISTINCT n.execution_id, n.amount_minor, n.currency_code
               FROM payment_ops.native_execution n
               JOIN payment_ops.transaction_outbox o
                 ON o.execution_id=n.execution_id AND o.tenant_id=n.tenant_id
                AND o.responsible_legal_entity_id=n.responsible_legal_entity_id
              WHERE n.tenant_id=$1 AND n.responsible_legal_entity_id=$2
                AND o.event_type='payments.provider-response.observed.v1'
                AND o.event_payload->>'provider_reference'=$3
                AND o.event_payload->>'merchant_id'=$4
                AND o.event_payload->>'provider_engine_instance_id'=$5
                AND o.event_payload->>'provider_environment'=$6",
                &[&source.tenant_id, &source.legal_entity_id, &payment_id,
                &source.merchant_id, &source.provider_engine_instance_id,
                &source.provider_environment],
            )
            .await
            .map_err(|_| WebhookError::Unavailable)?;
        if rows.len() != 1
            || rows[0].get::<_, i64>("amount_minor") != amount_minor
            || rows[0].get::<_, String>("currency_code") != currency
        {
            return Err(WebhookError::ScopeMismatch);
        }
        let execution_id: Uuid = rows[0].get("execution_id");
        let duplicate = tx
            .query_opt(
                "SELECT receipt_id,execution_id,verified_payload_sha256
               FROM payment_ops.provider_receipt
              WHERE provider_name='HYPERSWITCH'
                AND provider_engine_instance_id=$1 AND provider_environment=$2
                AND provider_event_id=$3",
                &[
                    &source.provider_engine_instance_id,
                    &source.provider_environment,
                    &event_id,
                ],
            )
            .await
            .map_err(|_| WebhookError::Unavailable)?;
        if let Some(record) = duplicate {
            if record.get::<_, Uuid>("execution_id") == execution_id
                && record.get::<_, String>("verified_payload_sha256") == hash
            {
                return Ok(WebhookResult::Duplicate { execution_id });
            }
            return Err(WebhookError::ReconciliationRequired);
        }
        let record = tx
            .query_one(
                "INSERT INTO payment_ops.provider_receipt
              (tenant_id, execution_id, responsible_legal_entity_id,
               provider_name, provider_engine_instance_id, provider_environment,
               provider_event_id, observed_status, verified_payload_sha256)
              VALUES($1,$2,$3,'HYPERSWITCH',$4,$5,$6,$7,$8)
              RETURNING receipt_id",
                &[
                    &source.tenant_id,
                    &execution_id,
                    &source.legal_entity_id,
                    &source.provider_engine_instance_id,
                    &source.provider_environment,
                    &event_id,
                    &status,
                    &hash,
                ],
            )
            .await
            .map_err(|_| WebhookError::ReconciliationRequired)?;
        let receipt_id: Uuid = record.get(0);
        tx.execute(
            "INSERT INTO payment_ops.transaction_outbox
              (tenant_id,responsible_legal_entity_id,execution_id,event_type,event_payload,correlation_id)
              VALUES($1,$2,$3,'payments.provider-webhook.verified.v1',$4,$5)",
            &[
                &source.tenant_id, &source.legal_entity_id, &execution_id,
                &json!({"receipt_id":receipt_id,"event_type":event_type,"status":status,
                    "bank_settlement_verified":false}), &Uuid::now_v7(),
            ],
        ).await.map_err(|_| WebhookError::Unavailable)?;
        tx.commit().await.map_err(|_| WebhookError::Unavailable)?;
        Ok(WebhookResult::Stored {
            execution_id,
            receipt_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> WebhookSource {
        WebhookSource {
            tenant_id: "tn_syntheticscope".into(),
            legal_entity_id: "LE-SYNTHETIC-ZA".into(),
            merchant_id: "merchant_synthetic".into(),
            provider_engine_instance_id: "provider_synthetic".into(),
            provider_environment: "SANDBOX".into(),
            signing_key: vec![72; 64],
        }
    }
    #[test]
    fn exact_raw_signature_and_configured_profile_are_required() {
        let body = br#"{"merchant_id":"merchant_synthetic","event_id":"evt_synthetic_01","event_type":"payment_captured","content":{"type":"payment_details","object":{"payment_id":"pay_synthetic","amount":1200,"currency":"ZAR"}}}"#;
        let key = hmac::Key::new(hmac::HMAC_SHA512, &source().signing_key);
        let sig = hex::encode(hmac::sign(&key, body).as_ref());
        assert_eq!(
            verify_hyperswitch_hmac512(body, &sig, &source().signing_key),
            Ok(())
        );
        assert_eq!(
            verify_hyperswitch_hmac512(b"{}", &sig, &source().signing_key),
            Err(WebhookError::SignatureDenied)
        );
        assert_eq!(
            verify_hyperswitch_hmac512(body, &"0".repeat(128), &source().signing_key),
            Err(WebhookError::SignatureDenied)
        );
        assert_eq!(status_for_event("payment_captured"), Some("CAPTURED"));
        assert_eq!(status_for_event("payment_processing"), Some("PENDING"));
        assert_eq!(status_for_event("settlement_succeeded"), None);
    }
    #[test]
    fn no_remote_cleartext_db_or_unsigned_production_activation() {
        let s = source();
        assert!(matches!(
            NativeWebhookReconciler::local_integration("host=payments-remote".into(), s.clone()),
            Err(WebhookError::ScopeMismatch)
        ));
        let mut production = s;
        production.provider_environment = "PRODUCTION".into();
        assert!(matches!(
            NativeWebhookReconciler::local_integration("host=127.0.0.1".into(), production),
            Err(WebhookError::ScopeMismatch)
        ));
    }
}
