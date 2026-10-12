-- LA-05E4 executable PostgreSQL 17 regression tests; synthetic actors only.
\set ON_ERROR_STOP on
CREATE ROLE payment_ci NOLOGIN;
GRANT USAGE ON SCHEMA payment_ops TO payment_ci;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA payment_ops TO payment_ci;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA payment_ops TO payment_ci;
SET ROLE payment_ci;
-- A caller without a trusted tenant context cannot reserve payment operations.
DO $$
DECLARE
    forbidden boolean := false;
BEGIN
    BEGIN
        PERFORM payment_ops.reserve_native_execution(
            'tn_synthetic_a','CAPTURE','synthetic-idemp-0001',repeat('a',64),
            'pay_synthetic_001','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c',
            'LE-SYNTHETIC-A','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d',
            'synthetic/evidence','synthetic/cert','synthetic/activation','ZA','ZAR',1200);
    EXCEPTION WHEN insufficient_privilege THEN
        forbidden := true;
    END;
    IF NOT forbidden THEN RAISE EXCEPTION 'RLS allowed no-tenant payment reservation'; END IF;
END;
$$;

SET baobab.tenant_id = 'tn_synthetic_a';
DO $$
DECLARE
    first_id uuid;
    replay_id uuid;
    denied boolean;
BEGIN
    first_id := payment_ops.reserve_native_execution(
        'tn_synthetic_a','CAPTURE','synthetic-idemp-0001',repeat('a',64),
        'pay_synthetic_001','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c',
        'LE-SYNTHETIC-A','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d',
        'synthetic/evidence','synthetic/cert','synthetic/activation','ZA','ZAR',1200);
    replay_id := payment_ops.reserve_native_execution(
        'tn_synthetic_a','CAPTURE','synthetic-idemp-0001',repeat('a',64),
        'pay_synthetic_001','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c',
        'LE-SYNTHETIC-A','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d',
        'synthetic/evidence','synthetic/cert','synthetic/activation','ZA','ZAR',1200);
    IF first_id IS DISTINCT FROM replay_id OR first_id IS NULL THEN
        RAISE EXCEPTION 'identical idempotent replay created another operation';
    END IF;
    denied := false;
    BEGIN
        PERFORM payment_ops.reserve_native_execution(
            'tn_synthetic_a','CAPTURE','synthetic-idemp-0001',repeat('b',64),
            'pay_synthetic_001','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c',
            'LE-SYNTHETIC-A','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d',
            'synthetic/evidence','synthetic/cert','synthetic/activation','ZA','ZAR',1200);
    EXCEPTION WHEN unique_violation THEN denied := true;
    END;
    IF NOT denied THEN RAISE EXCEPTION 'different request hash replay allowed'; END IF;
    denied := false;
    BEGIN
        PERFORM payment_ops.reserve_native_execution(
            'tn_synthetic_a','CAPTURE','synthetic-idemp-0001',repeat('a',64),
            'pay_synthetic_001','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c',
            'LE-SYNTHETIC-OTHER','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d',
            'synthetic/evidence','synthetic/cert','synthetic/activation','ZA','ZAR',1200);
    EXCEPTION WHEN unique_violation THEN denied := true;
    END;
    IF NOT denied THEN RAISE EXCEPTION 'actor-swapped replay allowed'; END IF;

    INSERT INTO payment_ops.provider_receipt
        (tenant_id, execution_id, provider_name, provider_event_id, observed_status, verified_payload_sha256)
    VALUES ('tn_synthetic_a', first_id, 'hyperswitch-test', 'synthetic-psp-event-001', 'PENDING', repeat('c',64));
    INSERT INTO payment_ops.transaction_outbox
        (tenant_id, execution_id, event_type, event_payload, correlation_id)
    VALUES ('tn_synthetic_a', first_id, 'payments.native-operation.reserved.v1',
            jsonb_build_object('reference','pay_synthetic_001','simulated',true),
            '0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5000');

    denied := false;
    BEGIN
        UPDATE payment_ops.native_execution SET responsible_legal_entity_id='LE-TAMPER'
         WHERE execution_id = first_id;
    EXCEPTION WHEN raise_exception THEN denied := true;
    END;
    IF NOT denied THEN RAISE EXCEPTION 'historical actor could be rewritten'; END IF;
    denied := false;
    BEGIN
        DELETE FROM payment_ops.provider_receipt WHERE execution_id = first_id;
    EXCEPTION WHEN raise_exception THEN denied := true;
    END;
    IF NOT denied THEN RAISE EXCEPTION 'verified provider receipt could be deleted'; END IF;
END;
$$;

-- RLS must hide another tenant's history and refuse cross-tenant writes.
SET baobab.tenant_id = 'tn_synthetic_b';
DO $$
DECLARE denied boolean := false;
BEGIN
    IF EXISTS (SELECT 1 FROM payment_ops.native_execution) THEN
        RAISE EXCEPTION 'foreign tenant execution visible';
    END IF;
    IF EXISTS (SELECT 1 FROM payment_ops.provider_receipt) THEN
        RAISE EXCEPTION 'foreign tenant provider receipt visible';
    END IF;
    IF EXISTS (SELECT 1 FROM payment_ops.transaction_outbox) THEN
        RAISE EXCEPTION 'foreign tenant outbox visible';
    END IF;
    BEGIN
        INSERT INTO payment_ops.native_execution (
          tenant_id,operation_kind,idempotency_key,request_sha256,
          canonical_payment_reference,operating_organisation_id,responsible_legal_entity_id,
          cp_mandate_id,legal_actor_evidence_reference,provider_certification_reference,
          merchant_activation_reference,market_code,currency_code,amount_minor
        ) VALUES (
          'tn_synthetic_a','CAPTURE','synthetic-idemp-0002',repeat('d',64),
          'pay_synthetic_002','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c','LE-SYNTHETIC-A',
          '0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6d',
          'synthetic/evidence','synthetic/cert','synthetic/activation','ZA','ZAR',1200
        );
    EXCEPTION WHEN check_violation THEN denied := true;
    END;
    IF NOT denied THEN RAISE EXCEPTION 'cross-tenant direct insertion succeeded'; END IF;
END;
$$;
RESET ROLE;
SELECT 'LA-05E4 RLS/immutability/idempotency tests passed' AS result;
