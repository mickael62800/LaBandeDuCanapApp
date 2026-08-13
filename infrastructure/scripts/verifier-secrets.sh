#!/bin/sh
# Verifie que le .env declare tous les secrets exiges par les composes.
#
# A lancer AVANT `docker compose up` depuis infrastructure/docker/ :
#
#     sh ../scripts/verifier-secrets.sh .env
#
# Pourquoi ce script existe : les mots de passe n'ont plus de valeur de repli
# publiee dans le depot (`:?` et non `:-`). C'est voulu — un deploiement dont
# le .env est incomplet demarrait sinon, sans avertissement, avec un mot de
# passe que tout lecteur du depot connait. La contrepartie est que `docker
# compose up` s'arrete desormais sur la premiere variable manquante, une par
# une. Ce script les liste TOUTES d'un coup.
#
# Il ne lit aucune valeur et n'en affiche aucune : seulement present / absent.

ENV_FILE="${1:-.env}"

if [ ! -f "$ENV_FILE" ]; then
  echo "Fichier introuvable : $ENV_FILE" >&2
  exit 2
fi

# Secrets exiges par les composes (`:?`). Tenir cette liste a jour en meme
# temps que les fichiers compose.
REQUIS="
POSTGRES_PASSWORD
REDIS_PASSWORD
SENTINEL_DB_PASSWORD
OPS_DB_PASSWORD
AUTH_DB_PASSWORD
AUTH_REDIS_PASSWORD
NEXUS_DB_PASSWORD
NEXUS_REDIS_PASSWORD
ATRIUM_DB_PASSWORD
PGADMIN_PASSWORD
GRAFANA_PASSWORD
DOCKER_AGENT_TOKEN
DOCKER_AGENT_GAME_TOKEN
OPS_API_TOKEN
AUTH_API_TOKEN
ATRIUM_API_TOKEN
ATRIUM_GRPC_TOKEN
NEXUS_API_KEY
SENTINEL_API_KEY
"

manquants=0
vides=0

for v in $REQUIS; do
  ligne=$(grep -E "^${v}=" "$ENV_FILE" | tail -n 1)
  if [ -z "$ligne" ]; then
    echo "ABSENT  $v"
    manquants=$((manquants + 1))
  elif [ "$ligne" = "${v}=" ]; then
    # Une variable declaree vide passe la presence mais pas `:?` : compose la
    # traite comme manquante. Le distinguer evite de chercher au mauvais endroit.
    echo "VIDE    $v"
    vides=$((vides + 1))
  else
    echo "ok      $v"
  fi
done

# DEEPSEEK_API_KEY accepte deux noms : la surcharge par plateforme suffit.
if grep -qE "^(ATRIUM_)?DEEPSEEK_API_KEY=." "$ENV_FILE"; then
  echo "ok      DEEPSEEK_API_KEY (ou ATRIUM_DEEPSEEK_API_KEY)"
else
  echo "ABSENT  DEEPSEEK_API_KEY (ou ATRIUM_DEEPSEEK_API_KEY)"
  manquants=$((manquants + 1))
fi

# NEXUS_API_KEY doit faire 16 caracteres minimum : nexus-api s'arrete en
# dessous (cf. bootstrap). Le verifier ici evite un conteneur qui redemarre
# en boucle sans que la cause soit lisible.
cle_nexus=$(grep -E "^NEXUS_API_KEY=" "$ENV_FILE" | tail -n 1 | cut -d= -f2-)
if [ -n "$cle_nexus" ] && [ "${#cle_nexus}" -lt 16 ]; then
  echo "COURTE  NEXUS_API_KEY (16 caracteres minimum)"
  vides=$((vides + 1))
fi

echo
if [ "$manquants" -eq 0 ] && [ "$vides" -eq 0 ]; then
  # Pas de backticks ici : dans une chaine entre guillemets doubles, sh les
  # traite comme une substitution de commande — la version precedente lancait
  # reellement `docker compose up` en annoncant que tout allait bien.
  echo 'Tout est declare. "docker compose up" ne s.arretera pas sur un secret.'
  exit 0
fi

echo "$manquants absente(s), $vides vide(s) ou trop courte(s)."
echo
echo "Generer une valeur :"
echo "  echo \"NOM_DE_LA_VARIABLE=\$(openssl rand -base64 32 | tr -d '/+=' | head -c 32)\" >> $ENV_FILE"
echo
echo "ATTENTION — si un service tourne DEJA avec l'ancienne valeur par defaut,"
echo "l'ajouter au .env ne suffit pas : c'est une rotation de secret."
echo "  - Postgres n'applique POSTGRES_PASSWORD qu'a l'initialisation du volume."
echo "    Sur un cluster existant : ALTER ROLE <role> WITH PASSWORD '<nouveau>'."
echo "  - Redis lit --requirepass au demarrage du conteneur : tous les clients"
echo "    doivent redemarrer avec la nouvelle URL."
exit 1
