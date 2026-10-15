-- Gate D1: the Payments runtime role. It can append to the journal and read
-- provider readiness, and can never activate a provider for itself or rewrite
-- history (the immutability triggers refuse UPDATE and DELETE for everyone).
\set ON_ERROR_STOP on
CREATE ROLE payment_runtime LOGIN PASSWORD :'runtime_password';
GRANT USAGE ON SCHEMA payment_ops TO payment_runtime;
GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA payment_ops TO payment_runtime;
REVOKE INSERT ON payment_ops.provider_activation, payment_ops.provider_activation_revocation FROM payment_runtime;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA payment_ops TO payment_runtime;
