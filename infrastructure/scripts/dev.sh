#!/bin/bash
# ============================================
# DiscordSentinel - Dev Launcher
# Lance TOUT : API, API ML, 10 bots, 6 workers, desktop
# ============================================

set -e

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$ROOT_DIR/.logs"
mkdir -p "$LOG_DIR"

# Couleurs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# PIDs des processus lances
PIDS=()

cleanup() {
    echo ""
    echo -e "${YELLOW}Arret de tous les services...${NC}"
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null
        fi
    done
    wait 2>/dev/null
    echo -e "${GREEN}Tous les services sont arretes.${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Charger .env si present
if [ -f "$ROOT_DIR/.env" ]; then
    echo -e "${CYAN}Chargement de .env...${NC}"
    set -a
    source "$ROOT_DIR/.env"
    set +a
fi

# ──────────────────────────────────────────────
# Verification des prerequis
# ──────────────────────────────────────────────

check_prereqs() {
    local missing=0

    if ! command -v cargo &>/dev/null; then
        echo -e "${RED}cargo non trouve. Installe Rust : https://rustup.rs${NC}"
        missing=1
    fi

    if ! command -v node &>/dev/null; then
        echo -e "${RED}node non trouve. Installe Node.js : https://nodejs.org${NC}"
        missing=1
    fi

    if ! command -v python3 &>/dev/null && ! command -v python &>/dev/null; then
        echo -e "${YELLOW}python3 non trouve — l'API ML ne sera pas lancee${NC}"
    fi

    if [ "$missing" -eq 1 ]; then
        exit 1
    fi
}

# ──────────────────────────────────────────────
# Lancement des services
# ──────────────────────────────────────────────

start_service() {
    local name="$1"
    local dir="$2"
    local cmd="$3"
    local color="$4"
    local log_file="$LOG_DIR/$name.log"

    if [ ! -d "$dir" ]; then
        echo -e "${YELLOW}[SKIP] $name - dossier $dir introuvable${NC}"
        return
    fi

    echo -e "${color}[START] $name${NC} (logs: .logs/$name.log)"
    (cd "$dir" && $cmd > "$log_file" 2>&1) &
    PIDS+=($!)
}

# ──────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────

echo ""
echo -e "${CYAN}================================================${NC}"
echo -e "${CYAN}   DiscordSentinel - Dev Mode (full stack)${NC}"
echo -e "${CYAN}================================================${NC}"
echo ""

check_prereqs

# ── 1. API Backend (Rust) ──
start_service "api" \
    "$ROOT_DIR/sentinel-api" \
    "cargo run" \
    "$GREEN"

# Attendre que les APIs demarrent avant les bots et workers
start_service "gateway" \
    "$ROOT_DIR/platform-gateway" \
    "cargo run" \
    "$GREEN"

# Attendre que les APIs + gateway demarrent avant les bots et workers
sleep 3

# ── 4. Workers (6) ──
start_service "analytics-worker" \
    "$ROOT_DIR/services/workers/analytics-worker" \
    "cargo run" \
    "$YELLOW"

start_service "moderation-worker" \
    "$ROOT_DIR/services/workers/moderation-worker" \
    "cargo run" \
    "$YELLOW"

start_service "monitoring-worker" \
    "$ROOT_DIR/services/workers/monitoring-worker" \
    "cargo run" \
    "$YELLOW"

start_service "cache-worker" \
    "$ROOT_DIR/services/workers/cache-worker" \
    "cargo run" \
    "$YELLOW"

start_service "cleanup-worker" \
    "$ROOT_DIR/services/workers/cleanup-worker" \
    "cargo run" \
    "$YELLOW"

# ── 5. Tous les bots Discord (11) ──
start_service "audit-bot" \
    "$ROOT_DIR/bots/audit-bot" \
    "cargo run" \
    "$BLUE"

start_service "automod-bot" \
    "$ROOT_DIR/bots/automod-bot" \
    "cargo run" \
    "$BLUE"

start_service "image-bot" \
    "$ROOT_DIR/bots/image-bot" \
    "cargo run" \
    "$BLUE"

start_service "moderation-bot" \
    "$ROOT_DIR/bots/moderation-bot" \
    "cargo run" \
    "$BLUE"

start_service "community-bot" \
    "$ROOT_DIR/bots/community-bot" \
    "cargo run" \
    "$BLUE"

start_service "security-bot" \
    "$ROOT_DIR/bots/security-bot" \
    "cargo run" \
    "$BLUE"

start_service "progression-bot" \
    "$ROOT_DIR/bots/progression-bot" \
    "cargo run" \
    "$BLUE"

start_service "ticket-bot" \
    "$ROOT_DIR/bots/ticket-bot" \
    "cargo run" \
    "$BLUE"

start_service "voice-bot" \
    "$ROOT_DIR/bots/voice-bot" \
    "cargo run" \
    "$BLUE"

start_service "roles-bot" \
    "$ROOT_DIR/bots/roles-bot" \
    "cargo run" \
    "$BLUE"

# ── 6. Desktop App (Tauri + Vue) ──
if [ -d "$ROOT_DIR/apps/desktop" ]; then
    if [ ! -d "$ROOT_DIR/apps/desktop/node_modules" ]; then
        echo -e "${YELLOW}[INSTALL] Desktop - npm install...${NC}"
        (cd "$ROOT_DIR/apps/desktop" && npm install) 2>&1
    fi
    start_service "desktop" \
        "$ROOT_DIR/apps/desktop" \
        "npm run tauri dev" \
        "$CYAN"
fi

echo ""
echo -e "${GREEN}================================================${NC}"
echo -e "${GREEN}   Tous les services sont lances !${NC}"
echo -e "${GREEN}================================================${NC}"
echo ""
echo -e "  API Backend : ${GREEN}http://localhost:3000${NC}"
echo -e "  API ML      : ${MAGENTA}http://localhost:8000${NC}"
echo -e "  Gateway WS  : ${GREEN}ws://localhost:3001${NC}"
echo -e "  Desktop     : ${CYAN}Tauri app (fenetre native)${NC}"
echo ""
echo -e "  Workers (7) :"
echo -e "    ${YELLOW}analytics-worker   moderation-worker  monitoring-worker${NC}"
echo -e "    ${YELLOW}cache-worker       cleanup-worker${NC}"
echo ""
echo -e "  Bots Discord (11) :"
echo -e "    ${BLUE}audit-bot     automod-bot    image-bot${NC}"
echo -e "    ${BLUE}community-bot moderation-bot security-bot${NC}"
echo -e "    ${BLUE}progression-bot ticket-bot${NC}"
echo -e "    ${BLUE}voice-bot     roles-bot${NC}"
echo ""
echo -e "  Logs        : ${YELLOW}.logs/*.log${NC}"
echo ""
echo -e "${YELLOW}Ctrl+C pour tout arreter${NC}"
echo ""

# Attendre que tous les processus tournent
wait
