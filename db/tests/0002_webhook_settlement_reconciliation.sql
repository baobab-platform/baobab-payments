-- LA-05E7 PostgreSQL 17: synthetic ONLY, no bank/PSP certification.
\set ON_ERROR_STOP on
GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA payment_ops TO payment_ci;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA payment_ops TO payment_ci;
SET ROLE payment_ci;
SET baobab.tenant_id='tn_syntheticrecon';
SET baobab.responsible_legal_entity_id='LE-SYNTHETIC-ZA';

DO $$
DECLARE
  exec_id uuid;
  psp uuid;
  bank uuid;
  bank_wrong uuid;
  outcome text;
  refused boolean := false;
BEGIN
  exec_id := payment_ops.reserve_native_execution(
    'tn_syntheticrecon','CREATE_INTENT','synthetic-recon-key-0001',repeat('a',64),
    'payment-synthetic-recon','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c',
    'LE-SYNTHETIC-ZA','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d',
    'synthetic/legal','synthetic/cert','synthetic/merchant','ZA','ZAR',1200);

  INSERT INTO payment_ops.settlement_evidence
    (tenant_id,responsible_legal_entity_id,execution_id,source_kind,independent_source,
     provider_engine_instance_id,provider_environment,batch_reference,external_event_id,
     payment_reference,currency_code,gross_minor,fee_minor,net_minor,
     verified_source_proof_sha256,verification_method)
  VALUES('tn_syntheticrecon','LE-SYNTHETIC-ZA',exec_id,'PSP','hyperswitch.synthetic',
    'synthetic-engine','SANDBOX','synthetic-batch','psp-event-synthetic-01',
    'payment-synthetic-recon','ZAR',1200,100,1100,repeat('b',64),'synthetic-test-only')
  RETURNING evidence_id INTO psp;

  INSERT INTO payment_ops.settlement_evidence
    (tenant_id,responsible_legal_entity_id,execution_id,source_kind,independent_source,
     provider_engine_instance_id,provider_environment,batch_reference,external_event_id,
     payment_reference,currency_code,gross_minor,fee_minor,net_minor,
     verified_source_proof_sha256,verification_method)
  VALUES('tn_syntheticrecon','LE-SYNTHETIC-ZA',exec_id,'BANK','bank.synthetic',
    'synthetic-engine','SANDBOX','synthetic-batch','bank-event-synthetic-bad',
    'payment-synthetic-recon','ZAR',1000,0,1000,repeat('c',64),'synthetic-test-only')
  RETURNING evidence_id INTO bank_wrong;
  outcome := payment_ops.reconcile_verified_settlement(
    'tn_syntheticrecon','LE-SYNTHETIC-ZA',exec_id,psp,bank_wrong,
    '0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5111');
  IF outcome <> 'EXCEPTION' THEN RAISE EXCEPTION 'wrong bank deposit accepted'; END IF;

  INSERT INTO payment_ops.settlement_evidence
    (tenant_id,responsible_legal_entity_id,execution_id,source_kind,independent_source,
     provider_engine_instance_id,provider_environment,batch_reference,external_event_id,
     payment_reference,currency_code,gross_minor,fee_minor,net_minor,
     verified_source_proof_sha256,verification_method)
  VALUES('tn_syntheticrecon','LE-SYNTHETIC-ZA',exec_id,'BANK','bank.synthetic',
    'synthetic-engine','SANDBOX','synthetic-batch','bank-event-synthetic-correct',
    'payment-synthetic-recon','ZAR',1100,0,1100,repeat('d',64),'synthetic-test-only')
  RETURNING evidence_id INTO bank;
  outcome := payment_ops.reconcile_verified_settlement(
    'tn_syntheticrecon','LE-SYNTHETIC-ZA',exec_id,psp,bank,
    '0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5222');
  IF outcome <> 'MATCHED' THEN RAISE EXCEPTION 'correct independent three-way match refused'; END IF;
  IF (SELECT COUNT(*) FROM payment_ops.settlement_match) <> 2 THEN
      RAISE EXCEPTION 'historical settlement exception lost';
  END IF;
  IF (SELECT COUNT(*) FROM payment_ops.transaction_outbox
      WHERE event_type IN ('payments.settlement.matched.v1','payments.settlement.exception.v1')) <> 2
  THEN RAISE EXCEPTION 'two immutable reconciliation outbox events required'; END IF;

  BEGIN
    PERFORM payment_ops.reconcile_verified_settlement(
     'tn_syntheticrecon','LE-SYNTHETIC-ZA',exec_id,psp,bank_wrong,
     '0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5333');
  EXCEPTION WHEN unique_violation THEN refused := true;
  END;
  IF NOT refused THEN RAISE EXCEPTION 'duplicate evidence pair rewrote history'; END IF;
  refused := false;
  BEGIN
    UPDATE payment_ops.settlement_match SET state='MATCHED'
     WHERE psp_evidence_id=psp AND bank_evidence_id=bank_wrong;
  EXCEPTION WHEN raise_exception OR insufficient_privilege THEN refused := true;
  END;
  IF NOT refused THEN RAISE EXCEPTION 'settlement exception was mutable'; END IF;

  PERFORM set_config('baobab.tenant_id','tn_another',true);
  IF EXISTS (SELECT 1 FROM payment_ops.settlement_match) THEN
    RAISE EXCEPTION 'RLS leaked foreign settlement matches';
  END IF;
END $$;
-- Preserve an independently verified but unlinked bank deposit without
-- inventing an iDempiere order or native payment execution.
DO $$
DECLARE
  unmatched_id uuid;
  denied boolean := false;
BEGIN
  INSERT INTO payment_ops.unmatched_settlement_evidence
    (tenant_id,responsible_legal_entity_id,source_kind,independent_source,
     provider_engine_instance_id,provider_environment,batch_reference,
     external_event_id,payment_reference,currency_code,gross_minor,
     fee_minor,net_minor,verified_source_proof_sha256,verification_method)
  VALUES('tn_syntheticrecon','LE-SYNTHETIC-ZA','BANK','synthetic.bank',
    'synthetic-engine','SANDBOX','unmatched-batch',
    'unmatched-bank-event','unknown-bank-deposit','ZAR',999,0,999,
    repeat('f',64),'synthetic-independent-bank-proof')
  RETURNING evidence_id INTO unmatched_id;
  IF unmatched_id IS NULL THEN RAISE EXCEPTION 'unmatched source evidence was lost'; END IF;
  BEGIN
    UPDATE payment_ops.unmatched_settlement_evidence
       SET payment_reference='fraudulent-relink'
     WHERE evidence_id=unmatched_id;
  EXCEPTION WHEN insufficient_privilege OR raise_exception THEN denied := true;
  END;
  IF NOT denied THEN RAISE EXCEPTION 'unmatched historical source was mutable'; END IF;
  PERFORM set_config('baobab.tenant_id','tn_another',true);
  IF EXISTS (SELECT 1 FROM payment_ops.unmatched_settlement_evidence
              WHERE evidence_id=unmatched_id) THEN
    RAISE EXCEPTION 'unmatched source leaked across tenant';
  END IF;
END $$;
RESET ROLE;
SELECT 'LA-05E7 PostgreSQL matching/RLS/immutable exception tests passed' AS result;
