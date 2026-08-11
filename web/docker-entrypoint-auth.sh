#!/bin/sh
# Genere le snippet nginx qui injecte le jeton de l'API d'identite cote serveur.
# Meme mecanique que 30-nexus-key.sh, 31-atrium-key.sh et 32-ops-key.sh.
#
# Ce jeton-ci est le plus critique des quatre : il ouvre /access, la sonde dont
# depend l'autorisation des TROIS passerelles. Sans lui, auth-api repond 401 et
# nginx refuse tout — fail-closed, ce qui est le bon sens de l'echec.

set -eu

SNIPPET_DIR="/etc/nginx/snippets"
SNIPPET="${SNIPPET_DIR}/auth-key.inc"

mkdir -p "${SNIPPET_DIR}"

if [ -n "${AUTH_API_TOKEN:-}" ]; then
    printf 'proxy_set_header Authorization "Bearer %s";\n' "${AUTH_API_TOKEN}" > "${SNIPPET}"
    chmod 600 "${SNIPPET}"
    echo "[auth-key] Jeton d'identite injecte dans les sous-requetes d'autorisation"
else
    # Fichier vide : l'include reste valide, auth-api repondra 401.
    : > "${SNIPPET}"
    echo "[auth-key] WARNING: AUTH_API_TOKEN absent — toutes les passerelles repondront 401"
fi
