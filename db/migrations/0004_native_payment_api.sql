-- P0-04B Payments Gate D1: the durable records behind the public Payment API
-- when it runs on the native backend (PostgreSQL + HyperSwitch). Install
-- AFTER 0003 in PostgreSQL 17.
--
-- An intent is Payments' own fact: its amount, currency, merchant and
-- obligation are recorded once and every later money movement is computed
-- from this row, never from a client body. Payment and refund state is never
-- stored as a mutable status: it is derived from the journal's money
-- movements and their provider observations (0003).
BEGIN;

-- HTTP idempotency for every native mutation: one key, one request. A reused
-- key with a different body is refused before anything else happens. The
-- request is recorded as a hash; payment method tokens are never stored.
CREATE TABLE payment_ops.native_api_request (
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL CHECK (length(btrim(responsible_legal_entity_id)) BETWEEN 2 AND 160),
    operation text NOT NULL CHECK (operation ~ '^(create|confirm|capture|cancel|refund):[a-z0-9_]{0,80}$'),
    idempotency_key text NOT NULL CHECK (length(idempotency_key) BETWEEN 16 AND 128
        AND idempotency_key ~ '^[A-Za-z0-9._:-]+$'),
    request_sha256 text NOT NULL CHECK (request_sha256 ~ '^[0-9a-f]{64}$'),
    -- The resource this request creates or acts on.
    resource_id text NOT NULL CHECK (resource_id ~ '^(payint|pay|refund)_[a-z0-9]+$'),
    -- A refund's reason is part of its resource; nothing sensitive is kept.
    reason text CHECK (reason IS NULL OR length(reason) BETWEEN 1 AND 500),
    recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (tenant_id, responsible_legal_entity_id, operation, idempotency_key)
);

CREATE TABLE payment_ops.native_intent (
    payment_intent_id text PRIMARY KEY CHECK (payment_intent_id ~ '^payint_[a-z0-9]+$'
        AND length(payment_intent_id) BETWEEN 8 AND 63),
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL CHECK (length(btrim(responsible_legal_entity_id)) BETWEEN 2 AND 160),
    operating_organisation_id uuid NOT NULL,
    -- The provider payment this intent drives, minted by Payments.
    payment_reference text NOT NULL UNIQUE CHECK (payment_reference ~ '^pay_[a-z0-9]+$'
        AND length(payment_reference) BETWEEN 5 AND 63),
    market_code text NOT NULL CHECK (market_code ~ '^[A-Z]{2}$'),
    currency_code text NOT NULL CHECK (currency_code ~ '^[A-Z]{3}$'),
    amount_minor bigint NOT NULL CHECK (amount_minor > 0),
    -- Native execution authorises and captures as separate movements.
    capture_method text NOT NULL CHECK (capture_method = 'MANUAL'),
    merchant_profile_reference text NOT NULL CHECK (length(btrim(merchant_profile_reference)) BETWEEN 3 AND 160),
    customer_reference text NOT NULL CHECK (customer_reference ~ '^pacct_[a-z0-9]+$'),
    market_id text CHECK (market_id IS NULL OR length(market_id) <= 128),
    source_engine text NOT NULL CHECK (source_engine ~ '^[a-z][a-z0-9-]{1,62}$'),
    source_reference text NOT NULL CHECK (length(source_reference) BETWEEN 1 AND 255),
    correlation_id text NOT NULL CHECK (length(correlation_id) BETWEEN 1 AND 255),
    description text CHECK (description IS NULL OR length(description) <= 500),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE (tenant_id, responsible_legal_entity_id, payment_intent_id)
);

-- Cancelling an intent that was never authorised moves no money and needs no
-- provider call: it is recorded here, and the journal then refuses any
-- authorisation of that payment. An authorised intent is voided instead.
CREATE TABLE payment_ops.native_intent_cancellation (
    payment_intent_id text PRIMARY KEY,
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL,
    reason text NOT NULL CHECK (length(reason) BETWEEN 1 AND 500),
    cancelled_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY (tenant_id, responsible_legal_entity_id, payment_intent_id)
       REFERENCES payment_ops.native_intent (tenant_id, responsible_legal_entity_id, payment_intent_id)
);

DO $$
DECLARE t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['native_api_request', 'native_intent', 'native_intent_cancellation'] LOOP
        EXECUTE format('ALTER TABLE payment_ops.%I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE payment_ops.%I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format($p$CREATE POLICY %I ON payment_ops.%I
            USING (tenant_id = nullif(current_setting('baobab.tenant_id', true), '')
               AND responsible_legal_entity_id = nullif(current_setting('baobab.responsible_legal_entity_id', true), ''))
            WITH CHECK (tenant_id = nullif(current_setting('baobab.tenant_id', true), '')
               AND responsible_legal_entity_id = nullif(current_setting('baobab.responsible_legal_entity_id', true), ''))$p$,
            t || '_tenant', t);
        EXECUTE format('CREATE TRIGGER immutable_%s BEFORE UPDATE OR DELETE ON payment_ops.%I
            FOR EACH ROW EXECUTE FUNCTION payment_ops.refuse_historical_rewrite()', t, t);
    END LOOP;
END;
$$;

-- A cancellation and an authorisation of the same payment serialise on the
-- payment lock 0003 already takes for every movement, so exactly one wins.
CREATE FUNCTION payment_ops.guard_intent_cancellation() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE
    v_payment text;
BEGIN
    NEW.cancelled_at := clock_timestamp();
    SELECT i.payment_reference INTO v_payment FROM payment_ops.native_intent i
     WHERE i.payment_intent_id = NEW.payment_intent_id;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'payment|' || NEW.tenant_id || '|' || NEW.responsible_legal_entity_id || '|' || v_payment, 0));
    IF EXISTS (SELECT 1 FROM payment_ops.native_execution e
                WHERE e.tenant_id = NEW.tenant_id AND e.responsible_legal_entity_id = NEW.responsible_legal_entity_id
                  AND e.operation_kind = 'AUTHORIZE' AND e.canonical_payment_reference = v_payment
                  AND payment_ops.execution_outcome(e.execution_id) IS DISTINCT FROM 'REJECTED') THEN
        RAISE EXCEPTION 'an authorised payment is voided, not cancelled' USING ERRCODE = 'BPS01';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_intent_cancellation BEFORE INSERT ON payment_ops.native_intent_cancellation
    FOR EACH ROW EXECUTE FUNCTION payment_ops.guard_intent_cancellation();

-- Runs after 0003's reservation guard (trigger names order AFTER it), which
-- already holds the payment lock for an authorisation.
CREATE FUNCTION payment_ops.guard_authorization_of_cancelled_intent() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER AS $$
BEGIN
    IF NEW.operation_kind = 'AUTHORIZE' AND EXISTS (
        SELECT 1 FROM payment_ops.native_intent i
          JOIN payment_ops.native_intent_cancellation c ON c.payment_intent_id = i.payment_intent_id
         WHERE i.tenant_id = NEW.tenant_id AND i.responsible_legal_entity_id = NEW.responsible_legal_entity_id
           AND i.payment_reference = NEW.canonical_payment_reference) THEN
        RAISE EXCEPTION 'a cancelled intent can never be authorised' USING ERRCODE = 'BPS01';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_money_movement_reservation_z_cancelled BEFORE INSERT ON payment_ops.native_execution
    FOR EACH ROW EXECUTE FUNCTION payment_ops.guard_authorization_of_cancelled_intent();

COMMIT;
