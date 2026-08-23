#!/usr/bin/env bash
# ============================================================================
# DiscordSentinel — Sauvegarde des bases et des mondes de jeu
#
# Sauvegarde, vers un disque dedie :
#   - les quatre bases logiques (discord_sentinel, nexus, atrium, auth) ;
#   - le volume Docker de CHAQUE serveur de jeu, c'est-a-dire son monde ;
#   - et inscrit chaque monde sauvegarde dans `game_backups`, table qui
#     existait depuis l'origine sans que rien ne l'alimente.
#
# Usage :
#   sudo bash backup.sh                  # bases + mondes
#   sudo bash backup.sh --db-only        # bases seules (rapide)
#   sudo bash backup.sh --worlds-only    # mondes seuls
#   sudo bash backup.sh --dry-run        # montre ce qui serait fait
#
# Cron (3 h du matin, hors des heures de jeu) :
#   0 3 * * * root /usr/local/bin/sentinel-backup.sh >> /var/log/sentinel-backup.log 2>&1
# ============================================================================

set -euo pipefail

DEST="${BACKUP_DEST:-/mnt/backup}"
RETENTION_JOURS="${BACKUP_RETENTION_DAYS:-14}"
PG_CONTAINER="${PG_CONTAINER:-sentinel-postgres}"
PG_USER="${PG_USER:-sentinel}"
BASES=(discord_sentinel nexus atrium auth)
# Surchargeable pour pouvoir eprouver le script hors production.
LOCK="${BACKUP_LOCK:-/var/run/sentinel-backup.lock}"

DB_ONLY=false
WORLDS_ONLY=false
DRY_RUN=false
for arg in "$@"; do
  case "$arg" in
    --db-only)     DB_ONLY=true ;;
    --worlds-only) WORLDS_ONLY=true ;;
    --dry-run)     DRY_RUN=true ;;
    *) echo "Option inconnue : $arg" >&2; exit 2 ;;
  esac
done

HORODATAGE=$(date +%Y%m%d-%H%M%S)
ERREURS=0
log()    { echo "[$(date +%H:%M:%S)] $*"; }
erreur() { echo "[$(date +%H:%M:%S)] ERREUR : $*" >&2; ERREURS=$((ERREURS + 1)); }

# ── Garde-fou : la destination doit etre un POINT DE MONTAGE ───────────────
#
# LA verification qui compte. Si le disque n'est pas monte, /mnt/backup est un
# simple dossier de la partition racine : le script y ecrirait des dizaines de
# gigaoctets sans rien signaler, jusqu'a saturer le systeme. C'est exactement
# ainsi que Docker a rempli la racine pendant deux mois, apres qu'une entree
# fstab devenue invalide eut fait disparaitre son disque en silence.
#
# On refuse donc de travailler tant que `findmnt` ne confirme pas un montage.
if ! findmnt -M "$DEST" >/dev/null 2>&1; then
  erreur "$DEST n'est pas un point de montage — disque absent ?"
  echo "       Verifie : findmnt $DEST  &&  grep backup /etc/fstab" >&2
  exit 1
fi

# ── Verrou : deux sauvegardes simultanees se marcheraient dessus ───────────
exec 9>"$LOCK"
if ! flock -n 9; then
  log "une sauvegarde est deja en cours — abandon"
  exit 0
fi

# ── Place disponible ───────────────────────────────────────────────────────
LIBRE_GO=$(df -BG --output=avail "$DEST" | tail -1 | tr -dc '0-9')
if [ "${LIBRE_GO:-0}" -lt 10 ]; then
  erreur "moins de 10 Go libres sur $DEST (${LIBRE_GO} Go) — sauvegarde annulee"
  exit 1
fi
log "destination $DEST — ${LIBRE_GO} Go libres, retention ${RETENTION_JOURS} j"

# zstd compresse deux a trois fois plus vite que gzip, a taux egal ou meilleur.
# Sur un disque a plateau, gzip devient le goulot avant le disque lui-meme.
if command -v zstd >/dev/null 2>&1; then
  COMPRESSEUR=(zstd -3 -T0)
  EXT="tar.zst"
else
  COMPRESSEUR=(gzip -1)
  EXT="tar.gz"
  log "zstd absent, repli sur gzip (apt install zstd pour accelerer)"
fi

psql_nexus() {
  docker exec -i "$PG_CONTAINER" psql -U "$PG_USER" -d nexus -v ON_ERROR_STOP=1 "$@"
}

# ── Bases PostgreSQL ───────────────────────────────────────────────────────
sauvegarder_bases() {
  mkdir -p "$DEST/postgres"
  local base sortie
  for base in "${BASES[@]}"; do
    sortie="$DEST/postgres/${base}-${HORODATAGE}.dump"
    if $DRY_RUN; then
      log "[simulation] pg_dump $base -> $sortie"
      continue
    fi
    log "pg_dump $base…"
    # Format `custom` (-Fc) : compresse, et surtout restaurable table par table
    # avec pg_restore. Un dump SQL brut impose de tout rejouer.
    if docker exec -i "$PG_CONTAINER" pg_dump -U "$PG_USER" -Fc -d "$base" > "$sortie.tmp"; then
      mv "$sortie.tmp" "$sortie"
      # Ces dumps contiennent les tokens Discord des administrateurs : c'est la
      # donnee la plus sensible de la plateforme.
      chmod 600 "$sortie"
      log "  $base : $(du -h "$sortie" | cut -f1)"
    else
      rm -f "$sortie.tmp"
      erreur "pg_dump $base a echoue"
    fi
  done
}

# ── Mondes de jeu ──────────────────────────────────────────────────────────
sauvegarder_mondes() {
  mkdir -p "$DEST/mondes"

  # `volume_name` est nul pour un serveur jamais demarre : il n'a pas encore de
  # monde a sauvegarder.
  local lignes
  if ! lignes=$(psql_nexus -tAF'|' -c "SELECT id, name, status, volume_name FROM game_servers WHERE deleted_at IS NULL AND volume_name IS NOT NULL"); then
    erreur "lecture de game_servers impossible"
    return
  fi

  if [ -z "${lignes//[[:space:]]/}" ]; then
    log "aucun serveur de jeu a sauvegarder"
    return
  fi

  local id nom statut volume point suffixe sortie taille nom_fichier
  while IFS='|' read -r id nom statut volume; do
    [ -z "$id" ] && continue

    # La base autorise les espaces dans le nom d'un serveur
    # (`chk_game_servers_name`). Les laisser tels quels donnerait des archives
    # du genre « minecraft test-2026….tar.zst » : penibles a manipuler en
    # ligne de commande, et cassantes pour tout script de restauration qui
    # oublierait une paire de guillemets.
    nom_fichier=$(printf '%s' "$nom" | tr -c 'A-Za-z0-9._-' '_')

    if ! point=$(docker volume inspect -f '{{.Mountpoint}}' "$volume" 2>/dev/null); then
      erreur "volume $volume introuvable (serveur $nom)"
      continue
    fi

    # Un monde copie pendant que le serveur tourne peut attraper un fichier a
    # moitie ecrit. On sauvegarde quand meme — une copie imparfaite vaut mieux
    # qu'aucune copie — mais le nom du fichier le dit, pour qu'on ne decouvre
    # pas le probleme au moment de restaurer.
    suffixe=""
    if [ "$statut" = "running" ]; then
      suffixe="-a-chaud"
      log "  ⚠ $nom tourne : copie a chaud, coherence non garantie"
    fi

    sortie="$DEST/mondes/${nom_fichier}-${HORODATAGE}${suffixe}.${EXT}"
    if $DRY_RUN; then
      log "[simulation] $volume -> $sortie"
      continue
    fi

    log "monde $nom ($volume)…"
    if tar -cf - -C "$point" . | "${COMPRESSEUR[@]}" > "$sortie.tmp"; then
      mv "$sortie.tmp" "$sortie"
      taille=$(stat -c %s "$sortie")
      log "  $nom : $(du -h "$sortie" | cut -f1)"

      # Trace en base : c'est ce qui donne enfin un contenu a `game_backups`,
      # et ce qui permettra a l'interface de lister les sauvegardes existantes.
      if ! psql_nexus -q -c "INSERT INTO game_backups (server_id, file_path, size_bytes, backup_type) VALUES ('$id', '$sortie', $taille, 'auto')"; then
        erreur "monde $nom sauvegarde, mais non enregistre en base"
      fi
    else
      rm -f "$sortie.tmp"
      erreur "archivage du monde $nom a echoue"
    fi
  done <<< "$lignes"
}

# ── Retention ──────────────────────────────────────────────────────────────
purger() {
  if $DRY_RUN; then
    log "[simulation] purge au-dela de ${RETENTION_JOURS} j"
    return
  fi

  local supprimes
  supprimes=$(find "$DEST/postgres" "$DEST/mondes" -type f \
    \( -name '*.dump' -o -name '*.tar.*' \) \
    -mtime "+$RETENTION_JOURS" -print -delete 2>/dev/null | wc -l)
  [ "$supprimes" -gt 0 ] && log "purge : $supprimes archive(s) supprimee(s)"

  # Les lignes de `game_backups` doivent suivre les fichiers : sans cela, la
  # table designerait des archives effacees, et l'interface proposerait de
  # restaurer ce qui n'existe plus.
  psql_nexus -q -c "DELETE FROM game_backups WHERE backup_type = 'auto' AND created_at < NOW() - INTERVAL '$RETENTION_JOURS days'" \
    || erreur "purge des lignes game_backups impossible"
}

# ── Deroulement ────────────────────────────────────────────────────────────
$WORLDS_ONLY || sauvegarder_bases
$DB_ONLY     || sauvegarder_mondes
purger

log "espace restant : $(df -h "$DEST" | tail -1 | awk '{print $4}')"

if [ "$ERREURS" -gt 0 ]; then
  # Sortie non nulle : cron enverra un courriel, et la supervision verra le job
  # en echec. Une sauvegarde qui echoue en silence est pire que pas de
  # sauvegarde du tout — on se croit protege.
  log "TERMINE AVEC $ERREURS ERREUR(S)"
  exit 1
fi
log "sauvegarde terminee"
