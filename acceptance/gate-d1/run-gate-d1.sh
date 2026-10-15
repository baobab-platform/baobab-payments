#!/usr/bin/env bash
# P0-04B Gate D1 for Payments (ADR-SHARED-035). Runs the release Payments
# service through its HTTP handlers against the real CP binary, real signed
# workload tokens, PostgreSQL 17 and a deployed HyperSwitch router whose own
# Stripe connector points at the deterministic test processor, then drives
# every scenario and writes the evidence bundle.
#
#   run-gate-d1.sh <evidence-dir>
#
# Required environment:
#   BAOBAB_ENVIRONMENT=integration   anything else is refused
#   PAYMENTS_IMAGE                   the Payments image under test
#   CP_SRC                           baobab-cp checkout at the pinned commit
#   HS_SRC                           hyperswitch checkout (config, migrations)
# Optional: GATE_D_KEEP=1 leaves the stack running for inspection.
set -euo pipefail

if [ "${BAOBAB_ENVIRONMENT:-}" != "integration" ]; then
  echo "Gate D1 runs only with BAOBAB_ENVIRONMENT=integration" >&2
  exit 2
fi
: "${PAYMENTS_IMAGE:?PAYMENTS_IMAGE is required}" "${CP_SRC:?CP_SRC is required}" "${HS_SRC:?HS_SRC is required}"
OUT=$(mkdir -p "${1:?evidence directory required}" && cd "$1" && pwd)
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
RUN=$(mktemp -d)
export D1_RUN=$RUN PAYMENTS_IMAGE
export CP_IMAGE=baobab-gate-d1/cp:local PYTHON_IMAGE=baobab-gate-d1/python:local
export PAYDB_PASSWORD="paydb-$(openssl rand -hex 12)" CPDB_PASSWORD="cpdb-$(openssl rand -hex 12)"
export PAYMENTS_ENGINE_INSTANCE_ID=ei_pending
COMPOSE=(docker compose -f "$HERE/compose.yaml" -p baobab-gate-d1)
log() { printf '[gate-d1] %s\n' "$*" | tee -a "$OUT/run.log"; }

collect() {
  for s in payments cp hyperswitch issuer processor edge; do
    "${COMPOSE[@]}" logs --no-color "$s" > "$OUT/$s.log" 2>&1 || true
  done
  # Logs are evidence: they must never carry a token, a secret or a card number.
  if grep -REl 'eyJ[A-Za-z0-9_-]{20,}\.|4242424242424242|snd_[A-Za-z0-9]{20,}' "$OUT"/*.log >/dev/null 2>&1; then
    log "a log carries a token or a card number"; exit 1
  fi
}
cleanup() {
  collect
  if [ "${GATE_D_KEEP:-0}" != "1" ]; then "${COMPOSE[@]}" down -v --remove-orphans >/dev/null 2>&1 || true; rm -rf "$RUN"; fi
}
trap cleanup EXIT
: > "$OUT/run.log"

log "throwaway CA and the TLS edge certificate"
mkdir -p "$RUN"/{edge,cp,payments,issuer,hyperswitch-config}
openssl req -x509 -newkey rsa:2048 -nodes -days 1 -subj "/CN=Baobab Gate D1 throwaway CA" \
  -keyout "$RUN/ca.key" -out "$RUN/ca.pem" 2>/dev/null
openssl req -newkey rsa:2048 -nodes -subj "/CN=d1.test" -keyout "$RUN/edge/server.key" -out "$RUN/edge.csr" 2>/dev/null
printf 'subjectAltName=DNS:cp.d1.test,DNS:iam.d1.test,DNS:hyperswitch.d1.test\nextendedKeyUsage=serverAuth\n' > "$RUN/edge.ext"
openssl x509 -req -in "$RUN/edge.csr" -CA "$RUN/ca.pem" -CAkey "$RUN/ca.key" -CAcreateserial -days 1 \
  -extfile "$RUN/edge.ext" -out "$RUN/edge/server.pem" 2>/dev/null
cp "$RUN/ca.pem" "$RUN/cp/ca.pem"; cp "$RUN/ca.pem" "$RUN/payments/ca.pem"; cp "$RUN/ca.pem" "$OUT/ca.pem"
chmod -R a+rX "$RUN"

log "workload clients, the CI-only registry snapshot and secrets"
PAYMENTS_SECRET=$(openssl rand -hex 24)
printf '%s' "$PAYMENTS_SECRET" > "$RUN/payments/payments-workload-secret"
python3 - "$RUN/issuer/clients.json" "$PAYMENTS_SECRET" <<'EOF'
import json, sys
json.dump({"baobab-payments-legal-actor-assessor-staging": {
    "secret": sys.argv[2], "audience": "baobab-control-plane",
    "scopes": ["context:resolve", "legal-actor:assess", "documentary:restriction-assess",
               "payments-money-movement-authority:resolve"]}}, open(sys.argv[1], "w"))
EOF
cp "$HERE/cp/workload-registry.yaml" "$RUN/cp/workload-registry.yaml"
printf 'postgres://payment_runtime:%s@127.0.0.1:5432/baobab_payments' "$PAYDB_PASSWORD" > "$RUN/payments/payments-database-url"

log "HyperSwitch configuration: the Stripe connector points at the test processor"
cp -r "$HS_SRC/config/." "$RUN/hyperswitch-config/"
sed -i 's#^stripe.base_url = "https://api.stripe.com/"#stripe.base_url = "http://processor:9000/"#' \
  "$RUN/hyperswitch-config/docker_compose.toml"
grep -q '^stripe.base_url = "http://processor:9000/"' "$RUN/hyperswitch-config/docker_compose.toml"

log "images: CP release binaries at $(git -C "$CP_SRC" rev-parse HEAD), test services"
mkdir -p "$RUN/build/cp" "$RUN/build/py"
(cd "$CP_SRC" && CGO_ENABLED=0 go build -trimpath -ldflags='-s -w' -o "$RUN/build/cp/controlplane" ./cmd/controlplane \
  && CGO_ENABLED=0 go build -trimpath -ldflags='-s -w' -o "$RUN/build/cp/migrate" ./cmd/migrate)
docker build -q -t "$CP_IMAGE" -f "$HERE/images/cp.Dockerfile" "$RUN/build/cp" >/dev/null
mkdir -p "$RUN/build/py/wheels" "$RUN/build/py/issuer" "$RUN/build/py/processor"
pip download --quiet --only-binary=:all: --platform manylinux_2_28_x86_64 --platform manylinux2014_x86_64 \
  --platform manylinux_2_17_x86_64 --python-version 3.12 --implementation cp "cryptography==44.0.3" \
  -d "$RUN/build/py/wheels"
cp "$HERE/issuer/issuer.py" "$RUN/build/py/issuer/"; cp "$HERE/processor/processor.py" "$RUN/build/py/processor/"
docker build -q -t "$PYTHON_IMAGE" -f "$HERE/images/python.Dockerfile" "$RUN/build/py" >/dev/null

log "HyperSwitch database, cache and migrations"
"${COMPOSE[@]}" up -d --wait pg redis-standalone processor >/dev/null
hs_failures=0
for dir in $(ls "$HS_SRC/migrations" | sort); do
  f="$HS_SRC/migrations/$dir/up.sql"; [ -f "$f" ] || continue
  # CREATE INDEX CONCURRENTLY cannot run inside a transaction.
  if grep -qi concurrently "$f"; then single=(); else single=(-1); fi
  "${COMPOSE[@]}" exec -T pg psql -q -X -v ON_ERROR_STOP=1 -U db_user -d hyperswitch_db "${single[@]}" < "$f" \
    >/dev/null 2>>"$OUT/hyperswitch-migrations.log" || hs_failures=$((hs_failures + 1))
done
log "HyperSwitch migrations: $(ls "$HS_SRC/migrations" | wc -l) applied, $hs_failures failed"
[ "$hs_failures" -eq 0 ]

log "CP and Payments databases, the TLS edge and the test issuer"
"${COMPOSE[@]}" up -d --wait cpdb paydb issuer edge >/dev/null
"${COMPOSE[@]}" run --rm --no-deps --entrypoint /migrate cp >"$OUT/cp-migrate.log" 2>&1
for f in "$REPO"/db/migrations/*.sql; do
  "${COMPOSE[@]}" exec -T paydb psql -q -X -v ON_ERROR_STOP=1 -U baobab -d baobab_payments < "$f" >/dev/null
done
"${COMPOSE[@]}" exec -T paydb psql -q -X -v ON_ERROR_STOP=1 -U baobab -d baobab_payments \
  -v runtime_password="$PAYDB_PASSWORD" < "$HERE/seed/payments-runtime-role.sql" >/dev/null

log "CP and HyperSwitch"
"${COMPOSE[@]}" up -d cp hyperswitch >/dev/null
for i in $(seq 1 90); do
  curl -sf http://127.0.0.1:18080/health >/dev/null && curl -sf http://127.0.0.1:18082/healthz >/dev/null && break
  sleep 2
done
curl -sf http://127.0.0.1:18080/health >/dev/null || { log "HyperSwitch did not start"; exit 1; }
curl -sf http://127.0.0.1:18082/healthz >/dev/null || { log "CP did not start"; exit 1; }

log "seeding CP, HyperSwitch and the merchant; then the scenarios"
python3 "$HERE/driver/gate_d1.py" --run "$RUN" --out "$OUT" --compose "$HERE/compose.yaml"
grep -q '^Gate D1 failures: 0$' "$OUT/gate-d1-report.txt"
