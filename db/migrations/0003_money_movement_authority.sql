-- P0-04B Payments (ADR-SHARED-035): every consequential money movement needs
-- current, operation-specific CP legal authority AND Payments' own financial
-- execution approval. Install AFTER 0002 in PostgreSQL 17.
--
-- These fences make the journal, not application code, the last line of
-- defence: a reservation or a provider dispatch that lacks a current CP
-- decision for its exact instruction fingerprint, lacks a current provider
-- activation, or breaks a financial limit, is refused even by direct SQL as
-- the restricted runtime role. Payouts are never reservable here.
BEGIN;

-- Money-movement identity on the existing immutable reservation. CREATE_INTENT
-- rows keep these NULL: creating an intent moves no money.
ALTER TABLE payment_ops.native_execution
    ADD COLUMN operation_class text
        CHECK (operation_class IS NULL OR operation_class ~ '^payments\.[a-z-]+\.[a-z-]+$'),
    ADD COLUMN operation_reference text
        CHECK (operation_reference IS NULL OR (length(operation_reference) BETWEEN 1 AND 256
               AND operation_reference ~ '^[\x21-\x7E]+$')),
    ADD COLUMN instruction_fingerprint text
        CHECK (instruction_fingerprint IS NULL OR instruction_fingerprint ~ '^[0-9a-f]{64}$'),
    ADD COLUMN parent_reference text
        CHECK (parent_reference IS NULL OR length(btrim(parent_reference)) BETWEEN 3 AND 160),
    ADD COLUMN merchant_profile_reference text
        CHECK (merchant_profile_reference IS NULL OR length(btrim(merchant_profile_reference)) BETWEEN 3 AND 160),
    ADD COLUMN destination_reference text
        CHECK (destination_reference IS NULL OR length(btrim(destination_reference)) BETWEEN 3 AND 160),
    ADD COLUMN recording_txid bigint NOT NULL DEFAULT txid_current();

ALTER TABLE payment_ops.transaction_outbox
DROP CONSTRAINT transaction_outbox_event_type_check;
ALTER TABLE payment_ops.transaction_outbox
ADD CONSTRAINT transaction_outbox_event_type_check CHECK (event_type IN (
  'payments.native-operation.reserved.v1',
  'payments.provider-response.observed.v1',
  'payments.settlement.reconciliation-requested.v1',
  'payments.provider-webhook.verified.v1',
  'payments.settlement.matched.v1',
  'payments.settlement.exception.v1',
  'payments.money-movement.dispatched.v1',
  'payments.money-movement.reconciled.v1'
));

-- One row per AUTHORIZED CP decision Payments acted on. Denials never reach
-- the journal; CP retains them. received_at is Payments' own clock, so a
-- decision claiming a longer lease can never extend a movement.
CREATE TABLE payment_ops.authority_decision (
    decision_id uuid PRIMARY KEY,
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL CHECK (length(btrim(responsible_legal_entity_id)) BETWEEN 3 AND 160),
    operation_class text NOT NULL CHECK (operation_class IN (
        'payments.authorization.create', 'payments.capture.execute',
        'payments.refund.execute', 'payments.authorization.void')),
    operation_reference text NOT NULL CHECK (length(operation_reference) BETWEEN 1 AND 256
        AND operation_reference ~ '^[\x21-\x7E]+$'),
    attempt integer NOT NULL CHECK (attempt BETWEEN 1 AND 1000),
    instruction_fingerprint text NOT NULL CHECK (instruction_fingerprint ~ '^[0-9a-f]{64}$'),
    mandate_id uuid NOT NULL,
    evaluated_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL,
    received_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    recording_txid bigint NOT NULL DEFAULT txid_current(),
    CHECK (expires_at > evaluated_at AND expires_at <= evaluated_at + interval '30 seconds'),
    UNIQUE (tenant_id, responsible_legal_entity_id, operation_reference, attempt)
);

-- Exactly one provider instruction per durable reservation, unless the
-- provider has since confirmed that it never received the previous one.
CREATE TABLE payment_ops.dispatch_attempt (
    dispatch_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL,
    execution_id uuid NOT NULL,
    decision_id uuid NOT NULL REFERENCES payment_ops.authority_decision (decision_id),
    activation_id uuid NOT NULL,
    dispatched_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    recording_txid bigint NOT NULL DEFAULT txid_current(),
    FOREIGN KEY (tenant_id, responsible_legal_entity_id, execution_id)
       REFERENCES payment_ops.native_execution (tenant_id, responsible_legal_entity_id, execution_id),
    UNIQUE (decision_id)
);

-- What the provider said, or that nobody knows yet. UNKNOWN never counts as
-- success; ABSENT means a reconciliation proved the provider has no record.
CREATE TABLE payment_ops.provider_observation (
    observation_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL,
    execution_id uuid NOT NULL,
    dispatch_id uuid NOT NULL REFERENCES payment_ops.dispatch_attempt (dispatch_id),
    outcome text NOT NULL CHECK (outcome IN ('CONFIRMED', 'REJECTED', 'UNKNOWN', 'ABSENT')),
    source text NOT NULL CHECK (source IN ('DISPATCH_RESPONSE', 'RECONCILIATION')),
    provider_reference text CHECK (provider_reference IS NULL OR provider_reference ~ '^[A-Za-z0-9_-]{3,120}$'),
    provider_status text CHECK (provider_status IS NULL OR provider_status ~ '^[a-z_]{1,64}$'),
    provider_environment text NOT NULL CHECK (provider_environment IN ('SANDBOX', 'STAGING', 'PRODUCTION')),
    observed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK (outcome <> 'CONFIRMED' OR (provider_reference IS NOT NULL AND provider_status IS NOT NULL)),
    FOREIGN KEY (tenant_id, responsible_legal_entity_id, execution_id)
       REFERENCES payment_ops.native_execution (tenant_id, responsible_legal_entity_id, execution_id)
);

-- Independent provider readiness (ADR-PAY-0022): a certified merchant profile
-- activated for this legal entity, market, currency and operation. Revocation
-- is a new row, never an edit. PAYOUT can never be activated here. The
-- payment runtime role is granted SELECT only: it can never activate itself.
CREATE TABLE payment_ops.provider_activation (
    activation_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL CHECK (length(btrim(responsible_legal_entity_id)) BETWEEN 3 AND 160),
    merchant_profile_reference text NOT NULL CHECK (length(btrim(merchant_profile_reference)) BETWEEN 3 AND 160),
    market_code text NOT NULL CHECK (market_code ~ '^[A-Z]{2}$'),
    currency_code text NOT NULL CHECK (currency_code ~ '^[A-Z]{3}$'),
    operation_kinds text[] NOT NULL CHECK (cardinality(operation_kinds) > 0
        AND operation_kinds <@ ARRAY['AUTHORIZE', 'CAPTURE', 'CANCEL', 'REFUND']::text[]),
    provider_environment text NOT NULL CHECK (provider_environment IN ('SANDBOX', 'STAGING', 'PRODUCTION')),
    provider_certification_reference text NOT NULL CHECK (length(btrim(provider_certification_reference)) BETWEEN 3 AND 256),
    merchant_activation_reference text NOT NULL CHECK (length(btrim(merchant_activation_reference)) BETWEEN 3 AND 256),
    valid_from timestamptz NOT NULL,
    valid_until timestamptz NOT NULL CHECK (valid_until > valid_from),
    recorded_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE payment_ops.provider_activation_revocation (
    activation_id uuid PRIMARY KEY REFERENCES payment_ops.provider_activation (activation_id),
    tenant_id text NOT NULL,
    responsible_legal_entity_id text NOT NULL,
    reason text NOT NULL CHECK (length(btrim(reason)) BETWEEN 3 AND 500),
    revoked_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX authority_decision_operation_idx
    ON payment_ops.authority_decision (tenant_id, responsible_legal_entity_id, operation_reference);
CREATE INDEX native_execution_parent_idx
    ON payment_ops.native_execution (tenant_id, responsible_legal_entity_id, parent_reference)
    WHERE parent_reference IS NOT NULL;
CREATE INDEX provider_observation_execution_idx
    ON payment_ops.provider_observation (tenant_id, execution_id, observed_at);

DO $$
DECLARE t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['authority_decision', 'dispatch_attempt', 'provider_observation',
                             'provider_activation', 'provider_activation_revocation'] LOOP
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

-- The latest decisive outcome of an execution: CONFIRMED or REJECTED win over
-- UNKNOWN; ABSENT re-opens it. NULL means nothing decisive is known.
CREATE FUNCTION payment_ops.execution_outcome(p_execution uuid) RETURNS text
LANGUAGE sql STABLE SECURITY INVOKER AS $$
    SELECT o.outcome FROM payment_ops.provider_observation o
     WHERE o.execution_id = p_execution AND o.outcome IN ('CONFIRMED', 'REJECTED', 'ABSENT')
     ORDER BY o.observed_at DESC, o.observation_id DESC LIMIT 1
$$;

-- The current activation for one operation kind, or NULL.
CREATE FUNCTION payment_ops.current_activation(
    p_tenant text, p_legal_actor text, p_profile text, p_market text, p_currency text, p_kind text
) RETURNS uuid LANGUAGE sql STABLE SECURITY INVOKER AS $$
    SELECT a.activation_id FROM payment_ops.provider_activation a
     WHERE a.tenant_id = p_tenant AND a.responsible_legal_entity_id = p_legal_actor
       AND a.merchant_profile_reference = p_profile AND a.market_code = p_market
       AND a.currency_code = p_currency AND p_kind = ANY (a.operation_kinds)
       AND a.valid_from <= clock_timestamp() AND a.valid_until > clock_timestamp()
       AND NOT EXISTS (SELECT 1 FROM payment_ops.provider_activation_revocation r
                        WHERE r.activation_id = a.activation_id)
     ORDER BY a.valid_from DESC, a.activation_id LIMIT 1
$$;

-- A decision is usable only inside min(expires_at, received_at + 30 s).
CREATE FUNCTION payment_ops.decision_is_current(d payment_ops.authority_decision) RETURNS boolean
LANGUAGE sql STABLE AS $$
    SELECT clock_timestamp() < least(d.expires_at, d.received_at + interval '30 seconds')
$$;

CREATE FUNCTION payment_ops.class_kind(p_class text) RETURNS text
LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE p_class
        WHEN 'payments.authorization.create' THEN 'AUTHORIZE'
        WHEN 'payments.capture.execute' THEN 'CAPTURE'
        WHEN 'payments.refund.execute' THEN 'REFUND'
        WHEN 'payments.authorization.void' THEN 'CANCEL'
        WHEN 'payments.payout.execute' THEN 'PAYOUT'
    END
$$;

-- Decisions: an already-expired or payout decision is never recorded.
CREATE FUNCTION payment_ops.guard_authority_decision() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER AS $$
BEGIN
    -- Receipt time and transaction are Payments' own facts, never a caller's.
    NEW.received_at := clock_timestamp();
    NEW.recording_txid := txid_current();
    IF NOT payment_ops.decision_is_current(NEW) THEN
        RAISE EXCEPTION 'expired money-movement decision' USING ERRCODE = 'BPA01';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_authority_decision BEFORE INSERT ON payment_ops.authority_decision
    FOR EACH ROW EXECUTE FUNCTION payment_ops.guard_authority_decision();

-- Reservation fence. Money-moving kinds need a current attempt-1 decision for
-- this exact instruction, recorded in this transaction; a current provider
-- activation; and Payments' own financial limits, evaluated under a lock on
-- the parent payment so concurrent movements serialise.
CREATE FUNCTION payment_ops.guard_money_movement_reservation() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE
    d payment_ops.authority_decision%ROWTYPE;
    v_parent payment_ops.native_execution%ROWTYPE;
    v_authorized bigint;
    v_committed_captures bigint;
    v_confirmed_captures bigint;
    v_committed_refunds bigint;
BEGIN
    NEW.recording_txid := txid_current();
    IF NEW.operation_kind = 'PAYOUT' THEN
        RAISE EXCEPTION 'payouts are not activated: collection authority is never disbursement authority'
            USING ERRCODE = 'BPP01';
    END IF;
    IF NEW.operation_kind NOT IN ('AUTHORIZE', 'CAPTURE', 'CANCEL', 'REFUND') THEN
        RETURN NEW;
    END IF;
    IF NEW.operation_class IS NULL OR payment_ops.class_kind(NEW.operation_class) IS DISTINCT FROM NEW.operation_kind
       OR NEW.operation_reference IS NULL OR NEW.instruction_fingerprint IS NULL
       OR NEW.merchant_profile_reference IS NULL THEN
        RAISE EXCEPTION 'a money movement needs its class, reference, fingerprint and merchant profile'
            USING ERRCODE = 'BPA01';
    END IF;
    SELECT * INTO d FROM payment_ops.authority_decision
     WHERE tenant_id = NEW.tenant_id AND responsible_legal_entity_id = NEW.responsible_legal_entity_id
       AND operation_reference = NEW.operation_reference AND attempt = 1;
    IF NOT FOUND OR d.operation_class <> NEW.operation_class
       OR d.instruction_fingerprint <> NEW.instruction_fingerprint
       OR d.recording_txid <> txid_current() OR d.mandate_id <> NEW.cp_mandate_id
       OR NOT payment_ops.decision_is_current(d) THEN
        RAISE EXCEPTION 'no current CP decision for this exact instruction' USING ERRCODE = 'BPA01';
    END IF;
    IF payment_ops.current_activation(NEW.tenant_id, NEW.responsible_legal_entity_id,
            NEW.merchant_profile_reference, NEW.market_code, NEW.currency_code, NEW.operation_kind) IS NULL THEN
        RAISE EXCEPTION 'no current provider activation for this operation' USING ERRCODE = 'BPR01';
    END IF;

    -- Payments' own financial approval, independent of CP.
    IF NEW.operation_kind = 'AUTHORIZE' THEN
        IF NEW.parent_reference IS NOT NULL OR NEW.destination_reference IS NOT NULL THEN
            RAISE EXCEPTION 'an authorisation has no parent or destination' USING ERRCODE = 'BPS01';
        END IF;
        PERFORM pg_advisory_xact_lock(hashtextextended(
            'payment|' || NEW.tenant_id || '|' || NEW.responsible_legal_entity_id || '|' || NEW.canonical_payment_reference, 0));
        IF EXISTS (SELECT 1 FROM payment_ops.native_execution e
                    WHERE e.tenant_id = NEW.tenant_id AND e.responsible_legal_entity_id = NEW.responsible_legal_entity_id
                      AND e.operation_kind = 'AUTHORIZE' AND e.canonical_payment_reference = NEW.canonical_payment_reference
                      AND payment_ops.execution_outcome(e.execution_id) IS DISTINCT FROM 'REJECTED') THEN
            RAISE EXCEPTION 'this payment already has an authorisation' USING ERRCODE = 'BPS01';
        END IF;
        RETURN NEW;
    END IF;

    IF NEW.parent_reference IS NULL THEN
        RAISE EXCEPTION 'this movement needs its parent payment' USING ERRCODE = 'BPS01';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'payment|' || NEW.tenant_id || '|' || NEW.responsible_legal_entity_id || '|' || NEW.parent_reference, 0));
    SELECT * INTO v_parent FROM payment_ops.native_execution e
     WHERE e.tenant_id = NEW.tenant_id AND e.responsible_legal_entity_id = NEW.responsible_legal_entity_id
       AND e.operation_kind = 'AUTHORIZE' AND e.canonical_payment_reference = NEW.parent_reference
       AND payment_ops.execution_outcome(e.execution_id) = 'CONFIRMED'
     ORDER BY e.recorded_at DESC LIMIT 1;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'the parent payment has no confirmed authorisation' USING ERRCODE = 'BPS01';
    END IF;
    IF v_parent.currency_code <> NEW.currency_code OR v_parent.market_code <> NEW.market_code
       OR v_parent.merchant_profile_reference <> NEW.merchant_profile_reference
       OR v_parent.operating_organisation_id <> NEW.operating_organisation_id THEN
        RAISE EXCEPTION 'a movement must stay within its parent payment''s merchant, market and currency'
            USING ERRCODE = 'BPS01';
    END IF;
    v_authorized := v_parent.amount_minor;
    -- Pending movements count against the balance until the provider rejects them.
    SELECT coalesce(sum(e.amount_minor) FILTER (WHERE e.operation_kind = 'CAPTURE'
                AND payment_ops.execution_outcome(e.execution_id) IS DISTINCT FROM 'REJECTED'), 0),
           coalesce(sum(e.amount_minor) FILTER (WHERE e.operation_kind = 'CAPTURE'
                AND payment_ops.execution_outcome(e.execution_id) = 'CONFIRMED'), 0),
           coalesce(sum(e.amount_minor) FILTER (WHERE e.operation_kind = 'REFUND'
                AND payment_ops.execution_outcome(e.execution_id) IS DISTINCT FROM 'REJECTED'), 0)
      INTO v_committed_captures, v_confirmed_captures, v_committed_refunds
      FROM payment_ops.native_execution e
     WHERE e.tenant_id = NEW.tenant_id AND e.responsible_legal_entity_id = NEW.responsible_legal_entity_id
       AND e.parent_reference = NEW.parent_reference;
    IF EXISTS (SELECT 1 FROM payment_ops.native_execution e
                WHERE e.tenant_id = NEW.tenant_id AND e.responsible_legal_entity_id = NEW.responsible_legal_entity_id
                  AND e.parent_reference = NEW.parent_reference AND e.operation_kind = 'CANCEL'
                  AND payment_ops.execution_outcome(e.execution_id) IS DISTINCT FROM 'REJECTED') THEN
        RAISE EXCEPTION 'the parent payment is voided or being voided' USING ERRCODE = 'BPS01';
    END IF;
    IF NEW.operation_kind = 'CAPTURE' THEN
        IF NEW.destination_reference IS NOT NULL OR v_committed_captures + NEW.amount_minor > v_authorized THEN
            RAISE EXCEPTION 'the capture exceeds the authorised amount' USING ERRCODE = 'BPF01';
        END IF;
    ELSIF NEW.operation_kind = 'CANCEL' THEN
        IF NEW.destination_reference IS NOT NULL OR v_committed_captures > 0
           OR NEW.amount_minor <> v_authorized THEN
            RAISE EXCEPTION 'only an uncaptured authorisation can be voided, in full' USING ERRCODE = 'BPS01';
        END IF;
    ELSIF NEW.operation_kind = 'REFUND' THEN
        IF NEW.destination_reference IS DISTINCT FROM 'ORIGINAL_PAYMENT_METHOD' THEN
            RAISE EXCEPTION 'a refund may only return funds to the original payment method' USING ERRCODE = 'BPF01';
        END IF;
        IF v_committed_refunds + NEW.amount_minor > v_confirmed_captures THEN
            RAISE EXCEPTION 'the refund exceeds the confirmed captured balance' USING ERRCODE = 'BPF01';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_money_movement_reservation BEFORE INSERT ON payment_ops.native_execution
    FOR EACH ROW EXECUTE FUNCTION payment_ops.guard_money_movement_reservation();

-- Dispatch fence: a SECOND, current decision for the same instruction,
-- recorded in this transaction; a current activation; and no earlier
-- dispatch that the provider has not proven ABSENT.
CREATE FUNCTION payment_ops.guard_dispatch_attempt() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE
    e payment_ops.native_execution%ROWTYPE;
    d payment_ops.authority_decision%ROWTYPE;
BEGIN
    NEW.dispatched_at := clock_timestamp();
    NEW.recording_txid := txid_current();
    SELECT * INTO e FROM payment_ops.native_execution
     WHERE tenant_id = NEW.tenant_id AND responsible_legal_entity_id = NEW.responsible_legal_entity_id
       AND execution_id = NEW.execution_id;
    IF NOT FOUND OR e.operation_kind NOT IN ('AUTHORIZE', 'CAPTURE', 'CANCEL', 'REFUND') THEN
        RAISE EXCEPTION 'only a reserved money movement can be dispatched' USING ERRCODE = 'BPP01';
    END IF;
    SELECT * INTO d FROM payment_ops.authority_decision WHERE decision_id = NEW.decision_id;
    IF NOT FOUND OR d.attempt < 2 OR d.operation_reference <> e.operation_reference
       OR d.operation_class <> e.operation_class OR d.instruction_fingerprint <> e.instruction_fingerprint
       OR d.mandate_id <> e.cp_mandate_id OR d.recording_txid <> txid_current()
       OR NOT payment_ops.decision_is_current(d) THEN
        RAISE EXCEPTION 'no current execution-point CP decision for this exact instruction' USING ERRCODE = 'BPA01';
    END IF;
    IF NEW.activation_id IS DISTINCT FROM payment_ops.current_activation(e.tenant_id, e.responsible_legal_entity_id,
            e.merchant_profile_reference, e.market_code, e.currency_code, e.operation_kind) THEN
        RAISE EXCEPTION 'no current provider activation for this dispatch' USING ERRCODE = 'BPR01';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('dispatch|' || NEW.execution_id::text, 0));
    IF EXISTS (SELECT 1 FROM payment_ops.dispatch_attempt p
                WHERE p.execution_id = NEW.execution_id
                  AND NOT EXISTS (SELECT 1 FROM payment_ops.provider_observation o
                                   WHERE o.dispatch_id = p.dispatch_id AND o.outcome = 'ABSENT')) THEN
        RAISE EXCEPTION 'this movement was already dispatched: reconcile, never re-send' USING ERRCODE = 'BPS02';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_dispatch_attempt BEFORE INSERT ON payment_ops.dispatch_attempt
    FOR EACH ROW EXECUTE FUNCTION payment_ops.guard_dispatch_attempt();

-- Observations belong to the dispatch of the same execution.
CREATE FUNCTION payment_ops.guard_provider_observation() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM payment_ops.dispatch_attempt p
                    WHERE p.dispatch_id = NEW.dispatch_id AND p.execution_id = NEW.execution_id) THEN
        RAISE EXCEPTION 'an observation must name its own dispatch' USING ERRCODE = 'BPS01';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_provider_observation BEFORE INSERT ON payment_ops.provider_observation
    FOR EACH ROW EXECUTE FUNCTION payment_ops.guard_provider_observation();

-- COMMIT fences: the decision must still be current when the transaction
-- that relies on it commits, not merely when the row was inserted.
CREATE FUNCTION payment_ops.reservation_commit_fence() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER AS $$
BEGIN
    IF NEW.operation_kind IN ('AUTHORIZE', 'CAPTURE', 'CANCEL', 'REFUND') AND NOT EXISTS (
        SELECT 1 FROM payment_ops.authority_decision d
         WHERE d.tenant_id = NEW.tenant_id AND d.responsible_legal_entity_id = NEW.responsible_legal_entity_id
           AND d.operation_reference = NEW.operation_reference AND d.attempt = 1
           AND d.instruction_fingerprint = NEW.instruction_fingerprint
           AND payment_ops.decision_is_current(d)) THEN
        RAISE EXCEPTION 'the CP decision expired before COMMIT' USING ERRCODE = 'BPA01';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER reservation_commit_fence AFTER INSERT ON payment_ops.native_execution
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION payment_ops.reservation_commit_fence();

CREATE FUNCTION payment_ops.dispatch_commit_fence() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM payment_ops.authority_decision d
                    WHERE d.decision_id = NEW.decision_id AND payment_ops.decision_is_current(d)) THEN
        RAISE EXCEPTION 'the execution-point CP decision expired before COMMIT' USING ERRCODE = 'BPA01';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER dispatch_commit_fence AFTER INSERT ON payment_ops.dispatch_attempt
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION payment_ops.dispatch_commit_fence();

-- The only reservation entry point for money movements. Replays of an
-- identical instruction return the original reservation; anything else under
-- the same key is a conflict.
CREATE FUNCTION payment_ops.reserve_money_movement(
    p_tenant text, p_kind text, p_key text, p_hash text, p_payment_ref text,
    p_organisation uuid, p_legal_actor text, p_mandate uuid, p_actor_evidence text,
    p_certification text, p_merchant_activation text, p_market text, p_currency text,
    p_amount bigint, p_class text, p_operation_ref text, p_fingerprint text,
    p_parent text, p_profile text, p_destination text
) RETURNS uuid LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE
    v_execution uuid;
    v_prior payment_ops.native_execution%ROWTYPE;
BEGIN
    IF p_tenant IS DISTINCT FROM nullif(current_setting('baobab.tenant_id', true), '')
        OR p_legal_actor IS DISTINCT FROM nullif(current_setting('baobab.responsible_legal_entity_id', true), '') THEN
        RAISE EXCEPTION 'tenant context required' USING ERRCODE = '42501';
    END IF;
    SELECT * INTO v_prior FROM payment_ops.native_execution
     WHERE tenant_id = p_tenant AND responsible_legal_entity_id = p_legal_actor
       AND operation_kind = p_kind AND idempotency_key = p_key;
    IF FOUND THEN
        IF v_prior.instruction_fingerprint IS DISTINCT FROM p_fingerprint
           OR v_prior.request_sha256 <> p_hash OR v_prior.operation_reference IS DISTINCT FROM p_operation_ref THEN
            RAISE EXCEPTION 'payment idempotency conflict' USING ERRCODE = '23505';
        END IF;
        RETURN v_prior.execution_id;
    END IF;
    INSERT INTO payment_ops.native_execution (
        tenant_id, operation_kind, idempotency_key, request_sha256,
        canonical_payment_reference, operating_organisation_id,
        responsible_legal_entity_id, cp_mandate_id, legal_actor_evidence_reference,
        provider_certification_reference, merchant_activation_reference,
        market_code, currency_code, amount_minor, operation_class, operation_reference,
        instruction_fingerprint, parent_reference, merchant_profile_reference, destination_reference
    ) VALUES (
        p_tenant, p_kind, p_key, p_hash, p_payment_ref, p_organisation, p_legal_actor, p_mandate,
        p_actor_evidence, p_certification, p_merchant_activation, p_market, p_currency, p_amount,
        p_class, p_operation_ref, p_fingerprint, p_parent, p_profile, p_destination
    ) RETURNING execution_id INTO v_execution;
    RETURN v_execution;
END;
$$;
COMMIT;
