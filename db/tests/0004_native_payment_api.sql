-- P0-04B Gate D1 PostgreSQL 17 regression tests for the native Payment API
-- records. Run after 0001-0004 and the 0003 tests (which create payment_ci and
-- the pt helpers). Every check runs as the restricted runtime role.
\set ON_ERROR_STOP on
GRANT SELECT, INSERT, UPDATE, DELETE ON
    payment_ops.native_api_request, payment_ops.native_intent, payment_ops.native_intent_cancellation
    TO payment_ci;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA payment_ops TO payment_ci;

CREATE FUNCTION pt.intent(p_id text, p_payment text) RETURNS void
LANGUAGE sql AS $$
    INSERT INTO payment_ops.native_intent (payment_intent_id, tenant_id, responsible_legal_entity_id,
        operating_organisation_id, payment_reference, market_code, currency_code, amount_minor, capture_method,
        merchant_profile_reference, customer_reference, source_engine, source_reference, correlation_id)
    VALUES (p_id, current_setting('baobab.tenant_id'), current_setting('baobab.responsible_legal_entity_id'),
        '0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c', p_payment, 'UG', 'UGX', 1000, 'MANUAL', 'mprof_synthetic',
        'pacct_synthetic', 'baobab-trade', 'order/1', '0199a1b2-0000-7000-8000-00000000c0c0')
$$;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA pt TO payment_ci;

SET ROLE payment_ci;
SET baobab.tenant_id = 'tn_moneytest';
SET baobab.responsible_legal_entity_id = 'LE-SYNTHETIC-PAY';

SELECT pt.intent('payint_d4one', 'pay_d4one');
SELECT pt.intent('payint_d4two', 'pay_d4two');

-- Intents are Payments' own facts: only MANUAL capture, a platform account,
-- and never rewritten.
SELECT pt.refused($$INSERT INTO payment_ops.native_intent (payment_intent_id, tenant_id, responsible_legal_entity_id,
    operating_organisation_id, payment_reference, market_code, currency_code, amount_minor, capture_method,
    merchant_profile_reference, customer_reference, source_engine, source_reference, correlation_id)
    VALUES ('payint_d4auto','tn_moneytest','LE-SYNTHETIC-PAY','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c','pay_d4auto',
    'UG','UGX',1000,'AUTOMATIC','mprof_synthetic','pacct_synthetic','baobab-trade','order/2','c')$$,
    '23514', 'automatic capture intent');
SELECT pt.refused($$INSERT INTO payment_ops.native_intent (payment_intent_id, tenant_id, responsible_legal_entity_id,
    operating_organisation_id, payment_reference, market_code, currency_code, amount_minor, capture_method,
    merchant_profile_reference, customer_reference, source_engine, source_reference, correlation_id)
    VALUES ('payint_d4card','tn_moneytest','LE-SYNTHETIC-PAY','0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6c','pay_d4card',
    'UG','UGX',1000,'MANUAL','mprof_synthetic','4242424242424242','baobab-trade','order/3','c')$$,
    '23514', 'card number as customer reference');
SELECT pt.refused($$SELECT pt.intent('payint_d4dup', 'pay_d4one')$$, '23505', 'two intents for one payment');
SELECT pt.refused($$UPDATE payment_ops.native_intent SET amount_minor = 1 WHERE payment_intent_id = 'payint_d4one'$$,
    'P0001', 'intent rewrite');
SELECT pt.refused($$DELETE FROM payment_ops.native_intent WHERE payment_intent_id = 'payint_d4one'$$,
    'P0001', 'intent delete');

-- One key, one request: a second request under the same key is a conflict,
-- and a request record is never rewritten.
INSERT INTO payment_ops.native_api_request (tenant_id, responsible_legal_entity_id, operation, idempotency_key,
    request_sha256, resource_id) VALUES ('tn_moneytest', 'LE-SYNTHETIC-PAY', 'create:', 'idem-d4-create-0001',
    repeat('a', 64), 'payint_d4one');
SELECT pt.refused($$INSERT INTO payment_ops.native_api_request (tenant_id, responsible_legal_entity_id, operation,
    idempotency_key, request_sha256, resource_id) VALUES ('tn_moneytest','LE-SYNTHETIC-PAY','create:',
    'idem-d4-create-0001',repeat('b',64),'payint_d4two')$$, '23505', 'key reuse');
SELECT pt.refused($$UPDATE payment_ops.native_api_request SET request_sha256 = repeat('b',64)$$, 'P0001',
    'request rewrite');
SELECT pt.refused($$INSERT INTO payment_ops.native_api_request (tenant_id, responsible_legal_entity_id, operation,
    idempotency_key, request_sha256, resource_id) VALUES ('tn_moneytest','LE-SYNTHETIC-PAY','payout:',
    'idem-d4-payout-0001',repeat('a',64),'pay_d4one')$$, '23514', 'payout request');

-- A cancelled intent can never be authorised, even by direct SQL with a
-- current CP decision; an authorised one can never be cancelled, only voided.
INSERT INTO payment_ops.native_intent_cancellation (payment_intent_id, tenant_id, responsible_legal_entity_id, reason)
VALUES ('payint_d4two', 'tn_moneytest', 'LE-SYNTHETIC-PAY', 'buyer abandoned checkout');
SELECT pt.refused($$SELECT pt.move('AUTHORIZE','payments.authorization.create','d4-auth-cancelled','pay_d4two',1000)$$,
    'BPS01', 'authorise a cancelled intent');
SELECT pt.move('AUTHORIZE', 'payments.authorization.create', 'd4-auth-one', 'pay_d4one', 1000);
SELECT pt.refused($$INSERT INTO payment_ops.native_intent_cancellation (payment_intent_id, tenant_id,
    responsible_legal_entity_id, reason) VALUES ('payint_d4one','tn_moneytest','LE-SYNTHETIC-PAY','too late')$$,
    'BPS01', 'cancel an authorised intent');
SELECT pt.refused($$DELETE FROM payment_ops.native_intent_cancellation$$, 'P0001', 'cancellation delete');

-- Another tenant, or another legal entity of this tenant, sees nothing and
-- can write nothing here.
SET baobab.responsible_legal_entity_id = 'LE-OTHER';
DO $$ BEGIN
    IF (SELECT count(*) FROM payment_ops.native_intent) + (SELECT count(*) FROM payment_ops.native_api_request)
       + (SELECT count(*) FROM payment_ops.native_intent_cancellation) <> 0 THEN
        RAISE EXCEPTION 'another legal entity can read native intents';
    END IF;
END $$;
SET baobab.tenant_id = 'tn_othertenant';
SET baobab.responsible_legal_entity_id = 'LE-SYNTHETIC-PAY';
SELECT pt.refused($$INSERT INTO payment_ops.native_intent_cancellation (payment_intent_id, tenant_id,
    responsible_legal_entity_id, reason) VALUES ('payint_d4one','tn_moneytest','LE-SYNTHETIC-PAY','cross tenant')$$,
    '42501', 'cross-tenant cancellation');
DO $$ BEGIN
    IF (SELECT count(*) FROM payment_ops.native_intent) <> 0 THEN
        RAISE EXCEPTION 'another tenant can read native intents';
    END IF;
END $$;
RESET ROLE;
SELECT 'native payment API journal tests passed';
