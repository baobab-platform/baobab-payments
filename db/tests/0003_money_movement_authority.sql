-- P0-04B Payments (ADR-SHARED-035) PostgreSQL 17 regression tests for the
-- money-movement fences. Synthetic actors only. Run after 0001, 0002, 0003
-- and their tests; every refusal is proven as the restricted runtime role.
\set ON_ERROR_STOP on
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'payment_ci') THEN
        CREATE ROLE payment_ci NOLOGIN;
    END IF;
END $$;
GRANT USAGE ON SCHEMA payment_ops TO payment_ci;
GRANT SELECT, INSERT, UPDATE, DELETE ON
    payment_ops.native_execution, payment_ops.provider_receipt, payment_ops.transaction_outbox,
    payment_ops.authority_decision, payment_ops.dispatch_attempt, payment_ops.provider_observation
    TO payment_ci;
-- The runtime can read provider readiness but never grant it to itself.
REVOKE ALL ON payment_ops.provider_activation, payment_ops.provider_activation_revocation FROM payment_ci;
GRANT SELECT ON payment_ops.provider_activation, payment_ops.provider_activation_revocation TO payment_ci;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA payment_ops TO payment_ci;

CREATE SCHEMA pt;
GRANT USAGE ON SCHEMA pt TO payment_ci;
-- Synthetic provider readiness, recorded by the operator (this superuser).
INSERT INTO payment_ops.provider_activation (activation_id, tenant_id, responsible_legal_entity_id,
    merchant_profile_reference, market_code, currency_code, operation_kinds, provider_environment,
    provider_certification_reference, merchant_activation_reference, valid_from, valid_until)
VALUES ('0199a1b2-0000-7000-8000-00000000a001', 'tn_moneytest', 'LE-SYNTHETIC-PAY', 'mprof_synthetic',
        'UG', 'UGX', ARRAY['AUTHORIZE','CAPTURE','CANCEL','REFUND'], 'SANDBOX',
        'synthetic/cert', 'synthetic/activation', now() - interval '1 day', now() + interval '1 day'),
       ('0199a1b2-0000-7000-8000-00000000a003', 'tn_moneytest', 'LE-SYNTHETIC-PAY', 'mprof_staging_only',
        'UG', 'UGX', ARRAY['AUTHORIZE','CAPTURE','CANCEL','REFUND'], 'STAGING',
        'synthetic/cert', 'synthetic/activation', now() - interval '1 day', now() + interval '1 day'),
       ('0199a1b2-0000-7000-8000-00000000a002', 'tn_moneytest', 'LE-SYNTHETIC-PAY', 'mprof_revoked',
        'UG', 'UGX', ARRAY['AUTHORIZE','CAPTURE','CANCEL','REFUND'], 'SANDBOX',
        'synthetic/cert', 'synthetic/activation', now() - interval '1 day', now() + interval '1 day');
INSERT INTO payment_ops.provider_activation_revocation (activation_id, tenant_id, responsible_legal_entity_id, reason)
VALUES ('0199a1b2-0000-7000-8000-00000000a002', 'tn_moneytest', 'LE-SYNTHETIC-PAY', 'synthetic withdrawal');

-- Test helpers. They run as the caller (SECURITY INVOKER), so RLS applies.
CREATE FUNCTION pt.decide(p_class text, p_ref text, p_attempt int, p_fp text,
                          p_lease interval DEFAULT interval '25 seconds',
                          p_mandate uuid DEFAULT '0199a1b2-0000-7000-8000-0000000000dd') RETURNS uuid
LANGUAGE sql AS $$
    INSERT INTO payment_ops.authority_decision (decision_id, tenant_id, responsible_legal_entity_id,
        operation_class, operation_reference, attempt, instruction_fingerprint, mandate_id, evaluated_at, expires_at)
    VALUES (gen_random_uuid(), current_setting('baobab.tenant_id'), current_setting('baobab.responsible_legal_entity_id'),
            p_class, p_ref, p_attempt, p_fp, p_mandate, clock_timestamp(), clock_timestamp() + p_lease)
    RETURNING decision_id
$$;
CREATE FUNCTION pt.reserve(p_kind text, p_class text, p_key text, p_payment text, p_amount bigint,
                           p_fp text, p_parent text DEFAULT NULL, p_profile text DEFAULT 'mprof_synthetic',
                           p_destination text DEFAULT NULL, p_currency text DEFAULT 'UGX',
                           p_mandate uuid DEFAULT '0199a1b2-0000-7000-8000-0000000000dd',
                           p_environment text DEFAULT 'SANDBOX') RETURNS uuid
LANGUAGE sql AS $$
    SELECT payment_ops.reserve_money_movement(current_setting('baobab.tenant_id'), p_kind, 'idem-' || p_key, repeat('c', 64),
        p_payment, '0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c', current_setting('baobab.responsible_legal_entity_id'),
        p_mandate, 'cp/decision/synthetic', 'synthetic/cert', 'synthetic/activation', 'UG', p_currency,
        p_amount, p_class, 'op/' || p_key, p_fp, p_parent, p_profile, p_destination, p_environment)
$$;
-- Reserve, dispatch on a fresh attempt-2 decision, and record an outcome.
CREATE FUNCTION pt.move(p_kind text, p_class text, p_key text, p_payment text, p_amount bigint,
                        p_parent text DEFAULT NULL, p_destination text DEFAULT NULL,
                        p_outcome text DEFAULT 'CONFIRMED') RETURNS uuid
LANGUAGE plpgsql AS $$
DECLARE v_execution uuid; v_dispatch uuid; v_fp text := md5(p_key) || md5(p_key);
BEGIN
    PERFORM pt.decide(p_class, 'op/' || p_key, 1, v_fp);
    v_execution := pt.reserve(p_kind, p_class, p_key, p_payment, p_amount, v_fp, p_parent, 'mprof_synthetic', p_destination);
    INSERT INTO payment_ops.dispatch_attempt (tenant_id, responsible_legal_entity_id, execution_id, decision_id, activation_id)
    VALUES (current_setting('baobab.tenant_id'), current_setting('baobab.responsible_legal_entity_id'), v_execution,
            pt.decide(p_class, 'op/' || p_key, 2, v_fp), '0199a1b2-0000-7000-8000-00000000a001')
    RETURNING dispatch_id INTO v_dispatch;
    INSERT INTO payment_ops.provider_observation (tenant_id, responsible_legal_entity_id, execution_id, dispatch_id,
        outcome, source, provider_reference, provider_status, provider_environment)
    VALUES (current_setting('baobab.tenant_id'), current_setting('baobab.responsible_legal_entity_id'), v_execution,
            v_dispatch, p_outcome, 'DISPATCH_RESPONSE',
            CASE WHEN p_outcome = 'CONFIRMED' THEN p_payment END,
            CASE WHEN p_outcome = 'CONFIRMED' THEN 'succeeded' END, 'SANDBOX');
    RETURN v_execution;
END;
$$;
-- Assert that a statement is refused with a given SQLSTATE.
CREATE FUNCTION pt.refused(p_sql text, p_state text, p_label text) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    BEGIN
        EXECUTE p_sql;
    EXCEPTION WHEN OTHERS THEN
        IF SQLSTATE <> p_state THEN
            RAISE EXCEPTION '%: expected SQLSTATE %, got % (%)', p_label, p_state, SQLSTATE, SQLERRM;
        END IF;
        RETURN;
    END;
    RAISE EXCEPTION '%: was not refused', p_label;
END;
$$;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA pt TO payment_ci;

SET ROLE payment_ci;
SET baobab.tenant_id = 'tn_moneytest';
SET baobab.responsible_legal_entity_id = 'LE-SYNTHETIC-PAY';

-- The runtime can never activate a provider for itself.
SELECT pt.refused($$INSERT INTO payment_ops.provider_activation (tenant_id, responsible_legal_entity_id,
    merchant_profile_reference, market_code, currency_code, operation_kinds, provider_environment,
    provider_certification_reference, merchant_activation_reference, valid_from, valid_until)
    VALUES ('tn_moneytest','LE-SYNTHETIC-PAY','mprof_self','UG','UGX',ARRAY['CAPTURE'],'SANDBOX',
            'self/cert','self/activation',now(),now()+interval '1 day')$$, '42501', 'self-activation');

-- The pre-existing reservation function cannot reserve a money movement.
SELECT pt.refused($$SELECT payment_ops.reserve_native_execution('tn_moneytest','CAPTURE','legacy-capture-0001',
    repeat('a',64),'pay_legacy','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c','LE-SYNTHETIC-PAY',
    '0199a1b2-0000-7000-8000-0000000000dd','synthetic','synthetic/cert','synthetic/activation','UG','UGX',100)$$,
    'BPA01', 'legacy capture without decision');
-- Payouts are refused by every route, decision or not.
SELECT pt.refused($$SELECT pt.reserve('PAYOUT','payments.payout.execute','payout-key-00001','payout_1',100,repeat('e',64),
    NULL,'mprof_synthetic','bnf_synthetic')$$, 'BPP01', 'payout reservation');
SELECT pt.refused($$SELECT payment_ops.reserve_native_execution('tn_moneytest','PAYOUT','legacy-payout-0001',
    repeat('a',64),'payout_legacy','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c','LE-SYNTHETIC-PAY',
    '0199a1b2-0000-7000-8000-0000000000dd','synthetic','synthetic/cert','synthetic/activation','UG','UGX',100)$$,
    'BPP01', 'legacy payout');
SELECT pt.refused($$SELECT pt.decide('payments.payout.execute','op/payout',1,repeat('e',64))$$, '23514', 'payout decision');

-- No decision, an expired decision, another instruction, another transaction.
SELECT pt.refused($$SELECT pt.reserve('AUTHORIZE','payments.authorization.create','auth-nodecision-1','pay_nodecision',1000,repeat('a',64))$$,
    'BPA01', 'authorisation without decision');
SELECT pt.refused($$SELECT pt.decide('payments.authorization.create','op/x',1,repeat('a',64),interval '31 seconds')$$,
    '23514', 'lease over 30 s');
-- An already-expired decision is never recorded, and Payments' own receipt
-- time and transaction overwrite anything a caller supplies.
SELECT pt.refused($$INSERT INTO payment_ops.authority_decision (decision_id, tenant_id, responsible_legal_entity_id,
    operation_class, operation_reference, attempt, instruction_fingerprint, mandate_id, evaluated_at, expires_at)
    VALUES (gen_random_uuid(),'tn_moneytest','LE-SYNTHETIC-PAY','payments.capture.execute','op/stale',1,repeat('a',64),
    '0199a1b2-0000-7000-8000-0000000000dd',clock_timestamp()-interval '20 seconds',clock_timestamp()-interval '1 second')$$,
    'BPA01', 'expired decision');
DO $$
DECLARE d payment_ops.authority_decision%ROWTYPE;
BEGIN
    INSERT INTO payment_ops.authority_decision (decision_id, tenant_id, responsible_legal_entity_id, operation_class,
        operation_reference, attempt, instruction_fingerprint, mandate_id, evaluated_at, expires_at, received_at, recording_txid)
    VALUES (gen_random_uuid(), 'tn_moneytest', 'LE-SYNTHETIC-PAY', 'payments.capture.execute', 'op/forged-receipt', 1,
            repeat('a', 64), '0199a1b2-0000-7000-8000-0000000000dd', clock_timestamp(), clock_timestamp() + interval '20 seconds',
            clock_timestamp() + interval '1 hour', 42)
    RETURNING * INTO d;
    IF d.received_at > clock_timestamp() OR d.recording_txid <> txid_current() THEN
        RAISE EXCEPTION 'a caller-supplied receipt time or transaction was kept';
    END IF;
END $$;
DO $$ BEGIN
    PERFORM pt.decide('payments.authorization.create', 'op/auth-wrongfp-01', 1, repeat('a', 64));
    PERFORM pt.refused($q$SELECT pt.reserve('AUTHORIZE','payments.authorization.create','auth-wrongfp-01','pay_wrongfp',1000,repeat('b',64))$q$,
        'BPA01', 'fingerprint drift');
    PERFORM pt.refused($q$SELECT pt.reserve('AUTHORIZE','payments.authorization.create','auth-wrongfp-01','pay_wrongfp',1000,repeat('a',64),
        NULL,'mprof_synthetic',NULL,'UGX','0199a1b2-0000-7000-8000-0000000000ee')$q$, 'BPA01', 'another mandate');
    PERFORM pt.refused($q$SELECT pt.reserve('CAPTURE','payments.authorization.create','auth-wrongfp-01','pay_wrongfp',1000,repeat('a',64))$q$,
        'BPA01', 'kind and class disagree');
END $$;
SELECT pt.decide('payments.authorization.create', 'op/auth-othertx-01', 1, repeat('a', 64));
SELECT pt.refused($$SELECT pt.reserve('AUTHORIZE','payments.authorization.create','auth-othertx-01','pay_othertx',1000,repeat('a',64))$$,
    'BPA01', 'decision from another transaction');

-- Readiness is independent of CP: no activation for the currency, or revoked.
DO $$ BEGIN
    PERFORM pt.decide('payments.authorization.create', 'op/auth-nocurrency', 1, repeat('a', 64));
    PERFORM pt.refused($q$SELECT pt.reserve('AUTHORIZE','payments.authorization.create','auth-nocurrency','pay_kes',1000,repeat('a',64),
        NULL,'mprof_synthetic',NULL,'KES')$q$, 'BPR01', 'no activation for currency');
    -- An activation for another provider environment never counts.
    PERFORM pt.decide('payments.authorization.create', 'op/auth-wrongenv-1', 1, repeat('a', 64));
    PERFORM pt.refused($q$SELECT pt.reserve('AUTHORIZE','payments.authorization.create','auth-wrongenv-1','pay_wrongenv',1000,repeat('a',64),
        NULL,'mprof_staging_only')$q$, 'BPR01', 'activation for another environment');
    PERFORM pt.decide('payments.authorization.create', 'op/auth-noenv-001', 1, repeat('a', 64));
    PERFORM pt.refused($q$SELECT pt.reserve('AUTHORIZE','payments.authorization.create','auth-noenv-001','pay_noenv',1000,repeat('a',64),
        NULL,'mprof_staging_only',NULL,'UGX','0199a1b2-0000-7000-8000-0000000000dd',NULL)$q$, 'BPR01', 'no environment');
    PERFORM pt.decide('payments.authorization.create', 'op/auth-revoked-01', 1, repeat('a', 64));
    PERFORM pt.refused($q$SELECT pt.reserve('AUTHORIZE','payments.authorization.create','auth-revoked-01','pay_revoked',1000,repeat('a',64),
        NULL,'mprof_revoked')$q$, 'BPR01', 'revoked activation');
END $$;

-- A full, legitimate lifecycle: authorise, partial captures, partial refunds.
DO $$
DECLARE v_auth uuid;
BEGIN
    v_auth := pt.move('AUTHORIZE', 'payments.authorization.create', 'auth-life-00001', 'pay_life', 1000);
    PERFORM pt.move('CAPTURE', 'payments.capture.execute', 'capture-life-0001', 'pay_life', 600, 'pay_life');
    PERFORM pt.refused($q$SELECT pt.move('CAPTURE','payments.capture.execute','capture-life-0002','pay_life',401,'pay_life')$q$,
        'BPF01', 'capture beyond authorisation');
    PERFORM pt.move('CAPTURE', 'payments.capture.execute', 'capture-life-0003', 'pay_life', 400, 'pay_life');
    PERFORM pt.refused($q$SELECT pt.move('REFUND','payments.refund.execute','refund-life-0001','refund_life_1',100,'pay_life','bnf_elsewhere')$q$,
        'BPF01', 'refund to another destination');
    PERFORM pt.move('REFUND', 'payments.refund.execute', 'refund-life-0002', 'refund_life_2', 700, 'pay_life', 'ORIGINAL_PAYMENT_METHOD');
    PERFORM pt.refused($q$SELECT pt.move('REFUND','payments.refund.execute','refund-life-0003','refund_life_3',301,'pay_life','ORIGINAL_PAYMENT_METHOD')$q$,
        'BPF01', 'refund beyond captured balance');
    PERFORM pt.move('REFUND', 'payments.refund.execute', 'refund-life-0004', 'refund_life_4', 300, 'pay_life', 'ORIGINAL_PAYMENT_METHOD');
    PERFORM pt.refused($q$SELECT pt.move('CANCEL','payments.authorization.void','void-life-000001','pay_life',1000,'pay_life')$q$,
        'BPS01', 'void after capture');
    PERFORM pt.refused($q$SELECT pt.move('AUTHORIZE','payments.authorization.create','auth-life-00002','pay_life',1000)$q$,
        'BPS01', 'second authorisation');
END $$;

-- Pending movements count; a rejected capture frees its amount.
DO $$ BEGIN
    PERFORM pt.move('AUTHORIZE', 'payments.authorization.create', 'auth-pend-00001', 'pay_pend', 500);
    PERFORM pt.move('CAPTURE', 'payments.capture.execute', 'capture-pend-0001', 'pay_pend', 500, 'pay_pend', NULL, 'UNKNOWN');
    PERFORM pt.refused($q$SELECT pt.move('CAPTURE','payments.capture.execute','capture-pend-0002','pay_pend',1,'pay_pend')$q$,
        'BPF01', 'unknown capture still reserves the balance');
    PERFORM pt.refused($q$SELECT pt.move('REFUND','payments.refund.execute','refund-pend-0001','refund_pend',1,'pay_pend','ORIGINAL_PAYMENT_METHOD')$q$,
        'BPF01', 'refund against an unconfirmed capture');
    PERFORM pt.move('AUTHORIZE', 'payments.authorization.create', 'auth-rej-000001', 'pay_rej', 500);
    PERFORM pt.move('CAPTURE', 'payments.capture.execute', 'capture-rej-00001', 'pay_rej', 500, 'pay_rej', NULL, 'REJECTED');
    PERFORM pt.move('CAPTURE', 'payments.capture.execute', 'capture-rej-00002', 'pay_rej', 500, 'pay_rej');
END $$;

-- Movements need a confirmed authorisation, in the same merchant and currency.
SELECT pt.refused($$SELECT pt.move('CAPTURE','payments.capture.execute','capture-orphan-01','pay_none',1,'pay_none')$$,
    'BPS01', 'capture without authorisation');
DO $$ BEGIN
    PERFORM pt.move('AUTHORIZE', 'payments.authorization.create', 'auth-unk-000001', 'pay_unk', 500, NULL, NULL, 'UNKNOWN');
    PERFORM pt.refused($q$SELECT pt.move('CAPTURE','payments.capture.execute','capture-unk-00001','pay_unk',1,'pay_unk')$q$,
        'BPS01', 'capture of an unconfirmed authorisation');
END $$;

-- Void: only in full, only uncaptured, and nothing moves after it.
DO $$ BEGIN
    PERFORM pt.move('AUTHORIZE', 'payments.authorization.create', 'auth-void-00001', 'pay_void', 800);
    PERFORM pt.refused($q$SELECT pt.move('CANCEL','payments.authorization.void','void-part-000001','pay_void',100,'pay_void')$q$,
        'BPS01', 'partial void');
    PERFORM pt.move('CANCEL', 'payments.authorization.void', 'void-full-000001', 'pay_void', 800, 'pay_void');
    PERFORM pt.refused($q$SELECT pt.move('CAPTURE','payments.capture.execute','capture-void-0001','pay_void',800,'pay_void')$q$,
        'BPS01', 'capture after void');
END $$;

-- Dispatch: a second decision is mandatory, one dispatch per movement, and a
-- re-send only after the provider proved it never received the first.
DO $$
DECLARE v_execution uuid; v_dispatch uuid; v_fp text := repeat('d', 64);
BEGIN
    PERFORM pt.decide('payments.authorization.create', 'op/auth-dispatch-01', 1, v_fp);
    v_execution := pt.reserve('AUTHORIZE', 'payments.authorization.create', 'auth-dispatch-01', 'pay_dispatch', 900, v_fp);
    PERFORM pt.refused(format($q$INSERT INTO payment_ops.dispatch_attempt (tenant_id, responsible_legal_entity_id,
        execution_id, decision_id, activation_id) SELECT 'tn_moneytest','LE-SYNTHETIC-PAY', %L, decision_id,
        '0199a1b2-0000-7000-8000-00000000a001' FROM payment_ops.authority_decision
        WHERE operation_reference='op/auth-dispatch-01' AND attempt=1$q$, v_execution), 'BPA01', 'dispatch on the reservation decision');
    PERFORM pt.refused(format($q$INSERT INTO payment_ops.dispatch_attempt (tenant_id, responsible_legal_entity_id,
        execution_id, decision_id, activation_id) VALUES ('tn_moneytest','LE-SYNTHETIC-PAY', %L,
        pt.decide('payments.authorization.create','op/auth-dispatch-01',2,%L), '0199a1b2-0000-7000-8000-00000000a002')$q$,
        v_execution, v_fp), 'BPR01', 'dispatch on a revoked activation');
    PERFORM pt.refused(format($q$INSERT INTO payment_ops.dispatch_attempt (tenant_id, responsible_legal_entity_id,
        execution_id, decision_id, activation_id) VALUES ('tn_moneytest','LE-SYNTHETIC-PAY', %L,
        pt.decide('payments.authorization.create','op/auth-dispatch-01',3,repeat('f',64)), '0199a1b2-0000-7000-8000-00000000a001')$q$,
        v_execution), 'BPA01', 'dispatch on another instruction');
    INSERT INTO payment_ops.dispatch_attempt (tenant_id, responsible_legal_entity_id, execution_id, decision_id, activation_id)
    VALUES ('tn_moneytest', 'LE-SYNTHETIC-PAY', v_execution,
            pt.decide('payments.authorization.create', 'op/auth-dispatch-01', 4, v_fp), '0199a1b2-0000-7000-8000-00000000a001')
    RETURNING dispatch_id INTO v_dispatch;
    PERFORM pt.refused(format($q$INSERT INTO payment_ops.dispatch_attempt (tenant_id, responsible_legal_entity_id,
        execution_id, decision_id, activation_id) VALUES ('tn_moneytest','LE-SYNTHETIC-PAY', %L,
        pt.decide('payments.authorization.create','op/auth-dispatch-01',5,%L), '0199a1b2-0000-7000-8000-00000000a001')$q$,
        v_execution, v_fp), 'BPS02', 'second dispatch of an unknown outcome');
    PERFORM pt.refused(format($q$INSERT INTO payment_ops.provider_observation (tenant_id, responsible_legal_entity_id,
        execution_id, dispatch_id, outcome, source, provider_environment)
        VALUES ('tn_moneytest','LE-SYNTHETIC-PAY',%L,%L,'ABSENT','RECONCILIATION','PRODUCTION')$q$, v_execution, v_dispatch),
        'BPS01', 'observation for another environment');
    INSERT INTO payment_ops.provider_observation (tenant_id, responsible_legal_entity_id, execution_id, dispatch_id,
        outcome, source, provider_environment)
    VALUES ('tn_moneytest', 'LE-SYNTHETIC-PAY', v_execution, v_dispatch, 'ABSENT', 'RECONCILIATION', 'SANDBOX');
    INSERT INTO payment_ops.dispatch_attempt (tenant_id, responsible_legal_entity_id, execution_id, decision_id, activation_id)
    VALUES ('tn_moneytest', 'LE-SYNTHETIC-PAY', v_execution,
            pt.decide('payments.authorization.create', 'op/auth-dispatch-01', 6, v_fp), '0199a1b2-0000-7000-8000-00000000a001');
END $$;

-- Replay: the identical instruction returns the reservation; a changed one conflicts.
DO $$
DECLARE v_first uuid; v_again uuid;
BEGIN
    PERFORM pt.decide('payments.authorization.create', 'op/auth-replay-001', 1, repeat('9', 64));
    v_first := pt.reserve('AUTHORIZE', 'payments.authorization.create', 'auth-replay-001', 'pay_replay', 700, repeat('9', 64));
    v_again := pt.reserve('AUTHORIZE', 'payments.authorization.create', 'auth-replay-001', 'pay_replay', 700, repeat('9', 64));
    IF v_first IS DISTINCT FROM v_again THEN RAISE EXCEPTION 'identical replay created another reservation'; END IF;
    PERFORM pt.refused($q$SELECT pt.reserve('AUTHORIZE','payments.authorization.create','auth-replay-001','pay_replay',701,repeat('8',64))$q$,
        '23505', 'changed instruction under the same key');
END $$;

-- History is immutable and tenant-isolated.
SELECT pt.refused($$UPDATE payment_ops.authority_decision SET mandate_id = '0199a1b2-0000-7000-8000-0000000000ee'$$, 'P0001', 'decision rewrite');
SELECT pt.refused($$DELETE FROM payment_ops.dispatch_attempt$$, 'P0001', 'dispatch delete');
SELECT pt.refused($$UPDATE payment_ops.provider_observation SET outcome = 'CONFIRMED'$$, 'P0001', 'observation rewrite');
SET baobab.tenant_id = 'tn_othertenant';
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM payment_ops.authority_decision) OR EXISTS (SELECT 1 FROM payment_ops.provider_activation)
       OR EXISTS (SELECT 1 FROM payment_ops.dispatch_attempt) THEN
        RAISE EXCEPTION 'another tenant can read money-movement history';
    END IF;
    PERFORM pt.refused($q$INSERT INTO payment_ops.authority_decision (decision_id, tenant_id, responsible_legal_entity_id,
        operation_class, operation_reference, attempt, instruction_fingerprint, mandate_id, evaluated_at, expires_at)
        VALUES (gen_random_uuid(),'tn_moneytest','LE-SYNTHETIC-PAY','payments.capture.execute','op/cross',1,repeat('a',64),
        '0199a1b2-0000-7000-8000-0000000000dd',clock_timestamp(),clock_timestamp()+interval '20 seconds')$q$,
        '42501', 'cross-tenant decision');
END $$;
SET baobab.tenant_id = 'tn_moneytest';

-- COMMIT fence: a decision that expires before COMMIT rolls the reservation back.
\set ON_ERROR_STOP off
BEGIN;
SELECT pt.decide('payments.authorization.create', 'op/auth-commit-001', 1, repeat('7', 64), interval '1 second');
SELECT pt.reserve('AUTHORIZE', 'payments.authorization.create', 'auth-commit-001', 'pay_commit', 300, repeat('7', 64));
SELECT pg_sleep(1.3);
COMMIT;
\set ON_ERROR_STOP on
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM payment_ops.native_execution WHERE idempotency_key = 'idem-auth-commit-001') THEN
        RAISE EXCEPTION 'a reservation committed after its CP decision expired';
    END IF;
END $$;
RESET ROLE;
SELECT 'money-movement fences passed' AS result;
