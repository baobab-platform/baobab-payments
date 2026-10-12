-- LA-05E4: durable, tenant-isolated native payment operation journal.
-- PostgreSQL 17. This migration creates EMPTY tables and does not enable PSPs.
-- Runtime service needs an unprivileged role (no BYPASSRLS), and MUST set
-- baobab.tenant_id from an independently authenticated IAM+CP runtime context.
BEGIN;
CREATE SCHEMA IF NOT EXISTS payment_ops;

CREATE TABLE payment_ops.native_execution (
    execution_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id text NOT NULL CHECK (tenant_id ~ '^tn_[a-z0-9]{3,60}$'),
    operation_kind text NOT NULL CHECK (operation_kind IN
        ('CREATE_INTENT','AUTHORIZE','CAPTURE','CANCEL','REFUND','PAYOUT','SETTLEMENT')),
    idempotency_key text NOT NULL CHECK (idempotency_key ~ '^[A-Za-z0-9._:-]{16,128}$'),
    request_sha256 text NOT NULL CHECK (request_sha256 ~ '^[a-f0-9]{64}$'),
    canonical_payment_reference text NOT NULL CHECK (length(btrim(canonical_payment_reference)) BETWEEN 3 AND 160),
    operating_organisation_id uuid NOT NULL,
    responsible_legal_entity_id text NOT NULL CHECK (length(btrim(responsible_legal_entity_id)) BETWEEN 3 AND 160),
    cp_mandate_id uuid NOT NULL,
    legal_actor_evidence_reference text NOT NULL CHECK (length(btrim(legal_actor_evidence_reference)) BETWEEN 3 AND 256),
    provider_certification_reference text NOT NULL CHECK (length(btrim(provider_certification_reference)) BETWEEN 3 AND 256),
    merchant_activation_reference text NOT NULL CHECK (length(btrim(merchant_activation_reference)) BETWEEN 3 AND 256),
    market_code text NOT NULL CHECK (market_code ~ '^[A-Z]{2}$'),
    currency_code text NOT NULL CHECK (currency_code ~ '^[A-Z]{3}$'),
    amount_minor bigint NOT NULL CHECK (amount_minor > 0),
    recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    UNIQUE (tenant_id, operation_kind, idempotency_key),
    UNIQUE (tenant_id, execution_id)
);
COMMENT ON TABLE payment_ops.native_execution IS
    'Immutable operation intent only; reference columns are not proof that external authorities or PSP certification exist.';

-- Provider callbacks MUST be verified cryptographically and context-bound by
-- the application BEFORE append. Every receipt and its historical scope is
-- immutable, even after refund/settlement or corporate restructuring.
CREATE TABLE payment_ops.provider_receipt (
    receipt_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id text NOT NULL,
    execution_id uuid NOT NULL,
    provider_name text NOT NULL CHECK (length(btrim(provider_name)) BETWEEN 2 AND 100),
    provider_event_id text NOT NULL CHECK (length(btrim(provider_event_id)) BETWEEN 3 AND 180),
    observed_status text NOT NULL CHECK (observed_status IN
        ('AUTHORIZED','CAPTURED','DECLINED','PENDING','REFUNDED','VOIDED','SETTLED','DISPUTED')),
    verified_payload_sha256 text NOT NULL CHECK (verified_payload_sha256 ~ '^[a-f0-9]{64}$'),
    received_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    FOREIGN KEY (tenant_id, execution_id)
       REFERENCES payment_ops.native_execution (tenant_id, execution_id),
    UNIQUE (provider_name, provider_event_id)
);

-- Immutable dispatch facts; retry delivery state belongs in a separate
-- delivery cursor, never an UPDATE to a historical payment event.
CREATE TABLE payment_ops.transaction_outbox (
    event_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id text NOT NULL,
    execution_id uuid NOT NULL,
    event_type text NOT NULL CHECK (event_type IN
       ('payments.native-operation.reserved.v1',
        'payments.provider-response.observed.v1',
        'payments.settlement.reconciliation-requested.v1')),
    event_payload jsonb NOT NULL CHECK (jsonb_typeof(event_payload) = 'object'),
    correlation_id uuid NOT NULL,
    recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    FOREIGN KEY (tenant_id, execution_id)
       REFERENCES payment_ops.native_execution (tenant_id, execution_id)
);

CREATE INDEX native_execution_actor_scope_idx
    ON payment_ops.native_execution
    (tenant_id, responsible_legal_entity_id, market_code, recorded_at);
CREATE INDEX provider_receipt_execution_idx
    ON payment_ops.provider_receipt (tenant_id, execution_id, received_at);
CREATE INDEX transaction_outbox_delivery_idx
    ON payment_ops.transaction_outbox (tenant_id, recorded_at, event_id);

-- FORCE prevents even ordinary table owners from silently bypassing RLS.
-- Superusers/BYPASSRLS roles are prohibited for Baobab application traffic.
ALTER TABLE payment_ops.native_execution ENABLE ROW LEVEL SECURITY;
ALTER TABLE payment_ops.native_execution FORCE ROW LEVEL SECURITY;
ALTER TABLE payment_ops.provider_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE payment_ops.provider_receipt FORCE ROW LEVEL SECURITY;
ALTER TABLE payment_ops.transaction_outbox ENABLE ROW LEVEL SECURITY;
ALTER TABLE payment_ops.transaction_outbox FORCE ROW LEVEL SECURITY;

CREATE POLICY execution_tenant ON payment_ops.native_execution
    USING (tenant_id = nullif(current_setting('baobab.tenant_id', true), ''))
    WITH CHECK (tenant_id = nullif(current_setting('baobab.tenant_id', true), ''));
CREATE POLICY receipt_tenant ON payment_ops.provider_receipt
    USING (tenant_id = nullif(current_setting('baobab.tenant_id', true), ''))
    WITH CHECK (tenant_id = nullif(current_setting('baobab.tenant_id', true), ''));
CREATE POLICY outbox_tenant ON payment_ops.transaction_outbox
    USING (tenant_id = nullif(current_setting('baobab.tenant_id', true), ''))
    WITH CHECK (tenant_id = nullif(current_setting('baobab.tenant_id', true), ''));

CREATE FUNCTION payment_ops.refuse_historical_rewrite() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'payment history is immutable: record a new operation/receipt/event';
END;
$$;
CREATE TRIGGER immutable_execution BEFORE UPDATE OR DELETE ON payment_ops.native_execution
    FOR EACH ROW EXECUTE FUNCTION payment_ops.refuse_historical_rewrite();
CREATE TRIGGER immutable_receipt BEFORE UPDATE OR DELETE ON payment_ops.provider_receipt
    FOR EACH ROW EXECUTE FUNCTION payment_ops.refuse_historical_rewrite();
CREATE TRIGGER immutable_outbox BEFORE UPDATE OR DELETE ON payment_ops.transaction_outbox
    FOR EACH ROW EXECUTE FUNCTION payment_ops.refuse_historical_rewrite();

-- Function enforces durable idempotent operation reservation; it cannot
-- self-approve IAM, mandate, documentary, merchant or PSP readiness.
-- Those must be verified before this routine is reached.
CREATE FUNCTION payment_ops.reserve_native_execution(
    p_tenant text,
    p_kind text,
    p_key text,
    p_hash text,
    p_payment_ref text,
    p_organisation uuid,
    p_legal_actor text,
    p_mandate uuid,
    p_actor_evidence text,
    p_certification text,
    p_merchant_activation text,
    p_market text,
    p_currency text,
    p_amount bigint
) RETURNS uuid LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE
    v_execution uuid;
    v_prior payment_ops.native_execution%ROWTYPE;
BEGIN
    IF p_tenant IS DISTINCT FROM nullif(current_setting('baobab.tenant_id', true), '') THEN
        RAISE EXCEPTION 'tenant context required' USING ERRCODE = '42501';
    END IF;
    INSERT INTO payment_ops.native_execution (
        tenant_id, operation_kind, idempotency_key, request_sha256,
        canonical_payment_reference, operating_organisation_id,
        responsible_legal_entity_id, cp_mandate_id, legal_actor_evidence_reference,
        provider_certification_reference, merchant_activation_reference,
        market_code, currency_code, amount_minor
    ) VALUES (
        p_tenant, p_kind, p_key, p_hash, p_payment_ref, p_organisation,
        p_legal_actor, p_mandate, p_actor_evidence, p_certification,
        p_merchant_activation, p_market, p_currency, p_amount
    ) ON CONFLICT (tenant_id, operation_kind, idempotency_key)
        DO NOTHING RETURNING execution_id INTO v_execution;
    IF v_execution IS NOT NULL THEN
        RETURN v_execution;
    END IF;
    SELECT * INTO v_prior FROM payment_ops.native_execution
      WHERE tenant_id = p_tenant AND operation_kind = p_kind
        AND idempotency_key = p_key;
    IF NOT FOUND OR v_prior.request_sha256 <> p_hash
        OR v_prior.canonical_payment_reference <> p_payment_ref
        OR v_prior.operating_organisation_id <> p_organisation
        OR v_prior.responsible_legal_entity_id <> p_legal_actor
        OR v_prior.cp_mandate_id <> p_mandate
        OR v_prior.legal_actor_evidence_reference <> p_actor_evidence
        OR v_prior.provider_certification_reference <> p_certification
        OR v_prior.merchant_activation_reference <> p_merchant_activation
        OR v_prior.market_code <> p_market OR v_prior.currency_code <> p_currency
        OR v_prior.amount_minor <> p_amount THEN
        RAISE EXCEPTION 'payment idempotency conflict' USING ERRCODE = '23505';
    END IF;
    RETURN v_prior.execution_id;
END;
$$;
COMMIT;
