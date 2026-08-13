# Script de tests DiscordSentinel (PowerShell)
# Usage : .\scripts\run-tests.ps1

$ErrorActionPreference = "Continue"
Set-Location (Join-Path $PSScriptRoot "..\..")
$COMPOSE_FILE = "infrastructure/docker/docker-compose.test.yml"
$DB_URL = "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test"
$REDIS_URL = "redis://localhost:6380"
$MIGRATIONS_DIR = "platform-api/migrations/sentinel"
$FAILED = 0

function Info($msg)  { Write-Host "[INFO] $msg" -ForegroundColor Cyan }
function Ok($msg)    { Write-Host "[OK] $msg" -ForegroundColor Green }
function Fail($msg)  { Write-Host "[FAIL] $msg" -ForegroundColor Red }

# 1. Demarrer les services
Info "Demarrage PostgreSQL + Redis de test..."
docker compose -f $COMPOSE_FILE up -d --wait
Ok "Services demarres (postgres:5433, redis:6380)"

# 2. Migrations
Info "Application des migrations..."
$env:DATABASE_URL = $DB_URL
cargo sqlx migrate run --source $MIGRATIONS_DIR 2>$null
if ($LASTEXITCODE -ne 0) {
    Info "sqlx-cli non disponible, application manuelle..."
    Get-ChildItem "$MIGRATIONS_DIR/*.sql" | Sort-Object Name | ForEach-Object {
        psql $DB_URL -f $_.FullName 2>$null
    }
}
Ok "Migrations appliquees"

# 3. Tests unitaires
Info "Tests unitaires..."

$services = @(
    "sentinel-bot", "nexus-bot", "atrium-bot", "platform-scheduler",
    "platform-gateway", "ops-agent", "auth-api", "docker-agent"
)

foreach ($service in $services) {
    $manifest = "$service/Cargo.toml"
    if (Test-Path $manifest) {
        Info "  $service..."
        cargo test --manifest-path $manifest --quiet 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) { Ok "  $service" } else { Fail "  $service"; $FAILED++ }
    }
}

Info "  API (lib)..."
cargo test --manifest-path platform-api/Cargo.toml --lib --quiet 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) { Ok "  API (lib)" } else { Fail "  API (lib)"; $FAILED++ }

# 4. Tests d'integration
Info "Tests d'integration HTTP..."
$env:DATABASE_URL = $DB_URL
$env:REDIS_URL = $REDIS_URL
# Noms lus par `platform-api/src/sentinel/config.rs` (cf. run-tests.sh).
$env:SENTINEL_API_KEY = ""
$env:SENTINEL_REQUIRE_API_KEY = "false"

cargo test --manifest-path platform-api/Cargo.toml --tests --quiet 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) { Ok "Tests d'integration HTTP" } else { Fail "Tests d'integration HTTP"; $FAILED++ }

# 5. Cleanup
Info "Arret des conteneurs..."
docker compose -f $COMPOSE_FILE down -v --remove-orphans 2>$null

# Resultat
Write-Host ""
if ($FAILED -eq 0) {
    Ok "Tous les tests passent !"
    exit 0
} else {
    Fail "$FAILED suite(s) de tests en echec"
    exit 1
}
