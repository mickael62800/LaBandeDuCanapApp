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

# ── Garde-fou : ne jamais ecrire sur la partition racine par accident ──────
#
# CE QUE L'ON EMPECHE. Si le disque de sauvegarde n'est pas monte, sa
# destination redevient un dossier ordinaire de la racine : le script y
# deverserait des gigaoctets sans rien signaler, jusqu'a saturer le systeme.
# C'est exactement ainsi que Docker a rempli la racine pendant deux mois, apres
# qu'une entree fstab devenue invalide eut fait disparaitre son disque.
#
# LE CONTROLE. La premiere version exigeait que la destination soit un POINT DE
# MONTAGE. C'etait trop rigide : un sous-repertoire choisi expres sur un autre
# disque — `/home/sauvegardes` quand /home est sur un second disque — se voyait
# refuse alors qu'il remplit parfaitement le but. On compare donc le SYSTEME DE
# FICHIERS de la destination a celui de la racine : different, c'est un autre
# disque, on accepte.
#
# `BACKUP_ALLOW_ROOTFS=1` leve le refus, pour le cas ou l'on veuille malgre tout
# sauvegarder sur la racine — depannage, machine a disque unique. Explicite, donc
# jamais subi.
# Numero de PERIPHERIQUE, pas le nom rendu par `df`. Celui-ci se decoupe sur
# les espaces, et une source reseau en contient volontiers —
# `//192.168.1.50/My Cloud` serait tronque a `//192.168.1.50/My`, faussant la
# comparaison sans que rien ne le signale.
fs_de() {
    stat -c %d "$1" 2>/dev/null
}

if [ ! -d "$DEST" ]; then
    erreur "$DEST n'existe pas"
    echo "       Cree-le, ou pointe BACKUP_DEST ailleurs." >&2
    exit 1
fi

if [ "$(fs_de "$DEST")" = "$(fs_de /)" ] && [ "${BACKUP_ALLOW_ROOTFS:-0}" != "1" ]; then
    erreur "$DEST est sur la partition racine — disque de sauvegarde absent ?"
    echo "       findmnt $DEST ; lsblk ; grep backup /etc/fstab" >&2
    echo "       Si c'est voulu : BACKUP_ALLOW_ROOTFS=1 $0" >&2
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

  # LECTURE SUR LE DESCRIPTEUR 3, ET NON SUR L'ENTREE STANDARD.
  #
  # `psql_nexus` appelle `docker exec -i` : le `-i` attache l'entree standard,
  # et psql la vide jusqu'au bout. L'INSERT dans `game_backups`, en fin
  # d'iteration, avalait donc les lignes restantes du here-string. Resultat :
  # le premier serveur etait sauvegarde, les suivants disparaissaient sans la
  # moindre erreur — pour bash, l'entree etait simplement terminee. Un serveur
  # Palworld actif est reste sans sauvegarde pendant que le journal affichait
  # « sauvegarde terminee ».
  #
  # Le descripteur 3 n'est attache a aucune commande fille : ce que la boucle
  # lit reste a la boucle, quoi que fasse son corps.
  while IFS='|' read -r id nom statut volume <&3; do
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
  done 3<<< "$lignes"
}

# ── Retention ──────────────────────────────────────────────────────────────

# Toutes les archives de mondes presentes, quel que soit leur suffixe.
#
# `*.tar` ET `*.tar.*` : les archives prises A FROID par le redemarrage
# programme ne sont pas compressees (les sauvegardes de jeu le sont deja), donc
# elles n'ont pas de second suffixe. Le motif `*.tar.*` seul les ignorait — 5 Go
# par jour qui s'accumulaient sans jamais etre purges, pendant que leurs lignes
# en base disparaissaient a 14 jours.
toutes_les_archives() {
    find "$DEST/mondes" -type f \( -name '*.tar' -o -name '*.tar.*' \) "$@" 2>/dev/null
}

# La plus recente archive de CHAQUE serveur, a conserver quoi qu'il arrive.
#
# Sans cette protection, un serveur qui ne redemarre pas pendant quinze jours
# verrait sa seule copie supprimee par l'age : on se retrouverait avec zero
# sauvegarde pour le serveur le plus tranquille, ce qui est exactement l'inverse
# du but.
archives_a_conserver() {
    toutes_les_archives -printf '%T@ %p
'         | sort -rn         | awk '{
              chemin = $2
              for (i = 3; i <= NF; i++) chemin = chemin " " $i
              serveur = chemin
              sub(/-[0-9]{8}-[0-9]{6}(-a-chaud)?\.tar(\.[a-z0-9]+)?$/, "", serveur)
              if (!(serveur in vu)) { vu[serveur] = 1; print chemin }
          }'
}

purger() {
    if $DRY_RUN; then
        log "[simulation] purge au-dela de ${RETENTION_JOURS} j"
        return
    fi

    # Dumps de bases : purge simple par age. Ils pesent quelques centaines de
    # kilo-octets, en perdre un vieux n'a aucune consequence.
    local dumps
    dumps=$(find "$DEST/postgres" -type f -name '*.dump'         -mtime "+$RETENTION_JOURS" -print -delete 2>/dev/null | wc -l)

    # Mondes : purge par age, sauf la plus recente de chaque serveur.
    local proteges supprimes=0 fichier
    proteges=$(archives_a_conserver)
    while IFS= read -r fichier <&3; do
        [ -z "$fichier" ] && continue
        if printf '%s
' "$proteges" | grep -qxF "$fichier"; then
            log "  conserve $(basename "$fichier") — seule archive de ce serveur"
            continue
        fi
        rm -f "$fichier" && supprimes=$((supprimes + 1))
    done 3<<< "$(toutes_les_archives -mtime "+$RETENTION_JOURS")"

    local total=$((dumps + supprimes))
    [ "$total" -gt 0 ] && log "purge : $total archive(s) supprimee(s)"

    # Les lignes de `game_backups` suivent les fichiers. Meme protection : on ne
    # supprime une ligne que s'il en existe une PLUS RECENTE pour ce serveur,
    # sinon la table cesserait de designer la sauvegarde qu'on vient justement
    # de conserver.
    psql_nexus -q -c "DELETE FROM game_backups gb WHERE gb.backup_type = 'auto' AND gb.created_at < NOW() - INTERVAL '$RETENTION_JOURS days' AND EXISTS (SELECT 1 FROM game_backups plus_recent WHERE plus_recent.server_id = gb.server_id AND plus_recent.created_at > gb.created_at)"         || erreur "purge des lignes game_backups impossible"
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
