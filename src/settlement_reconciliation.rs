//! LA-05E7: three-way order/PSP/bank reconciliation over immutable evidence.
//! Bank and PSP proof MUST be independently verified by externally certified
//! adapters. This code matches facts; it does not mint new provider authority.
use sha2::{Digest, Sha256};
use tokio_postgres::NoTls;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    Psp,
    Bank,
}
impl EvidenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Psp => "PSP",
            Self::Bank => "BANK",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatementLine {
    pub source_kind: EvidenceKind,
    pub independent_source: String,
    pub engine_instance_id: String,
    pub environment: String,
    pub batch_reference: String,
    pub source_event_id: String,
    pub canonical_payment_reference: String,
    pub currency: String,
    pub gross_minor: i64,
    pub fee_minor: i64,
    pub net_minor: i64,
    pub verified_method: String,
    /// Exact signed/attested upstream statement bytes, not a caller-supplied
    /// SHA. Canonical source identity MUST be checked by EvidenceVerifier.
    pub raw_source: Vec<u8>,
}
#[derive(Debug, Clone)]
pub struct AttributionScope {
    pub tenant_id: String,
    pub responsible_legal_entity_id: String,
    pub execution_id: Uuid,
    pub correlation_id: Uuid,
}

/// Verified independently against bank/PSP source: not an opaque string
/// annotation or caller's claim. Production must install two different
/// cert-validated verifiers with separated credentials.
pub trait EvidenceVerifier: Send + Sync {
    fn independently_verified(&self, statement: &StatementLine) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationError {
    InvalidEvidence,
    UntrustedSource,
    DatabaseUnavailable,
    AlreadyRecorded,
}

pub struct LocalSettlementJournal {
    database_url: String,
}
impl LocalSettlementJournal {
    pub fn new_local(database_url: String, environment: &str) -> Result<Self, ReconciliationError> {
        let address: tokio_postgres::Config = database_url
            .parse()
            .map_err(|_| ReconciliationError::InvalidEvidence)?;
        if !matches!(environment, "test" | "development" | "integration")
            || address.get_hosts().is_empty()
            || address.get_hosts().iter().any(|host| {
                !matches!(host,
                tokio_postgres::config::Host::Tcp(h)
                if h=="localhost" || h=="127.0.0.1" || h=="::1")
            })
        {
            return Err(ReconciliationError::UntrustedSource);
        }
        Ok(Self { database_url })
    }

    /// Source-specific verification is an independent adapter responsibility.
    /// Never select a verifier based only on untrusted statement data.
    pub async fn append_verified(
        &self,
        actor: &AttributionScope,
        statement: &StatementLine,
        verifier: &dyn EvidenceVerifier,
    ) -> Result<Uuid, ReconciliationError> {
        if !verifier.independently_verified(statement)
            || statement.raw_source.is_empty()
            || statement.raw_source.len() > 256 * 1024
            || statement.independent_source.trim().len() < 3
            || statement.source_event_id.trim().len() < 3
            || statement.verified_method.trim().len() < 3
            || statement.engine_instance_id.trim().len() < 3
            || statement.batch_reference.trim().len() < 3
            || statement.canonical_payment_reference.trim().len() < 3
            || statement.currency.len() != 3
            || statement.gross_minor <= 0
            || statement.fee_minor < 0
            || statement.net_minor < 0
            || statement.gross_minor.checked_sub(statement.fee_minor) != Some(statement.net_minor)
            || !matches!(statement.environment.as_str(), "SANDBOX" | "STAGING")
        {
            return Err(ReconciliationError::InvalidEvidence);
        }
        let hash = hex::encode(Sha256::digest(&statement.raw_source));
        let (mut db, connection) = tokio_postgres::connect(&self.database_url, NoTls)
            .await
            .map_err(|_| ReconciliationError::DatabaseUnavailable)?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let tx = db
            .transaction()
            .await
            .map_err(|_| ReconciliationError::DatabaseUnavailable)?;
        tx.query_one(
            "SELECT set_config('baobab.tenant_id',$1,true)",
            &[&actor.tenant_id],
        )
        .await
        .map_err(|_| ReconciliationError::DatabaseUnavailable)?;
        tx.query_one(
            "SELECT set_config('baobab.responsible_legal_entity_id',$1,true)",
            &[&actor.responsible_legal_entity_id],
        )
        .await
        .map_err(|_| ReconciliationError::DatabaseUnavailable)?;
        let row = tx
            .query_one(
                "INSERT INTO payment_ops.settlement_evidence
              (tenant_id,responsible_legal_entity_id,execution_id,source_kind,independent_source,
               provider_engine_instance_id,provider_environment,batch_reference,external_event_id,
               payment_reference,currency_code,gross_minor,fee_minor,net_minor,
               verified_source_proof_sha256,verification_method)
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
             RETURNING evidence_id",
                &[
                    &actor.tenant_id,
                    &actor.responsible_legal_entity_id,
                    &actor.execution_id,
                    &statement.source_kind.as_str(),
                    &statement.independent_source,
                    &statement.engine_instance_id,
                    &statement.environment,
                    &statement.batch_reference,
                    &statement.source_event_id,
                    &statement.canonical_payment_reference,
                    &statement.currency,
                    &statement.gross_minor,
                    &statement.fee_minor,
                    &statement.net_minor,
                    &hash,
                    &statement.verified_method,
                ],
            )
            .await
            .map_err(|_| ReconciliationError::AlreadyRecorded)?;
        let id: Uuid = row.get(0);
        tx.commit()
            .await
            .map_err(|_| ReconciliationError::DatabaseUnavailable)?;
        Ok(id)
    }

    /// Keep a signed/independently attested bank/PSP line even when no
    /// canonical payment execution has been identified. This is an immutable
    /// exception, not a matched or paid state. Later resolution inserts
    /// separate attribution evidence rather than rewriting this row.
    pub async fn append_unmatched_verified(
        &self, actor: &AttributionScope, statement: &StatementLine,
        verifier: &dyn EvidenceVerifier,
    ) -> Result<Uuid, ReconciliationError> {
        if !verifier.independently_verified(statement)
            || statement.raw_source.is_empty()
            || statement.raw_source.len() > 256*1024
            || statement.independent_source.trim().len() < 3
            || statement.source_event_id.trim().len() < 3
            || statement.batch_reference.trim().len() < 3
            || statement.verified_method.trim().len() < 3
            || statement.engine_instance_id.trim().len() < 3
            || statement.canonical_payment_reference.trim().len() < 3
            || statement.currency.len() != 3
            || statement.gross_minor <= 0 || statement.fee_minor < 0
            || statement.net_minor < 0
            || statement.gross_minor.checked_sub(statement.fee_minor) != Some(statement.net_minor)
            || !matches!(statement.environment.as_str(),"SANDBOX"|"STAGING")
        { return Err(ReconciliationError::InvalidEvidence); }
        let hash = hex::encode(Sha256::digest(&statement.raw_source));
        let (mut db, connection) = tokio_postgres::connect(&self.database_url,NoTls)
            .await.map_err(|_|ReconciliationError::DatabaseUnavailable)?;
        tokio::spawn(async move { let _ = connection.await; });
        let tx = db.transaction().await.map_err(|_|ReconciliationError::DatabaseUnavailable)?;
        tx.query_one("SELECT set_config('baobab.tenant_id',$1,true)", &[&actor.tenant_id])
            .await.map_err(|_|ReconciliationError::DatabaseUnavailable)?;
        tx.query_one("SELECT set_config('baobab.responsible_legal_entity_id',$1,true)",
            &[&actor.responsible_legal_entity_id]).await
            .map_err(|_|ReconciliationError::DatabaseUnavailable)?;
        let record=tx.query_one(
            "INSERT INTO payment_ops.unmatched_settlement_evidence
             (tenant_id,responsible_legal_entity_id,source_kind,independent_source,
              provider_engine_instance_id,provider_environment,batch_reference,external_event_id,
              payment_reference,currency_code,gross_minor,fee_minor,net_minor,
              verified_source_proof_sha256,verification_method)
              VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
              RETURNING evidence_id",
            &[
                &actor.tenant_id,&actor.responsible_legal_entity_id,
                &statement.source_kind.as_str(),&statement.independent_source,
                &statement.engine_instance_id,&statement.environment,
                &statement.batch_reference,&statement.source_event_id,
                &statement.canonical_payment_reference,&statement.currency,
                &statement.gross_minor,&statement.fee_minor,&statement.net_minor,
                &hash,&statement.verified_method,
            ],
        ).await.map_err(|_|ReconciliationError::AlreadyRecorded)?;
        let id: Uuid=record.get(0);
        tx.commit().await.map_err(|_|ReconciliationError::DatabaseUnavailable)?;
        Ok(id)
    }

    /// Append a deterministic reconciliation decision; a MATCHED pair is
    /// still NOT an authorisation to disburse funds or post an ERP invoice.
    pub async fn reconcile(
        &self,
        actor: &AttributionScope,
        psp: Uuid,
        bank: Uuid,
    ) -> Result<String, ReconciliationError> {
        let (mut db, connection) = tokio_postgres::connect(&self.database_url, NoTls)
            .await
            .map_err(|_| ReconciliationError::DatabaseUnavailable)?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let tx = db
            .transaction()
            .await
            .map_err(|_| ReconciliationError::DatabaseUnavailable)?;
        tx.query_one(
            "SELECT set_config('baobab.tenant_id',$1,true)",
            &[&actor.tenant_id],
        )
        .await
        .map_err(|_| ReconciliationError::DatabaseUnavailable)?;
        tx.query_one(
            "SELECT set_config('baobab.responsible_legal_entity_id',$1,true)",
            &[&actor.responsible_legal_entity_id],
        )
        .await
        .map_err(|_| ReconciliationError::DatabaseUnavailable)?;
        let row = tx
            .query_one(
                "SELECT payment_ops.reconcile_verified_settlement($1,$2,$3,$4,$5,$6)",
                &[
                    &actor.tenant_id,
                    &actor.responsible_legal_entity_id,
                    &actor.execution_id,
                    &psp,
                    &bank,
                    &actor.correlation_id,
                ],
            )
            .await
            .map_err(|_| ReconciliationError::InvalidEvidence)?;
        let outcome: String = row.get(0);
        tx.commit()
            .await
            .map_err(|_| ReconciliationError::DatabaseUnavailable)?;
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Deny;
    impl EvidenceVerifier for Deny {
        fn independently_verified(&self, _: &StatementLine) -> bool {
            false
        }
    }
    #[tokio::test]
    async fn unverified_external_statement_never_reaches_journal() {
        let journal =
            LocalSettlementJournal::new_local("host=127.0.0.1 user=synthetic".into(), "test")
                .unwrap();
        let line = StatementLine {
            source_kind: EvidenceKind::Bank,
            independent_source: "bank.synthetic".into(),
            engine_instance_id: "instance_test".into(),
            environment: "SANDBOX".into(),
            batch_reference: "batch-test".into(),
            source_event_id: "bank-event-test".into(),
            canonical_payment_reference: "payment-test".into(),
            currency: "ZAR".into(),
            gross_minor: 1000,
            fee_minor: 0,
            net_minor: 1000,
            verified_method: "test-not-certified".into(),
            raw_source: vec![1, 2, 3],
        };
        let actor = AttributionScope {
            tenant_id: "tn_synthetic".into(),
            responsible_legal_entity_id: "LE-TEST".into(),
            execution_id: Uuid::now_v7(),
            correlation_id: Uuid::now_v7(),
        };
        assert_eq!(
            journal.append_verified(&actor, &line, &Deny).await,
            Err(ReconciliationError::InvalidEvidence)
        );
        assert!(matches!(
            LocalSettlementJournal::new_local("host=db.internal".into(), "production"),
            Err(ReconciliationError::UntrustedSource)
        ));
    }
}
