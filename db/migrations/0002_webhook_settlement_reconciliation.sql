-- LA-05E7: signed provider evidence and independent three-way settlement.
-- Install AFTER 0001_native_payment_journal.sql in PostgreSQL 17.
BEGIN;

ALTER TABLE payment_ops.transaction_outbox
DROP CONSTRAINT transaction_outbox_event_type_check;
ALTER TABLE payment_ops.transaction_outbox
ADD CONSTRAINT transaction_outbox_event_type_check CHECK (event_type IN (
  'payments.native-operation.reserved.v1',
  'payments.provider-response.observed.v1',
  'payments.settlement.reconciliation-requested.v1',
  'payments.provider-webhook.verified.v1',
  'payments.settlement.matched.v1',
  'payments.settlement.exception.v1'
));

CREATE TABLE payment_ops.settlement_evidence (
    evidence_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL,
    execution_id uuid NOT NULL,
    source_kind text NOT NULL CHECK (source_kind IN ('PSP','BANK')),
    independent_source text NOT NULL CHECK (length(btrim(independent_source)) BETWEEN 3 AND 160),
    provider_engine_instance_id text NOT NULL,
    provider_environment text NOT NULL CHECK (provider_environment IN ('SANDBOX','STAGING','PRODUCTION')),
    batch_reference text NOT NULL CHECK (length(btrim(batch_reference)) BETWEEN 3 AND 180),
    external_event_id text NOT NULL CHECK (length(btrim(external_event_id)) BETWEEN 3 AND 180),
    payment_reference text NOT NULL CHECK (length(btrim(payment_reference)) BETWEEN 3 AND 160),
    currency_code text NOT NULL CHECK (currency_code ~ '^[A-Z]{3}$'),
    gross_minor bigint NOT NULL CHECK (gross_minor > 0),
    fee_minor bigint NOT NULL CHECK (fee_minor >= 0),
    net_minor bigint NOT NULL CHECK (net_minor >= 0),
    verified_source_proof_sha256 text NOT NULL CHECK (verified_source_proof_sha256 ~ '^[a-f0-9]{64}$'),
    verification_method text NOT NULL CHECK (length(btrim(verification_method)) BETWEEN 3 AND 160),
    recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    CHECK (gross_minor = fee_minor + net_minor),
    FOREIGN KEY (tenant_id, responsible_legal_entity_id, execution_id)
      REFERENCES payment_ops.native_execution (tenant_id,responsible_legal_entity_id,execution_id),
    UNIQUE (source_kind, independent_source, provider_engine_instance_id, provider_environment, external_event_id)
);

-- Unmatched PSP/BANK evidence is a first-class immutable exception,
-- not lost because the native execution cannot yet be resolved.
CREATE TABLE payment_ops.unmatched_settlement_evidence (
    evidence_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL,
    source_kind text NOT NULL CHECK (source_kind IN ('PSP','BANK')),
    independent_source text NOT NULL CHECK (length(btrim(independent_source)) BETWEEN 3 AND 160),
    provider_engine_instance_id text NOT NULL,
    provider_environment text NOT NULL CHECK (provider_environment IN ('SANDBOX','STAGING','PRODUCTION')),
    batch_reference text NOT NULL,
    external_event_id text NOT NULL,
    payment_reference text NOT NULL,
    currency_code text NOT NULL CHECK (currency_code ~ '^[A-Z]{3}$'),
    gross_minor bigint NOT NULL CHECK (gross_minor > 0),
    fee_minor bigint NOT NULL CHECK (fee_minor >= 0),
    net_minor bigint NOT NULL CHECK (net_minor >= 0),
    verified_source_proof_sha256 text NOT NULL CHECK (verified_source_proof_sha256 ~ '^[a-f0-9]{64}$'),
    verification_method text NOT NULL,
    exception_reason text NOT NULL DEFAULT 'NATIVE_EXECUTION_NOT_IDENTIFIED'
       CHECK (exception_reason = 'NATIVE_EXECUTION_NOT_IDENTIFIED'),
    recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    CHECK (gross_minor = fee_minor + net_minor),
    UNIQUE (source_kind, independent_source, provider_engine_instance_id,
            provider_environment, external_event_id)
);

CREATE TABLE payment_ops.settlement_match (
    match_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL,
    execution_id uuid NOT NULL,
    psp_evidence_id uuid NOT NULL
        REFERENCES payment_ops.settlement_evidence(evidence_id),
    bank_evidence_id uuid NOT NULL
        REFERENCES payment_ops.settlement_evidence(evidence_id),
    state text NOT NULL CHECK (state IN ('MATCHED','EXCEPTION')),
    reason_code text NOT NULL CHECK (reason_code IN (
       'THREE_WAY_MATCH','BANK_AMOUNT_MISMATCH','CURRENCY_MISMATCH',
       'MISSING_INDEPENDENT_EVIDENCE','PROVIDER_SCOPE_MISMATCH')),
    correlation_id uuid NOT NULL,
    recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    FOREIGN KEY (tenant_id, responsible_legal_entity_id, execution_id)
      REFERENCES payment_ops.native_execution(tenant_id,responsible_legal_entity_id,execution_id),
    CHECK ((state='MATCHED') = (reason_code='THREE_WAY_MATCH')),
    UNIQUE (tenant_id, responsible_legal_entity_id, execution_id, psp_evidence_id, bank_evidence_id)
);

ALTER TABLE payment_ops.settlement_evidence ENABLE ROW LEVEL SECURITY;
ALTER TABLE payment_ops.settlement_evidence FORCE ROW LEVEL SECURITY;
ALTER TABLE payment_ops.settlement_match ENABLE ROW LEVEL SECURITY;
ALTER TABLE payment_ops.unmatched_settlement_evidence ENABLE ROW LEVEL SECURITY;
ALTER TABLE payment_ops.unmatched_settlement_evidence FORCE ROW LEVEL SECURITY;
ALTER TABLE payment_ops.settlement_match FORCE ROW LEVEL SECURITY;

CREATE POLICY settlement_evidence_actor ON payment_ops.settlement_evidence
  USING (tenant_id = nullif(current_setting('baobab.tenant_id',true),'')
    AND responsible_legal_entity_id = nullif(current_setting('baobab.responsible_legal_entity_id',true),''))
  WITH CHECK (tenant_id = nullif(current_setting('baobab.tenant_id',true),'')
    AND responsible_legal_entity_id = nullif(current_setting('baobab.responsible_legal_entity_id',true),''));
CREATE POLICY unmatched_settlement_actor ON payment_ops.unmatched_settlement_evidence
  USING (tenant_id = nullif(current_setting('baobab.tenant_id',true),'')
    AND responsible_legal_entity_id = nullif(current_setting('baobab.responsible_legal_entity_id',true),''))
  WITH CHECK (tenant_id = nullif(current_setting('baobab.tenant_id',true),'')
    AND responsible_legal_entity_id = nullif(current_setting('baobab.responsible_legal_entity_id',true),''));
CREATE POLICY settlement_match_actor ON payment_ops.settlement_match
  USING (tenant_id = nullif(current_setting('baobab.tenant_id',true),'')
    AND responsible_legal_entity_id = nullif(current_setting('baobab.responsible_legal_entity_id',true),''))
  WITH CHECK (tenant_id = nullif(current_setting('baobab.tenant_id',true),'')
    AND responsible_legal_entity_id = nullif(current_setting('baobab.responsible_legal_entity_id',true),''));

CREATE TRIGGER immutable_settlement_evidence BEFORE UPDATE OR DELETE
  ON payment_ops.settlement_evidence FOR EACH ROW
  EXECUTE FUNCTION payment_ops.refuse_historical_rewrite();
CREATE TRIGGER immutable_unmatched_settlement_evidence BEFORE UPDATE OR DELETE
 ON payment_ops.unmatched_settlement_evidence FOR EACH ROW
 EXECUTE FUNCTION payment_ops.refuse_historical_rewrite();
CREATE TRIGGER immutable_settlement_match BEFORE UPDATE OR DELETE
  ON payment_ops.settlement_match FOR EACH ROW
  EXECUTE FUNCTION payment_ops.refuse_historical_rewrite();

CREATE INDEX settlement_evidence_reference_idx
 ON payment_ops.settlement_evidence
 (tenant_id,responsible_legal_entity_id,payment_reference,source_kind,recorded_at);
-- One genuinely matched PSP/BANK pair per execution. A failed match is
-- append-only evidence, not an irreversible lockout against a later
-- independently verified corrected bank statement.
CREATE UNIQUE INDEX settlement_one_matched_execution
 ON payment_ops.settlement_match(tenant_id,responsible_legal_entity_id,execution_id)
 WHERE state='MATCHED';

CREATE INDEX settlement_match_execution_idx
 ON payment_ops.settlement_match
 (tenant_id,responsible_legal_entity_id,execution_id,recorded_at);

-- Guard direct database inserts too: no client with INSERT privilege may
-- fabricate a MATCHED fact by skipping the reconcile function.
CREATE FUNCTION payment_ops.guard_settlement_match_insert() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE
  p payment_ops.settlement_evidence%ROWTYPE;
  b payment_ops.settlement_evidence%ROWTYPE;
  n payment_ops.native_execution%ROWTYPE;
  actual_reason text;
  actual_state text;
BEGIN
  IF NEW.tenant_id IS DISTINCT FROM nullif(current_setting('baobab.tenant_id',true),'')
    OR NEW.responsible_legal_entity_id IS DISTINCT FROM
        nullif(current_setting('baobab.responsible_legal_entity_id',true),'') THEN
     RAISE EXCEPTION 'settlement actor denied' USING ERRCODE='42501';
  END IF;
  SELECT * INTO p FROM payment_ops.settlement_evidence
    WHERE evidence_id=NEW.psp_evidence_id AND source_kind='PSP'
      AND tenant_id=NEW.tenant_id
      AND responsible_legal_entity_id=NEW.responsible_legal_entity_id
      AND execution_id=NEW.execution_id;
  SELECT * INTO b FROM payment_ops.settlement_evidence
    WHERE evidence_id=NEW.bank_evidence_id AND source_kind='BANK'
      AND tenant_id=NEW.tenant_id
      AND responsible_legal_entity_id=NEW.responsible_legal_entity_id
      AND execution_id=NEW.execution_id;
  SELECT * INTO n FROM payment_ops.native_execution
    WHERE execution_id=NEW.execution_id
      AND tenant_id=NEW.tenant_id
      AND responsible_legal_entity_id=NEW.responsible_legal_entity_id;
  IF p.evidence_id IS NULL OR b.evidence_id IS NULL OR n.execution_id IS NULL THEN
    RAISE EXCEPTION 'verified PSP, independent bank and native obligation required'
      USING ERRCODE='23514';
  END IF;
  actual_reason = CASE
    WHEN p.gross_minor<>n.amount_minor OR p.currency_code<>n.currency_code
      OR p.payment_reference<>n.canonical_payment_reference
      THEN 'PROVIDER_SCOPE_MISMATCH'
    WHEN p.independent_source=b.independent_source THEN 'PROVIDER_SCOPE_MISMATCH'
    WHEN p.provider_engine_instance_id<>b.provider_engine_instance_id
      OR p.provider_environment<>b.provider_environment
      OR p.batch_reference<>b.batch_reference
      OR p.payment_reference<>b.payment_reference
      THEN 'PROVIDER_SCOPE_MISMATCH'
    WHEN p.currency_code<>b.currency_code THEN 'CURRENCY_MISMATCH'
    WHEN p.net_minor<>b.net_minor OR b.fee_minor<>0 OR b.gross_minor<>b.net_minor
      THEN 'BANK_AMOUNT_MISMATCH'
    ELSE 'THREE_WAY_MATCH' END;
  actual_state = CASE WHEN actual_reason='THREE_WAY_MATCH' THEN 'MATCHED' ELSE 'EXCEPTION' END;
  IF NEW.reason_code IS DISTINCT FROM actual_reason
    OR NEW.state IS DISTINCT FROM actual_state THEN
     RAISE EXCEPTION 'settlement match contradicts independently verified source evidence'
       USING ERRCODE='23514';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER guarded_settlement_match_insert BEFORE INSERT
ON payment_ops.settlement_match FOR EACH ROW
EXECUTE FUNCTION payment_ops.guard_settlement_match_insert();

-- This function is deliberately *NOT* an approval for a payout, a financial
-- ERP posting or certification. It deterministically appends a comparison
-- of two independently ingested verified source records in the same actor
-- scope, and emits an immutable audit outbox fact.
CREATE FUNCTION payment_ops.reconcile_verified_settlement(
  p_tenant text, p_actor text, p_execution uuid,
  p_psp uuid, p_bank uuid, p_correlation uuid
) RETURNS text LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE
  p payment_ops.settlement_evidence%ROWTYPE;
  b payment_ops.settlement_evidence%ROWTYPE;
  n payment_ops.native_execution%ROWTYPE;
  v_match text;
  v_reason text;
BEGIN
  IF p_tenant IS DISTINCT FROM nullif(current_setting('baobab.tenant_id',true),'')
     OR p_actor IS DISTINCT FROM nullif(current_setting('baobab.responsible_legal_entity_id',true),'')
     OR p_psp=p_bank THEN
      RAISE EXCEPTION 'settlement actor context denied' USING ERRCODE='42501';
  END IF;
  SELECT * INTO p FROM payment_ops.settlement_evidence
    WHERE evidence_id=p_psp AND tenant_id=p_tenant AND responsible_legal_entity_id=p_actor
      AND execution_id=p_execution AND source_kind='PSP';
  SELECT * INTO b FROM payment_ops.settlement_evidence
    WHERE evidence_id=p_bank AND tenant_id=p_tenant AND responsible_legal_entity_id=p_actor
      AND execution_id=p_execution AND source_kind='BANK';
  IF p.evidence_id IS NULL OR b.evidence_id IS NULL THEN
      RAISE EXCEPTION 'two independently verified evidence entries required' USING ERRCODE='23514';
  END IF;
  SELECT * INTO n FROM payment_ops.native_execution
   WHERE execution_id=p_execution AND tenant_id=p_tenant
     AND responsible_legal_entity_id=p_actor;
  IF n.execution_id IS NULL THEN
     RAISE EXCEPTION 'native payment intent not present' USING ERRCODE='23514';
  END IF;
  v_reason = CASE
    WHEN p.gross_minor<>n.amount_minor OR p.currency_code<>n.currency_code
      OR p.payment_reference<>n.canonical_payment_reference
        THEN 'PROVIDER_SCOPE_MISMATCH'
    WHEN p.independent_source=b.independent_source THEN 'PROVIDER_SCOPE_MISMATCH'
    WHEN p.provider_engine_instance_id<>b.provider_engine_instance_id
      OR p.provider_environment<>b.provider_environment
      OR p.batch_reference<>b.batch_reference
      OR p.payment_reference<>b.payment_reference THEN 'PROVIDER_SCOPE_MISMATCH'
    WHEN p.currency_code<>b.currency_code THEN 'CURRENCY_MISMATCH'
    WHEN p.net_minor<>b.net_minor OR b.fee_minor<>0 OR b.gross_minor<>b.net_minor
        THEN 'BANK_AMOUNT_MISMATCH'
    ELSE 'THREE_WAY_MATCH' END;
  v_match = CASE WHEN v_reason='THREE_WAY_MATCH' THEN 'MATCHED' ELSE 'EXCEPTION' END;
  INSERT INTO payment_ops.settlement_match
   (tenant_id,responsible_legal_entity_id,execution_id,psp_evidence_id,bank_evidence_id,
    state,reason_code,correlation_id)
   VALUES(p_tenant,p_actor,p_execution,p_psp,p_bank,v_match,v_reason,p_correlation);
  INSERT INTO payment_ops.transaction_outbox
    (tenant_id,responsible_legal_entity_id,execution_id,event_type,
     event_payload,correlation_id)
  VALUES(p_tenant,p_actor,p_execution,
    CASE WHEN v_match='MATCHED' THEN 'payments.settlement.matched.v1'
      ELSE 'payments.settlement.exception.v1' END,
    jsonb_build_object('psp_evidence_id',p_psp,'bank_evidence_id',p_bank,
       'state',v_match,'reason_code',v_reason),p_correlation);
  RETURN v_match;
END $$;
COMMIT;
