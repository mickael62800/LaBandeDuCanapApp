#!/bin/sh
# Genere le snippet nginx qui injecte le jeton d'API Atrium cote serveur.
#
# Meme mecanique que 30-nexus-key.sh, et pour la meme raison : nginx.conf est
# copie tel quel dans l'image (pas de template envsubst), et on ne commite pas
# un secret. Le snippet est donc ecrit au demarrage du conteneur a partir de la
# variable d'environnement.
#
# Place dans /docker-entrypoint.d/ : l'image nginx:alpine execute ces scripts
# par ordre alphabetique avant de lancer nginx.

set -eu

SNIPPET_DIR="/etc/nginx/snippets"
SNIPPET="${SNIPPET_DIR}/atrium-auth.inc"

mkdir -p "${SNIPPET_DIR}"

if [ -n "${ATRIUM_API_TOKEN:-}" ]; then
    printf 'proxy_set_header Authorization "Bearer %s";\n' "${ATRIUM_API_TOKEN}" > "${SNIPPET}"
    chmod 600 "${SNIPPET}"
    echo "[atrium-key] Jeton Atrium injecte dans le proxy /atrium-api/"
else
    # Fichier vide : la directive `include` de nginx.conf reste valide, la
    # requete part sans Authorization et atrium-api repond 401. Sans ce
    # fichier, nginx refuserait de demarrer (include introuvable).
    : > "${SNIPPET}"
    echo "[atrium-key] WARNING: ATRIUM_API_TOKEN absent — /atrium-api/ repondra 401"
fi
