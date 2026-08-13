#!/bin/bash
# ============================================
# DiscordSentinel - TLS certificate issuer
# ============================================
# Obtient un cert TLS et le depose dans les volumes Docker partages avec
# le service `web` (nginx). Apres succes, reload nginx sans downtime (SIGHUP).
#
# Trois modes :
#   --letsencrypt          vrai cert Let's Encrypt (prod)
#   --staging              cert LE staging (tests, pas de rate-limit)
#   --self-signed          cert self-signed local (dev)
#
# Prerequis communs :
#   - Docker daemon accessible (`docker info` OK).
#   - Etre dans la racine du repo (ou laisser le script se debrouiller).
#
# Prerequis Let's Encrypt :
#   - Le service `web` doit etre up (il sert le challenge HTTP-01).
#   - Le domaine doit resoudre DNS vers cette machine.
#   - Port 80 ouvert depuis Internet (firewall, NAT, cloud provider).
#
# Exemples :
#   ./infrastructure/scripts/tls-issue.sh --letsencrypt --domain bot.exemple.com --email toi@exemple.com
#   ./infrastructure/scripts/tls-issue.sh --staging     --domain bot.exemple.com --email toi@exemple.com
#   ./infrastructure/scripts/tls-issue.sh --self-signed --domain localhost
# ============================================

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
info()  { echo -e "${BLUE}[tls]${NC} $*"; }
ok()    { echo -e "${GREEN}[tls]${NC} $*"; }
warn()  { echo -e "${YELLOW}[tls]${NC} $*"; }
err()   { echo -e "${RED}[tls]${NC} $*" >&2; }

# ---------- Parsing args ----------
MODE=""
DOMAIN=""
EMAIL=""
DAYS=30

usage() {
    cat <<EOF
Usage: $0 <mode> --domain <fqdn> [--email <email>] [--days N]

Modes (un seul requis) :
  --letsencrypt       Obtient un cert Let's Encrypt production
  --staging           Obtient un cert LE staging (pour tests, pas de rate-limit)
  --self-signed       Genere un cert self-signed (defaut: 30 jours)

Options :
  --domain <fqdn>     Domaine cible (requis)
  --email <addr>      Email de contact (requis pour Let's Encrypt)
  --days <n>          Duree de validite du self-signed (defaut: 30)
  -h, --help          Cette aide
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --letsencrypt)  MODE="letsencrypt"; shift ;;
        --staging)      MODE="staging";     shift ;;
        --self-signed)  MODE="selfsigned";  shift ;;
        --domain)       DOMAIN="${2:?}"; shift 2 ;;
        --email)        EMAIL="${2:?}";  shift 2 ;;
        --days)         DAYS="${2:?}";   shift 2 ;;
        -h|--help)      usage; exit 0 ;;
        *) err "Option inconnue : $1"; usage; exit 2 ;;
    esac
done

[[ -z "$MODE" ]]   && { err "Aucun mode specifie.";    usage; exit 2; }
[[ -z "$DOMAIN" ]] && { err "--domain est requis.";    exit 2; }
if [[ "$MODE" != "selfsigned" && -z "$EMAIL" ]]; then
    err "--email est requis en mode Let's Encrypt."
    exit 2
fi

# ---------- Checks prerequis ----------
if ! docker info >/dev/null 2>&1; then
    err "Docker daemon inaccessible. Demarre Docker Desktop / dockerd."
    exit 1
fi

# docker compose v2 (plugin) ou fallback docker-compose legacy.
if docker compose version >/dev/null 2>&1; then
    DC="docker compose"
elif command -v docker-compose >/dev/null 2>&1; then
    DC="docker-compose"
else
    err "Ni 'docker compose' ni 'docker-compose' disponibles."
    exit 1
fi
info "Utilisation de : $DC"

# ---------- Helper : reload nginx apres obtention ----------
reload_nginx() {
    local cid
    cid="$(docker ps -q -f label=com.docker.compose.service=web || true)"
    if [[ -z "$cid" ]]; then
        warn "Conteneur 'web' non trouve ; relance manuellement :"
        echo "    $DC up -d web"
        return
    fi
    info "Reload nginx (SIGHUP) dans le conteneur web..."
    # Un conteneur web peut avoir plusieurs replicas (scaling) : on les cible tous.
    echo "$cid" | xargs -r docker kill --signal=HUP >/dev/null
    ok "nginx reloaded, nouveau cert actif."
}

# ---------- Mode self-signed ----------
if [[ "$MODE" == "selfsigned" ]]; then
    info "Generation d'un cert self-signed pour ${DOMAIN} (valide ${DAYS} jours)..."
    # On passe par `compose run --rm` pour beneficier des volumes
    # declaratifs (letsencrypt_etc). L'image certbot embarque openssl nativement.
    $DC run --rm --no-deps --entrypoint /bin/sh certbot -c "
        set -e
        mkdir -p /etc/letsencrypt/live/${DOMAIN}
        openssl req -x509 -nodes -newkey rsa:2048 -days ${DAYS} \
            -keyout /etc/letsencrypt/live/${DOMAIN}/privkey.pem \
            -out    /etc/letsencrypt/live/${DOMAIN}/fullchain.pem \
            -subj '/CN=${DOMAIN}' \
            -addext 'subjectAltName=DNS:${DOMAIN},DNS:localhost,IP:127.0.0.1' \
            >/dev/null 2>&1
        chmod 600 /etc/letsencrypt/live/${DOMAIN}/privkey.pem
        echo '[ok] cert ecrit dans /etc/letsencrypt/live/${DOMAIN}/'
    "
    ok "Self-signed genere."
    # Si le service web tourne deja avec un ancien cert, on le reload.
    if docker ps -q -f label=com.docker.compose.service=web | grep -q .; then
        reload_nginx
    else
        info "Service web non demarre. Lance-le pour utiliser le nouveau cert :"
        echo "    $DC up -d web"
    fi
    exit 0
fi

# ---------- Modes Let's Encrypt ----------
WEB_CID="$(docker ps -q -f label=com.docker.compose.service=web || true)"
if [[ -z "$WEB_CID" ]]; then
    err "Le service 'web' doit etre demarre pour le challenge HTTP-01 de Let's Encrypt."
    echo "    $DC up -d web"
    exit 1
fi

STAGING_FLAG=""
if [[ "$MODE" == "staging" ]]; then
    STAGING_FLAG="--staging"
    warn "Mode STAGING : cert non trustable (pour tests uniquement)."
fi

info "Obtention d'un cert Let's Encrypt pour ${DOMAIN} (email: ${EMAIL})..."
info "Challenge : LE appelle http://${DOMAIN}/.well-known/acme-challenge/<token>"

# `run --rm --no-deps` : on n'active pas les depends_on (web deja up de toute facon).
# `--keep-until-expiring` : idempotent, ne reemet pas un cert encore valide.
if ! $DC run --rm --no-deps --entrypoint certbot certbot \
        certonly --webroot -w /var/www/certbot \
        --non-interactive --agree-tos \
        --email "${EMAIL}" \
        ${STAGING_FLAG} \
        --keep-until-expiring \
        -d "${DOMAIN}"; then
    err "Echec obtention du cert. Causes courantes :"
    echo "  - DNS ${DOMAIN} ne pointe pas sur cette machine."
    echo "  - Port 80 non ouvert depuis Internet (firewall, NAT, cloud)."
    echo "  - nginx ne sert pas /.well-known/acme-challenge/ (cf. nginx.conf)."
    echo "  - Rate-limit LE depasse (tester d'abord avec --staging)."
    exit 1
fi

ok "Cert obtenu avec succes."
reload_nginx

echo ""
ok "Acces : https://${DOMAIN}/"
if [[ "$MODE" == "staging" ]]; then
    warn "Rappel : cert STAGING non trustable. Relance avec --letsencrypt pour la prod."
fi
info "Renouvellement auto : lance le sidecar certbot => $DC up -d certbot"
