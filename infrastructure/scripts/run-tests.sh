#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────
# Script de tests DiscordSentinel
# Lance PostgreSQL + Redis via Docker, execute les
# migrations, puis les tests unitaires + integration.
# ─────────────────────────────────────────────────

cd "$(dirname "$0")/../.."
COMPOSE_FILE="infrastructure/docker/docker-compose.test.yml"
DB_URL="postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test"
REDIS_URL="redis://localhost:6380"
MIGRATIONS_DIR="platform-api/migrations/sentinel"

# Couleurs
GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[INFO]${NC} $*"; }
ok()    { echo -e "${GREEN}[OK]${NC} $*"; }
fail()  { echo -e "${RED}[FAIL]${NC} $*"; }

cleanup() {
    info "Arret des conteneurs de test..."
    docker compose -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || true
}
trap cleanup EXIT

# ── 1. Demarrer les services ──
info "Demarrage PostgreSQL + Redis de test..."
docker compose -f "$COMPOSE_FILE" up -d --wait

ok "Services demarres (postgres:5433, redis:6380)"

# ── 2. Lancer les migrations ──
info "Application des migrations..."
export DATABASE_URL="$DB_URL"
cargo sqlx migrate run --source "$MIGRATIONS_DIR" 2>/dev/null || {
    # Si sqlx-cli n'est pas installe, utiliser psql dans le container de test
    # (evite de requerir psql sur le host Windows).
    #
    # Note Windows/Git Bash : MSYS_NO_PATHCONV=1 desactive la conversion
    # automatique des paths POSIX (/tmp/...) en paths Windows (C:/...)
    # sur les arguments passes aux commandes docker.
    info "sqlx-cli non disponible, application via docker exec..."
    MSYS_NO_PATHCONV=1 docker cp "$MIGRATIONS_DIR/." sentinel-test-postgres:/tmp/migrations_run
    MIGRATION_FAIL=0
    for f in "$MIGRATIONS_DIR"/*.sql; do
        BASENAME=$(basename "$f")
        # Pattern OUT=$(...) + RC capture : set -e ne peut pas killer
        # le script sur une assignment avec || ... bloc.
        OUT=$(MSYS_NO_PATHCONV=1 docker exec sentinel-test-postgres psql -U sentinel_test -d sentinel_test \
            -v ON_ERROR_STOP=1 -f "/tmp/migrations_run/$BASENAME" 2>&1) && RC=0 || RC=$?
        if [ "$RC" -ne 0 ]; then
            fail "Migration $BASENAME a echoue :"
            echo "$OUT" | tail -10
            MIGRATION_FAIL=1
        fi
    done
    if [ "$MIGRATION_FAIL" -ne 0 ]; then
        fail "Une ou plusieurs migrations ont echoue — arret"
        exit 1
    fi
}
ok "Migrations appliquees"

# ── 3. Tests unitaires (processus deployes + API) ──
info "Tests unitaires..."
FAILED=0

for service in sentinel-bot nexus-bot atrium-bot platform-scheduler platform-gateway ops-agent auth-api docker-agent; do
    if [ -f "$service/Cargo.toml" ]; then
        info "  $service..."
        if cargo test --manifest-path "$service/Cargo.toml" --quiet 2>&1; then
            ok "  $service"
        else
            fail "  $service"
            FAILED=$((FAILED + 1))
        fi
    fi
done

info "  API (lib)..."
if cargo test --manifest-path platform-api/Cargo.toml --lib --quiet 2>&1; then
    ok "  API (lib)"
else
    fail "  API (lib)"
    FAILED=$((FAILED + 1))
fi

# ── 4. Tests d'integration HTTP (API) ──
info "Tests d'integration HTTP..."
export DATABASE_URL="$DB_URL"
export REDIS_URL="$REDIS_URL"
# Noms lus par `platform-api/src/sentinel/config.rs`. Sous les anciens noms (API_KEY /
# REQUIRE_API_KEY), la config retombait sur son defaut `require = true` avec une
# cle vide et l'API sortait en exit(1) au demarrage.
export SENTINEL_API_KEY=""
export SENTINEL_REQUIRE_API_KEY="false"

if cargo test --manifest-path platform-api/Cargo.toml --tests --quiet 2>&1; then
    ok "Tests d'integration HTTP"
else
    fail "Tests d'integration HTTP"
    FAILED=$((FAILED + 1))
fi

# ── 5. Resultat ──
echo ""
if [ "$FAILED" -eq 0 ]; then
    ok "Tous les tests passent !"
    exit 0
else
    fail "$FAILED suite(s) de tests en echec"
    exit 1
fi
