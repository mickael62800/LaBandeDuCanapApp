#!/bin/bash
# ============================================
# DiscordSentinel - Lancement sequentiel
# Demarre les conteneurs un par un dans l'ordre.
# Ne lance que les images deja buildees.
#
# Usage:
#   bash start-all.sh
# ============================================

set -o pipefail

cd "$(dirname "$0")/../.."
export COMPOSE_FILE=infrastructure/docker/docker-compose.yml

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

FAILED=()
SUCCESS=()
SKIPPED=()
TOTAL=0
CURRENT=0

# Verification que Docker tourne
if ! docker info &>/dev/null; then
  echo -e "${RED}Docker n'est pas lance. Demarre Docker Desktop et reessaie.${NC}"
  exit 1
fi

# ── Ordre de lancement ──
# 1. Infra (postgres, redis)
# 2. API
# 3. AI API (si profile ai actif)
# 4. Gateway
# 5. Workers
# 6. Bots

INFRA=(postgres redis)
BACKEND=(api)
GATEWAY=(gateway)
WORKERS=(moderation-worker analytics-worker monitoring-worker cache-worker cleanup-worker ai-worker appeal-sla-worker audit-cache-worker discord-audit-sync-worker export-worker temp-roles-worker)
BOTS=(sentinel-bot)

ALL_SERVICES=("${INFRA[@]}" "${BACKEND[@]}" "${GATEWAY[@]}" "${WORKERS[@]}" "${BOTS[@]}")
TOTAL=${#ALL_SERVICES[@]}

start_service() {
  local svc="$1"
  CURRENT=$((CURRENT + 1))

  echo -e "${CYAN}  [$CURRENT/$TOTAL] $svc${NC}"

  # Verifier si l'image existe (sauf pour les images officielles: postgres, redis)
  if [[ "$svc" != "postgres" && "$svc" != "redis" ]]; then
    local image
    image=$(docker compose config --images 2>/dev/null | grep "$svc" || true)
    if [ -z "$image" ]; then
      echo -e "${YELLOW}    [SKIP] Image non trouvee — lance 'docker compose build' d'abord${NC}"
      SKIPPED+=("$svc")
      return
    fi
  fi

  if docker compose up -d "$svc" 2>&1 | grep -v "level=warning"; then
    echo -e "${GREEN}    [OK]${NC}"
    SUCCESS+=("$svc")
  else
    echo -e "${RED}    [FAIL]${NC}"
    FAILED+=("$svc")
  fi
}

wait_healthy() {
  local svc="$1"
  local max_wait="$2"
  local elapsed=0

  echo -e "${YELLOW}  Attente que $svc soit healthy...${NC}"
  while [ $elapsed -lt $max_wait ]; do
    local status
    status=$(docker compose ps "$svc" --format '{{.Status}}' 2>/dev/null)
    if echo "$status" | grep -q "healthy"; then
      echo -e "${GREEN}  $svc est pret.${NC}"
      return 0
    fi
    sleep 2
    elapsed=$((elapsed + 2))
  done
  echo -e "${RED}  $svc n'est pas healthy apres ${max_wait}s${NC}"
  return 1
}

# Demarrage
echo ""
echo -e "${CYAN}================================================${NC}"
echo -e "${CYAN}  DiscordSentinel - Lancement sequentiel${NC}"
echo -e "${CYAN}================================================${NC}"

# 1. Infra
echo ""
echo -e "${CYAN}── Infrastructure ──${NC}"
for svc in "${INFRA[@]}"; do
  start_service "$svc"
done
wait_healthy "postgres" 30
wait_healthy "redis" 30

# 2. API
echo ""
echo -e "${CYAN}── API ──${NC}"
for svc in "${BACKEND[@]}"; do
  start_service "$svc"
done
echo -e "${YELLOW}  Attente 20s que l'API demarre...${NC}"
sleep 20

# 3. Gateway
echo ""
echo -e "${CYAN}── Gateway ──${NC}"
for svc in "${GATEWAY[@]}"; do
  start_service "$svc"
done

# 4. Workers
echo ""
echo -e "${CYAN}── Workers ──${NC}"
for svc in "${WORKERS[@]}"; do
  start_service "$svc"
done

# 5. Bots
echo ""
echo -e "${CYAN}── Bots ──${NC}"
for svc in "${BOTS[@]}"; do
  start_service "$svc"
done

# Resume
echo ""
echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}  RESUME${NC}"
echo -e "${CYAN}========================================${NC}"
echo -e "${GREEN}OK (${#SUCCESS[@]}):${NC} ${SUCCESS[*]}"
[ ${#SKIPPED[@]} -gt 0 ] && echo -e "${YELLOW}SKIP (${#SKIPPED[@]}):${NC} ${SKIPPED[*]}"
if [ ${#FAILED[@]} -gt 0 ]; then
  echo -e "${RED}ECHEC (${#FAILED[@]}):${NC} ${FAILED[*]}"
  exit 1
else
  echo ""
  echo -e "${GREEN}Tous les services sont lances !${NC}"
  echo ""
  docker compose ps --format "table {{.Name}}\t{{.Status}}"
fi
