#!/bin/bash
# ============================================
# DiscordSentinel - Dev Launcher
# Lance les processus consolides du workspace et le front Vite.
# ============================================

set -e

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
LOG_DIR="$ROOT_DIR/.logs"
mkdir -p "$LOG_DIR"

# Couleurs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
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

# ── Processus consolides ──
start_service "auth-api" \
    "$ROOT_DIR" \
    "cargo run -p auth-api" \
    "$GREEN"

start_service "platform-api" \
    "$ROOT_DIR" \
    "cargo run -p platform-api --bin platform-api" \
    "$GREEN"

start_service "platform-gateway" \
    "$ROOT_DIR" \
    "cargo run -p platform-gateway" \
    "$GREEN"

sleep 3

start_service "platform-scheduler" \
    "$ROOT_DIR" \
    "cargo run -p platform-scheduler" \
    "$YELLOW"

start_service "ops-agent" \
    "$ROOT_DIR" \
    "cargo run -p ops-agent" \
    "$YELLOW"

start_service "docker-agent" \
    "$ROOT_DIR" \
    "cargo run -p docker-agent" \
    "$YELLOW"

start_service "sentinel-bot" \
    "$ROOT_DIR" \
    "cargo run -p sentinel-bot" \
    "$BLUE"

start_service "nexus-bot" \
    "$ROOT_DIR" \
    "cargo run -p nexus-bot" \
    "$BLUE"

start_service "atrium-bot" \
    "$ROOT_DIR" \
    "cargo run -p atrium-bot" \
    "$BLUE"

if [ -d "$ROOT_DIR/web" ]; then
    if [ ! -d "$ROOT_DIR/web/node_modules" ]; then
        echo -e "${YELLOW}[INSTALL] Web - npm install...${NC}"
        (cd "$ROOT_DIR/web" && npm install) 2>&1
    fi
    start_service "web" \
        "$ROOT_DIR/web" \
        "npm run dev" \
        "$CYAN"
fi

echo ""
echo -e "${GREEN}================================================${NC}"
echo -e "${GREEN}   Tous les services sont lances !${NC}"
echo -e "${GREEN}================================================${NC}"
echo ""
echo -e "  API Backend : ${GREEN}http://localhost:3000${NC}"
echo -e "  Gateway WS  : ${GREEN}ws://localhost:3001${NC}"
echo -e "  Web Vite    : ${CYAN}http://localhost:5173${NC}"
echo ""
echo -e "  Services    : ${YELLOW}platform-scheduler  ops-agent  docker-agent  auth-api${NC}"
echo ""
echo -e "  Bots Discord: ${BLUE}sentinel-bot  nexus-bot  atrium-bot${NC}"
echo ""
echo -e "  Logs        : ${YELLOW}.logs/*.log${NC}"
echo ""
echo -e "${YELLOW}Ctrl+C pour tout arreter${NC}"
echo ""

# Attendre que tous les processus tournent
wait
