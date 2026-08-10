#!/bin/sh
# Genere le snippet nginx qui injecte le jeton d'API Exploitation cote serveur.
# Meme mecanique que 30-nexus-key.sh et 31-atrium-key.sh.

set -eu

SNIPPET_DIR="/etc/nginx/snippets"
SNIPPET="${SNIPPET_DIR}/ops-auth.inc"

mkdir -p "${SNIPPET_DIR}"

if [ -n "${OPS_API_TOKEN:-}" ]; then
    printf 'proxy_set_header Authorization "Bearer %s";\n' "${OPS_API_TOKEN}" > "${SNIPPET}"
    chmod 600 "${SNIPPET}"
    echo "[ops-key] Jeton Exploitation injecte dans le proxy /ops-api/"
else
    # Fichier vide : l'include reste valide, ops-api repondra 401.
    : > "${SNIPPET}"
    echo "[ops-key] WARNING: OPS_API_TOKEN absent — /ops-api/ repondra 401"
fi