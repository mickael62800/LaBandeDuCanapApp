#!/bin/bash
# ============================================================================
# Setup helpers HOST pour la page Securite serveur de DiscordSentinel.
#
# Pattern : chaque module installe un script + cron qui ecrit un fichier
# JSON dans /var/lib/sentinel/<feature>.json. L'API DiscordSentinel
# (conteneur) lit ces fichiers en read-only via volume mount.
#
# Usage :
#   sudo bash infrastructure/scripts/setup-host-security.sh fail2ban
#   sudo bash infrastructure/scripts/setup-host-security.sh all
#
# Modules :
#   fail2ban      Installation fail2ban + jail SSH + export status
#   ban-apply     Cron qui applique les bans/unbans IPs ecrits par l'API
#   ssh-failures  Cron parse journalctl SSH -> ssh-failures.json
#   disk-trend    Cron snapshot df -> disk-trend.json (historique 7j)
#   connections   Cron snapshot ss -tn -> connections.json
#   open-ports    Cron nmap localhost -> open-ports.json (avec whitelist)
#   trivy         Scan vulns Docker images -> trivy.json (manuel ou cron)
#   all           Tous les modules ci-dessus
# ============================================================================

set -euo pipefail

SENTINEL_DATA_DIR="/var/lib/sentinel"
SCRIPT_DIR="/usr/local/bin"

require_root() {
    if [ "$EUID" -ne 0 ]; then
        echo "❌ Ce script doit être lancé en root (sudo)."
        exit 1
    fi
}

ensure_data_dir() {
    mkdir -p "$SENTINEL_DATA_DIR"
    # UID 1000 = user `sentinel` dans le conteneur api (cf. Dockerfile.rust-debian).
    # L'API ecrit bans-pending.txt / unbans-pending.txt depuis /api/security/ban-ip.
    # Sans cet ownership, le handler retourne 500 (permission denied).
    chown 1000:1000 "$SENTINEL_DATA_DIR"
    chmod 755 "$SENTINEL_DATA_DIR"
}

apt_install() {
    if ! command -v "$1" &>/dev/null; then
        echo "→ Installation $1…"
        apt-get update -qq
        apt-get install -y "$1"
    fi
}

# ── Module fail2ban ─────────────────────────────────────────────────────

setup_fail2ban() {
    echo "🛡  fail2ban"
    apt_install fail2ban

    if [ ! -f /etc/fail2ban/jail.local ]; then
        cat > /etc/fail2ban/jail.local <<'EOF'
[DEFAULT]
bantime  = 1h
findtime = 10m
maxretry = 5
ignoreip = 127.0.0.1/8 ::1 192.168.0.0/16 10.0.0.0/8 172.16.0.0/12

[sshd]
enabled = true
port    = ssh
backend = systemd
maxretry = 3
bantime  = 1h
EOF
        systemctl restart fail2ban
    fi
    systemctl enable --now fail2ban

    # Script export
    cat > "$SCRIPT_DIR/fail2ban-export.sh" <<'EOF'
#!/bin/bash
set -eu
OUT=/var/lib/sentinel/fail2ban-status.json
mkdir -p /var/lib/sentinel && chmod 755 /var/lib/sentinel
JAILS=$(fail2ban-client status 2>/dev/null | grep "Jail list:" | sed 's/.*://;s/,/ /g')
{
    echo "{"
    echo "  \"updated_at\": \"$(date -Iseconds)\","
    echo "  \"jails\": ["
    F=1
    for J in $JAILS; do
        J=$(echo "$J" | xargs); [ -z "$J" ] && continue
        B=$(fail2ban-client status "$J" 2>/dev/null | grep "Banned IP list:" | sed 's/.*://' | xargs || true)
        T=$(fail2ban-client status "$J" 2>/dev/null | grep "Total banned:" | sed 's/.*://' | xargs || true)
        [ $F -eq 0 ] && echo "    ,"
        echo "    {\"name\": \"$J\", \"total_banned\": ${T:-0}, \"banned_ips\": \"${B:-}\"}"
        F=0
    done
    echo "  ]"
    echo "}"
} > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"
chmod 644 "$OUT"
EOF
    chmod +x "$SCRIPT_DIR/fail2ban-export.sh"
    "$SCRIPT_DIR/fail2ban-export.sh"
    echo "*/2 * * * * root /usr/local/bin/fail2ban-export.sh" > /etc/cron.d/fail2ban-export
    echo "  ✅ fail2ban configure"
}

# ── Module ban-apply ────────────────────────────────────────────────────

setup_ban_apply() {
    echo "🚫 ban-apply"
    apt_install ufw

    cat > "$SCRIPT_DIR/sentinel-apply-bans.sh" <<'EOF'
#!/bin/bash
set -eu
DIR=/var/lib/sentinel
BANS=$DIR/bans-pending.txt
UNBANS=$DIR/unbans-pending.txt
LOG=$DIR/bans-applied.log
mkdir -p $DIR
touch $BANS $UNBANS $LOG
if [ -s "$BANS" ]; then
    while IFS=$'\t' read -r IP TS REASON; do
        [ -z "$IP" ] && continue
        ufw deny from "$IP" 2>/dev/null && \
            echo "$(date -Iseconds) BAN $IP reason=$REASON" >> $LOG || \
            echo "$(date -Iseconds) BAN_FAIL $IP" >> $LOG
    done < "$BANS"
    : > "$BANS"
fi
if [ -s "$UNBANS" ]; then
    while IFS=$'\t' read -r IP TS REASON; do
        [ -z "$IP" ] && continue
        ufw delete deny from "$IP" 2>/dev/null && \
            echo "$(date -Iseconds) UNBAN $IP reason=$REASON" >> $LOG || \
            echo "$(date -Iseconds) UNBAN_FAIL $IP" >> $LOG
    done < "$UNBANS"
    : > "$UNBANS"
fi
EOF
    chmod +x "$SCRIPT_DIR/sentinel-apply-bans.sh"
    echo "* * * * * root /usr/local/bin/sentinel-apply-bans.sh" > /etc/cron.d/sentinel-apply-bans
    echo "  ✅ ban-apply configure"
}

# ── Module ssh-failures ─────────────────────────────────────────────────

setup_ssh_failures() {
    echo "🔑 ssh-failures"

    cat > "$SCRIPT_DIR/sentinel-ssh-failures.sh" <<'EOF'
#!/bin/bash
set -eu
OUT=/var/lib/sentinel/ssh-failures.json
mkdir -p /var/lib/sentinel
# Parse les 24 dernieres heures de journal SSH
ENTRIES=$(journalctl _SYSTEMD_UNIT=ssh.service --since "24 hours ago" 2>/dev/null | \
    grep -E "Failed password|Invalid user|authentication failure" | tail -200 || true)
TOTAL=$(echo "$ENTRIES" | wc -l)
[ -z "$ENTRIES" ] && TOTAL=0

{
    echo "{"
    echo "  \"updated_at\": \"$(date -Iseconds)\","
    echo "  \"total_24h\": $TOTAL,"
    echo "  \"entries\": ["
    F=1
    while IFS= read -r LINE; do
        [ -z "$LINE" ] && continue
        TS=$(echo "$LINE" | awk '{print $1, $2, $3}')
        # Extract user + IP
        USER=$(echo "$LINE" | grep -oE "user [a-zA-Z0-9_-]+" | head -1 | awk '{print $2}' || echo "?")
        IP=$(echo "$LINE" | grep -oE "from [0-9a-f.:]+" | head -1 | awk '{print $2}' || echo "?")
        MSG=$(echo "$LINE" | sed 's/"/\\"/g' | cut -c1-200)
        [ $F -eq 0 ] && echo "    ,"
        echo "    {\"timestamp\": \"$TS\", \"user\": \"${USER:-?}\", \"ip\": \"${IP:-?}\", \"message\": \"$MSG\"}"
        F=0
    done <<< "$ENTRIES"
    echo "  ]"
    echo "}"
} > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"
chmod 644 "$OUT"
EOF
    chmod +x "$SCRIPT_DIR/sentinel-ssh-failures.sh"
    "$SCRIPT_DIR/sentinel-ssh-failures.sh"
    echo "*/5 * * * * root /usr/local/bin/sentinel-ssh-failures.sh" > /etc/cron.d/sentinel-ssh-failures
    echo "  ✅ ssh-failures configure"
}

# ── Module disk-trend ───────────────────────────────────────────────────

setup_disk_trend() {
    echo "💾 disk-trend"

    cat > "$SCRIPT_DIR/sentinel-disk-trend.sh" <<'EOF'
#!/bin/bash
set -eu
OUT=/var/lib/sentinel/disk-trend.json
CURRENT=/var/lib/sentinel/disks-current.json
HISTORY=/var/lib/sentinel/.disk-history.json
mkdir -p /var/lib/sentinel

# Snapshot actuel : liste tous les filesystems locaux REELS (exclut tmpfs,
# devtmpfs, overlay, squashfs, fuse.snapfuse). On garde uniquement les
# devices /dev/* pour ignorer les pseudo-FS. Si plusieurs disques physiques
# sont montes (ex: / et /mnt/docker), ils apparaissent tous.
#
# UN DISQUE = UNE LIGNE. `df` liste aussi les montages LIES, qui exposent le
# meme device sous un second point de montage : /mnt/docker et son bind
# /var/lib/containerd sont le meme /dev/sda1. Sans deduplication, le tableau
# de bord comptait ce disque deux fois et doublait l espace total annonce.
# On ne garde donc que la PREMIERE occurrence de chaque device.
NOW=$(date -Iseconds)
SNAPSHOT=$(df -BG --output=source,target,size,used,pcent \
        -x tmpfs -x devtmpfs -x overlay -x squashfs -x fuse.snapfuse \
        -x proc -x sysfs -x cgroup -x cgroup2 -x autofs \
        2>/dev/null | tail -n +2 | \
    awk -v ts="$NOW" '
        $1 ~ /^\/dev\// && $1 !~ /\/loop/ && !vu[$1]++ {
            gsub("G","",$3); gsub("G","",$4); gsub("%","",$5);
            printf "{\"timestamp\":\"%s\",\"mount\":\"%s\",\"used_gb\":%s,\"total_gb\":%s,\"usage_pct\":%s}\n", ts, $2, $4, $3, $5
        }')

# Append au fichier history (un JSON object par ligne, max 7 jours = 168 entrees a 1/h)
echo "$SNAPSHOT" >> $HISTORY.tmp
[ -f $HISTORY ] && cat $HISTORY >> $HISTORY.tmp
mv $HISTORY.tmp $HISTORY
# Garde les 1000 plus recentes
tail -1000 $HISTORY > $HISTORY.tmp && mv $HISTORY.tmp $HISTORY

# Genere le JSON final
{
    echo "{"
    echo "  \"updated_at\": \"$NOW\","
    echo "  \"points\": ["
    F=1
    while IFS= read -r P; do
        [ -z "$P" ] && continue
        [ $F -eq 0 ] && echo "    ,"
        echo "    $P"
        F=0
    done < $HISTORY
    echo "  ]"
    echo "}"
} > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"
chmod 644 "$OUT"

# Snapshot instantane lisible par l'API live (/api/system/info). L'API
# dans son container Docker ne voit que son rootfs via sysinfo, donc on
# expose ici la photo host (tous les disques /dev/*).
{
    echo "{"
    echo "  \"updated_at\": \"$NOW\","
    echo "  \"disks\": ["
    F=1
    while IFS= read -r P; do
        [ -z "$P" ] && continue
        [ $F -eq 0 ] && echo "    ,"
        echo "    $P"
        F=0
    done <<< "$SNAPSHOT"
    echo "  ]"
    echo "}"
} > "$CURRENT.tmp" && mv "$CURRENT.tmp" "$CURRENT"
chmod 644 "$CURRENT"
EOF
    chmod +x "$SCRIPT_DIR/sentinel-disk-trend.sh"
    "$SCRIPT_DIR/sentinel-disk-trend.sh"
    # Toutes les heures
    echo "0 * * * * root /usr/local/bin/sentinel-disk-trend.sh" > /etc/cron.d/sentinel-disk-trend
    echo "  ✅ disk-trend configure"
}

# ── Module connections ──────────────────────────────────────────────────

setup_connections() {
    echo "🌐 connections"
    apt_install iproute2

    cat > "$SCRIPT_DIR/sentinel-connections.sh" <<'EOF'
#!/bin/bash
set -eu
OUT=/var/lib/sentinel/connections.json
mkdir -p /var/lib/sentinel
NOW=$(date -Iseconds)

ENTRIES=$(ss -tn state established 2>/dev/null | tail -n +2 || true)
TOTAL=$(echo "$ENTRIES" | grep -c . || echo 0)

{
    echo "{"
    echo "  \"updated_at\": \"$NOW\","
    echo "  \"total\": $TOTAL,"
    echo "  \"connections\": ["
    F=1
    echo "$ENTRIES" | head -100 | while IFS= read -r LINE; do
        [ -z "$LINE" ] && continue
        LOCAL=$(echo "$LINE" | awk '{print $3}')
        REMOTE=$(echo "$LINE" | awk '{print $4}')
        [ $F -eq 0 ] && echo "    ,"
        echo "    {\"state\":\"established\",\"local_addr\":\"$LOCAL\",\"remote_addr\":\"$REMOTE\",\"process\":null}"
        F=0
    done
    echo "  ]"
    echo "}"
} > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"
chmod 644 "$OUT"
EOF
    chmod +x "$SCRIPT_DIR/sentinel-connections.sh"
    "$SCRIPT_DIR/sentinel-connections.sh"
    echo "*/2 * * * * root /usr/local/bin/sentinel-connections.sh" > /etc/cron.d/sentinel-connections
    echo "  ✅ connections configure"
}

# ── Module open-ports ───────────────────────────────────────────────────

setup_open_ports() {
    echo "🔍 open-ports"
    apt_install nmap

    cat > "$SCRIPT_DIR/sentinel-open-ports.sh" <<'EOF'
#!/bin/bash
set -eu
OUT=/var/lib/sentinel/open-ports.json
mkdir -p /var/lib/sentinel
NOW=$(date -Iseconds)
EXPECTED_PORTS="22 2222 80 443"

# Scan de l'IP publique : on ne veut voir que la surface d'attaque externe,
# pas les services bindes sur 127.0.0.1 (containerd, postfix local, etc).
# Si l'IP publique est injoignable depuis l'host (hairpin NAT KO), le scan
# renverra 0 port et "unexpected_count" sera 0 -- a verifier au deploiement.
TARGET=$(curl -s --max-time 5 https://api.ipify.org 2>/dev/null || true)
if [ -z "$TARGET" ]; then
    echo "WARN: IP publique non resolue, fallback sur localhost" >&2
    TARGET=localhost
fi
PORTS=$(nmap -p- -T4 --open "$TARGET" 2>/dev/null | grep -E "^[0-9]+/" | head -50 || true)
UNEXPECTED=0

{
    echo "{"
    echo "  \"updated_at\": \"$NOW\","
    echo "  \"ports\": ["
    F=1
    while IFS= read -r LINE; do
        [ -z "$LINE" ] && continue
        PORT=$(echo "$LINE" | awk -F'/' '{print $1}')
        PROTO=$(echo "$LINE" | awk -F'/' '{print $2}' | awk '{print $1}')
        SERVICE=$(echo "$LINE" | awk '{print $3}')
        EXPECTED="false"
        if echo "$EXPECTED_PORTS" | grep -qw "$PORT"; then EXPECTED="true"; else UNEXPECTED=$((UNEXPECTED+1)); fi
        [ $F -eq 0 ] && echo "    ,"
        echo "    {\"port\":$PORT,\"protocol\":\"$PROTO\",\"service\":\"$SERVICE\",\"expected\":$EXPECTED}"
        F=0
    done <<< "$PORTS"
    echo "  ],"
    echo "  \"unexpected_count\": $UNEXPECTED"
    echo "}"
} > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"
chmod 644 "$OUT"
EOF
    chmod +x "$SCRIPT_DIR/sentinel-open-ports.sh"
    "$SCRIPT_DIR/sentinel-open-ports.sh"
    # 1x par heure (nmap est lent)
    echo "30 * * * * root /usr/local/bin/sentinel-open-ports.sh" > /etc/cron.d/sentinel-open-ports
    echo "  ✅ open-ports configure"
}

# ── Module file-integrity ──────────────────────────────────────────────

setup_file_integrity() {
    echo "📁 file-integrity"

    cat > "$SCRIPT_DIR/sentinel-file-integrity.sh" <<'EOF'
#!/bin/bash
set -eu
OUT=/var/lib/sentinel/file-integrity.json
BASELINE=/var/lib/sentinel/.integrity-baseline.txt
mkdir -p /var/lib/sentinel

# Liste des fichiers a surveiller (modifie selon ton setup)
FILES=(
    "/etc/nginx/conf.d/default.conf"
    "/etc/fail2ban/jail.local"
    "/etc/cron.d/fail2ban-export"
    "/etc/cron.d/sentinel-apply-bans"
    "/etc/cron.d/sentinel-ssh-failures"
    "/usr/local/bin/sentinel-apply-bans.sh"
    "/usr/local/bin/fail2ban-export.sh"
    "/etc/ufw/user.rules"
)

# Au premier run, etablit le baseline
if [ ! -f $BASELINE ]; then
    : > $BASELINE
    for F in "${FILES[@]}"; do
        if [ -f "$F" ]; then
            HASH=$(sha256sum "$F" | awk '{print $1}')
            echo -e "$F\t$HASH" >> $BASELINE
        fi
    done
fi

NOW=$(date -Iseconds)
BASELINE_AT=$(stat -c %y $BASELINE 2>/dev/null | cut -d. -f1 || echo "")
MODIFIED=0

{
    echo "{"
    echo "  \"updated_at\": \"$NOW\","
    echo "  \"baseline_at\": \"$BASELINE_AT\","
    echo "  \"files\": ["
    F_FIRST=1
    for FILE in "${FILES[@]}"; do
        STATUS="missing"
        HASH=""
        MTIME=""
        if [ -f "$FILE" ]; then
            HASH=$(sha256sum "$FILE" | awk '{print $1}')
            MTIME=$(stat -c %y "$FILE" 2>/dev/null | cut -d. -f1)
            BASELINE_HASH=$(grep -F "$FILE" $BASELINE 2>/dev/null | awk '{print $2}' || echo "")
            if [ -z "$BASELINE_HASH" ] || [ "$HASH" = "$BASELINE_HASH" ]; then
                STATUS="ok"
            else
                STATUS="modified"
                MODIFIED=$((MODIFIED+1))
            fi
        fi
        [ $F_FIRST -eq 0 ] && echo "    ,"
        echo "    {\"path\":\"$FILE\",\"sha256\":\"$HASH\",\"modified_at\":\"$MTIME\",\"status\":\"$STATUS\"}"
        F_FIRST=0
    done
    echo "  ],"
    echo "  \"modified_count\": $MODIFIED"
    echo "}"
} > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"
chmod 644 "$OUT"
EOF
    chmod +x "$SCRIPT_DIR/sentinel-file-integrity.sh"
    "$SCRIPT_DIR/sentinel-file-integrity.sh"
    echo "*/30 * * * * root /usr/local/bin/sentinel-file-integrity.sh" > /etc/cron.d/sentinel-file-integrity
    echo "  ✅ file-integrity configure (baseline initial cree)"
    echo "  ℹ Pour re-baseliner apres modif legitime : rm /var/lib/sentinel/.integrity-baseline.txt"
}

# ── Module outbound ────────────────────────────────────────────────────

setup_outbound() {
    echo "🌐 outbound"
    apt_install iproute2

    cat > "$SCRIPT_DIR/sentinel-outbound.sh" <<'EOF'
#!/bin/bash
set -eu
OUT=/var/lib/sentinel/outbound.json
TMP=$(mktemp)
mkdir -p /var/lib/sentinel
NOW=$(date -Iseconds)

# ss -tnp state established :
# colonnes : Recv-Q Send-Q Local-Addr:Port Peer-Addr:Port users:(("name",pid=...,fd=...))
# On filtre les peers privees (localhost / LAN) sur la colonne $4 et on
# extrait le nom de process via regex propre (pas de double-quote en sortie).
ss -H -tnp state established 2>/dev/null \
    | awk '$4 !~ /^127\.|^10\.|^172\.(1[6-9]|2[0-9]|3[01])\.|^192\.168\.|^::1|^\[::1\]|^\[fe80|^\[::ffff:127\.|^\[::ffff:10\.|^\[::ffff:172\.(1[6-9]|2[0-9]|3[01])\.|^\[::ffff:192\.168\./' \
    | head -100 > "$TMP" || true

TOTAL=$(wc -l < "$TMP" | tr -d ' ')

{
    echo "{"
    echo "  \"updated_at\": \"$NOW\","
    echo "  \"total\": $TOTAL,"
    echo "  \"connections\": ["
    awk 'BEGIN { first=1 }
    {
        local=$3; remote=$4;
        # Process : extrait le 1er nom dans users:(("name",pid=...,fd=...))
        proc="";
        if (match($0, /users:\(\("[^"]+/)) {
            proc=substr($0, RSTART, RLENGTH);
            sub(/users:\(\("/, "", proc);
        }
        # Echappe les " et \ pour le JSON
        gsub(/\\/, "\\\\", local); gsub(/"/, "\\\"", local);
        gsub(/\\/, "\\\\", remote); gsub(/"/, "\\\"", remote);
        gsub(/\\/, "\\\\", proc); gsub(/"/, "\\\"", proc);
        sep = (first==1) ? "" : ",";
        printf("%s\n    {\"local_addr\":\"%s\",\"remote_addr\":\"%s\",\"remote_host\":null,\"process\":\"%s\"}", sep, local, remote, proc);
        first=0;
    }
    END { if (first==0) print "" }' "$TMP"
    echo "  ]"
    echo "}"
} > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"
rm -f "$TMP"
chmod 644 "$OUT"
EOF
    chmod +x "$SCRIPT_DIR/sentinel-outbound.sh"
    "$SCRIPT_DIR/sentinel-outbound.sh"
    echo "*/3 * * * * root /usr/local/bin/sentinel-outbound.sh" > /etc/cron.d/sentinel-outbound
    echo "  ✅ outbound configure"
}

# ── Module trivy ────────────────────────────────────────────────────────

setup_trivy() {
    echo "🐳 trivy"

    if ! command -v trivy &>/dev/null; then
        echo "  Installation Trivy…"
        apt-get install -y wget gnupg lsb-release
        wget -qO - https://aquasecurity.github.io/trivy-repo/deb/public.key | apt-key add - 2>/dev/null
        echo "deb https://aquasecurity.github.io/trivy-repo/deb $(lsb_release -sc) main" | tee /etc/apt/sources.list.d/trivy.list
        apt-get update -qq
        apt-get install -y trivy
    fi

    cat > "$SCRIPT_DIR/sentinel-trivy-scan.sh" <<'EOF'
#!/bin/bash
set -eu
OUT=/var/lib/sentinel/trivy.json
mkdir -p /var/lib/sentinel
NOW=$(date -Iseconds)

# Scan toutes les images discordsentinel-*
IMAGES=$(docker images --format "{{.Repository}}:{{.Tag}}" 2>/dev/null | grep "^discordsentinel-" || true)

CRIT=0; HIGH=0; MED=0; LOW=0
VULNS_JSON=""

while IFS= read -r IMG; do
    [ -z "$IMG" ] && continue
    RAW=$(trivy image --quiet --severity CRITICAL,HIGH,MEDIUM,LOW -f json "$IMG" 2>/dev/null || echo '{}')
    # Parse via jq si dispo, sinon skip
    if command -v jq &>/dev/null; then
        IMG_VULNS=$(echo "$RAW" | jq -c --arg img "$IMG" '
            .Results[]?.Vulnerabilities[]? |
            {image:$img, cve:.VulnerabilityID, severity:.Severity,
             package:.PkgName, fixed_version:.FixedVersion}' 2>/dev/null | tr '\n' ',' || true)
        VULNS_JSON+="$IMG_VULNS"
    fi
done <<< "$IMAGES"

# Compteurs (rough, depuis le json final)
VULNS_JSON=$(echo "$VULNS_JSON" | sed 's/,$//')
[ -z "$VULNS_JSON" ] && VULNS_JSON="[]" || VULNS_JSON="[$VULNS_JSON]"

if command -v jq &>/dev/null; then
    CRIT=$(echo "$VULNS_JSON" | jq '[.[] | select(.severity=="CRITICAL")] | length')
    HIGH=$(echo "$VULNS_JSON" | jq '[.[] | select(.severity=="HIGH")] | length')
    MED=$(echo "$VULNS_JSON" | jq '[.[] | select(.severity=="MEDIUM")] | length')
    LOW=$(echo "$VULNS_JSON" | jq '[.[] | select(.severity=="LOW")] | length')
fi

{
    echo "{"
    echo "  \"updated_at\": \"$NOW\","
    echo "  \"critical\": $CRIT,"
    echo "  \"high\": $HIGH,"
    echo "  \"medium\": $MED,"
    echo "  \"low\": $LOW,"
    echo "  \"vulnerabilities\": $VULNS_JSON"
    echo "}"
} > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"
chmod 644 "$OUT"
EOF
    chmod +x "$SCRIPT_DIR/sentinel-trivy-scan.sh"
    apt_install jq

    # Trivy est lent (~1min/image), 1x par jour la nuit
    echo "0 3 * * * root /usr/local/bin/sentinel-trivy-scan.sh" > /etc/cron.d/sentinel-trivy
    echo "  ✅ trivy configure (scan 1x/jour 3h du matin, ou lance manuellement /usr/local/bin/sentinel-trivy-scan.sh)"
}

# ── Module nginx-suspicious ─────────────────────────────────────────────

setup_nginx_suspicious() {
    echo "🚨 nginx-suspicious"
    apt_install jq

    cat > "$SCRIPT_DIR/sentinel-nginx-suspicious.sh" <<'EOF'
#!/bin/bash
set -e
OUT=/var/lib/sentinel/nginx-suspicious.json
NOW=$(date -Is)
CONTAINER=$(docker ps --filter name=web --format '{{.Names}}' | head -n1)
[ -z "$CONTAINER" ] && CONTAINER=$(docker ps --filter name=nginx --format '{{.Names}}' | head -n1)
if [ -z "$CONTAINER" ]; then
    echo "{\"updated_at\":\"$NOW\",\"total_24h\":0,\"by_category\":{},\"entries\":[],\"error\":\"container web/nginx introuvable\"}" > "$OUT"
    chmod 644 "$OUT"; exit 0
fi

LOGS=$(docker logs --since 24h "$CONTAINER" 2>&1 || true)
ENTRIES=""
TOTAL=0
declare -A CATS
CATS[scanner]=0; CATS[sqli]=0; CATS[xss]=0; CATS[traversal]=0

while IFS= read -r line; do
    [ -z "$line" ] && continue
    # nginx combined: IP - - [time] "METHOD URL HTTP/x" status size "ref" "UA"
    IP=$(echo "$line" | awk '{print $1}')
    METHOD=$(echo "$line" | grep -oP '"\K[A-Z]+(?= )' | head -n1)
    URL=$(echo "$line" | grep -oP '"[A-Z]+ \K[^ ]+' | head -n1)
    STATUS=$(echo "$line" | grep -oP '" \K[0-9]{3}' | head -n1)
    UA=$(echo "$line" | grep -oP '"[^"]*"$' | tr -d '"')
    [ -z "$IP" ] || [ -z "$URL" ] && continue

    CAT=""
    LURL=$(echo "$URL" | tr 'A-Z' 'a-z')
    LUA=$(echo "$UA" | tr 'A-Z' 'a-z')

    # Path traversal
    if echo "$URL" | grep -qE '\.\./|/\.\.|\%2e\%2e' ; then CAT="traversal"
    # SQLi
    elif echo "$LURL" | grep -qE '(union[+ ]select|select[+ ].*from|or[+ ]1=1|sleep\(|benchmark\(|information_schema|--[+ ]|/\*\*/)' ; then CAT="sqli"
    # XSS
    elif echo "$LURL" | grep -qE '(<script|javascript:|onerror=|onload=|alert\()' ; then CAT="xss"
    # Scanners (paths bien connus + UA)
    elif echo "$LURL" | grep -qE '(/\.env|/\.git|/\.svn|/\.aws|/\.ssh|/\.htaccess|/wp-admin|/wp-login|/wp-content|/wp-includes|/wp-json|/wordpress|/xmlrpc\.php|/phpmyadmin|/pma|/myadmin|/adminer|/laravel|/joomla|/drupal|/magento|/administrator|/admin\.php|\.php|\.aspx|\.jsp|/vendor/phpunit|/cgi-bin|/owa/|/autodiscover|/ecp/|/_ignition|/_profiler|/server-status|/server-info|/manager/html|/composer\.json)' ; then CAT="scanner"
    elif echo "$LUA" | grep -qE '(nmap|sqlmap|nikto|masscan|zgrab|nuclei|gobuster|dirbuster|wpscan|acunetix|nessus|openvas|hydra)' ; then CAT="scanner"
    fi

    [ -z "$CAT" ] && continue
    CATS[$CAT]=$((CATS[$CAT]+1))
    TOTAL=$((TOTAL+1))

    # Limite a 200 entrees
    if [ $TOTAL -le 200 ]; then
        IPESC=$(echo "$IP" | jq -Rs '.' 2>/dev/null || echo "\"$IP\"")
        URLESC=$(echo "$URL" | jq -Rs '.' 2>/dev/null || echo "\"\"")
        UAESC=$(echo "$UA" | jq -Rs '.' 2>/dev/null || echo "\"\"")
        ENTRY="{\"timestamp\":\"$NOW\",\"ip\":$IPESC,\"method\":\"${METHOD:-?}\",\"url\":$URLESC,\"status\":${STATUS:-0},\"category\":\"$CAT\",\"user_agent\":$UAESC}"
        [ -z "$ENTRIES" ] && ENTRIES="$ENTRY" || ENTRIES="$ENTRIES,$ENTRY"
    fi
done <<< "$LOGS"

BY_CAT="{\"scanner\":${CATS[scanner]},\"sqli\":${CATS[sqli]},\"xss\":${CATS[xss]},\"traversal\":${CATS[traversal]}}"

cat > "$OUT.tmp" <<JSON
{"updated_at":"$NOW","total_24h":$TOTAL,"by_category":$BY_CAT,"entries":[$ENTRIES]}
JSON
mv "$OUT.tmp" "$OUT"
chmod 644 "$OUT"
EOF
    chmod +x "$SCRIPT_DIR/sentinel-nginx-suspicious.sh"
    echo "*/10 * * * * root /usr/local/bin/sentinel-nginx-suspicious.sh" > /etc/cron.d/sentinel-nginx-suspicious
    echo "  ✅ nginx-suspicious configure (toutes les 10 min)"
}

# ── Module tls-errors ───────────────────────────────────────────────────

setup_tls_errors() {
    echo "🔒 tls-errors"
    apt_install jq

    cat > "$SCRIPT_DIR/sentinel-tls-errors.sh" <<'EOF'
#!/bin/bash
set -e
OUT=/var/lib/sentinel/tls-errors.json
NOW=$(date -Is)
CONTAINER=$(docker ps --filter name=web --format '{{.Names}}' | head -n1)
[ -z "$CONTAINER" ] && CONTAINER=$(docker ps --filter name=nginx --format '{{.Names}}' | head -n1)
if [ -z "$CONTAINER" ]; then
    echo "{\"updated_at\":\"$NOW\",\"total_24h\":0,\"entries\":[],\"error\":\"container web introuvable\"}" > "$OUT"
    chmod 644 "$OUT"; exit 0
fi

LOGS=$(docker logs --since 24h "$CONTAINER" 2>&1 | grep -iE 'ssl|tls|handshake' | grep -iE 'error|fail|alert' || true)
ENTRIES=""
TOTAL=0
while IFS= read -r line; do
    [ -z "$line" ] && continue
    CLIENT=$(echo "$line" | grep -oP 'client: \K[0-9.:a-fA-F]+' | head -n1)
    [ -z "$CLIENT" ] && CLIENT="?"
    ERR=$(echo "$line" | sed 's/"/\\"/g' | head -c 240)
    TOTAL=$((TOTAL+1))
    if [ $TOTAL -le 100 ]; then
        ERRESC=$(echo "$ERR" | jq -Rs '.' 2>/dev/null || echo "\"$ERR\"")
        ENTRY="{\"timestamp\":\"$NOW\",\"client\":\"$CLIENT\",\"error\":$ERRESC}"
        [ -z "$ENTRIES" ] && ENTRIES="$ENTRY" || ENTRIES="$ENTRIES,$ENTRY"
    fi
done <<< "$LOGS"

cat > "$OUT.tmp" <<JSON
{"updated_at":"$NOW","total_24h":$TOTAL,"entries":[$ENTRIES]}
JSON
mv "$OUT.tmp" "$OUT"
chmod 644 "$OUT"
EOF
    chmod +x "$SCRIPT_DIR/sentinel-tls-errors.sh"
    echo "*/15 * * * * root /usr/local/bin/sentinel-tls-errors.sh" > /etc/cron.d/sentinel-tls-errors
    echo "  ✅ tls-errors configure (toutes les 15 min)"
}

# ── Module nginx-scanner (jail fail2ban) ────────────────────────────────
# Bannit les IPs qui hit notre trap 444 (paths /laravel, /wp-admin, .env,
# .php, etc. — cf. web/nginx.conf "Anti-scanner"). Les requetes 444
# sont logguees dans /var/log/nginx/sentinel/scanners.log via volume bind.
# Pre-requis : fail2ban deja installe (module fail2ban).

setup_nginx_scanner() {
    echo "🚫 nginx-scanner (jail fail2ban)"

    if ! command -v fail2ban-client &>/dev/null; then
        echo "  ❌ fail2ban absent. Lance d'abord : sudo bash $0 fail2ban"
        exit 1
    fi

    # 1. Repertoire host pour les logs scanner. UID 101 = user nginx
    #    dans nginx:1.27-alpine (cf. web/Dockerfile).
    LOG_DIR=/var/log/sentinel-nginx
    mkdir -p "$LOG_DIR"
    chown 101:101 "$LOG_DIR"
    chmod 755 "$LOG_DIR"
    touch "$LOG_DIR/scanners.log"
    chown 101:101 "$LOG_DIR/scanners.log"

    # 2. Filtre fail2ban : matche les lignes nginx combined log avec status 444.
    cat > /etc/fail2ban/filter.d/nginx-scanner.conf <<'EOF'
# fail2ban filter for sentinel nginx scanner trap.
# nginx ecrit dans scanners.log uniquement les requetes qui ont hit
# une location "Anti-scanner" et recu 444 (return 444 dans nginx.conf).
# Donc UNE entree dans ce log = UNE tentative scanner -> bannir direct.
[Definition]
failregex = ^<HOST> .* "(GET|POST|HEAD|PUT|DELETE|PATCH|OPTIONS) [^"]*" 444 .*$
ignoreregex =
EOF

    # 3. Jail : 3 hits sur 1h -> ban 24h.
    if grep -q "^\[nginx-scanner\]" /etc/fail2ban/jail.local 2>/dev/null; then
        echo "  ℹ Jail nginx-scanner deja present dans jail.local — pas d'override."
    else
        cat >> /etc/fail2ban/jail.local <<EOF

[nginx-scanner]
enabled  = true
filter   = nginx-scanner
# polling : sinon defaults-debian.conf force backend=systemd et le logpath est ignore.
backend  = polling
logpath  = $LOG_DIR/scanners.log
maxretry = 3
findtime = 1h
bantime  = 24h
banaction = ufw
EOF
    fi

    # Patch retroactif si le bloc existe deja sans backend (installations < 2026-05).
    if grep -q "^\[nginx-scanner\]" /etc/fail2ban/jail.local 2>/dev/null \
       && ! awk '/^\[nginx-scanner\]/,/^\[/' /etc/fail2ban/jail.local | grep -q "^backend"; then
        sed -i '/^\[nginx-scanner\]/a backend  = polling' /etc/fail2ban/jail.local
        echo "  🔧 backend=polling ajoute au jail existant"
    fi

    systemctl restart fail2ban
    sleep 1
    if fail2ban-client status nginx-scanner &>/dev/null; then
        echo "  ✅ jail nginx-scanner actif (3 hits/1h -> ban 24h)"
    else
        echo "  ⚠ jail nginx-scanner non charge — verifie 'systemctl status fail2ban'"
    fi
}

# ── Module recidive (jail fail2ban meta) ────────────────────────────────
# Bannit 1 semaine toute IP deja bannie 3x dans /var/log/fail2ban.log
# (par n'importe quel jail). Complement de nginx-scanner pour les scanners
# persistants qui reviennent apres expiration du ban 24h.

setup_recidive() {
    echo "🔁 recidive (jail fail2ban meta)"

    if ! command -v fail2ban-client &>/dev/null; then
        echo "  ❌ fail2ban absent. Lance d'abord : sudo bash $0 fail2ban"
        exit 1
    fi

    if grep -q "^\[recidive\]" /etc/fail2ban/jail.local 2>/dev/null; then
        echo "  ℹ Jail recidive deja present dans jail.local — pas d'override."
    else
        cat >> /etc/fail2ban/jail.local <<'EOF'

[recidive]
enabled  = true
filter   = recidive
backend  = polling
logpath  = /var/log/fail2ban.log
maxretry = 3
findtime = 1d
bantime  = 1w
banaction = ufw
EOF
    fi

    systemctl restart fail2ban
    sleep 1
    if fail2ban-client status recidive &>/dev/null; then
        echo "  ✅ jail recidive actif (3 bans/24h -> ban 1 semaine)"
    else
        echo "  ⚠ jail recidive non charge — verifie 'systemctl status fail2ban'"
    fi
}

# ── Dispatcher ──────────────────────────────────────────────────────────

main() {
    require_root
    ensure_data_dir

    case "${1:-help}" in
        fail2ban)     setup_fail2ban ;;
        ban-apply)    setup_ban_apply ;;
        ssh-failures) setup_ssh_failures ;;
        disk-trend)   setup_disk_trend ;;
        connections)  setup_connections ;;
        open-ports)   setup_open_ports ;;
        trivy)          setup_trivy ;;
        file-integrity)    setup_file_integrity ;;
        outbound)          setup_outbound ;;
        nginx-suspicious)  setup_nginx_suspicious ;;
        nginx-scanner)     setup_nginx_scanner ;;
        recidive)          setup_recidive ;;
        tls-errors)        setup_tls_errors ;;
        all)
            setup_fail2ban
            setup_ban_apply
            setup_ssh_failures
            setup_disk_trend
            setup_connections
            setup_open_ports
            setup_trivy
            setup_file_integrity
            setup_outbound
            setup_nginx_suspicious
            setup_nginx_scanner
            setup_recidive
            setup_tls_errors
            ;;
        help|--help|-h|*)
            cat <<'HELP'
Usage: sudo bash setup-host-security.sh <module>

Modules :
  fail2ban       Installation fail2ban + jail SSH + cron export status
  ban-apply      Cron qui applique les bans/unbans IPs ecrits par l'API
  ssh-failures   Cron parse journalctl SSH -> ssh-failures.json (5 min)
  disk-trend     Cron snapshot df -> disk-trend.json (1h, history 7j)
  connections    Cron snapshot ss -tn -> connections.json (2 min)
  open-ports     Cron nmap localhost -> open-ports.json (1h)
  trivy          Scan vulns Docker images -> trivy.json (1x/jour 3h)
  file-integrity SHA256 fichiers critiques -> file-integrity.json (30 min)
  outbound       Connexions sortantes -> outbound.json (3 min)
  nginx-suspicious Patterns SQLi/XSS/scanners nginx -> nginx-suspicious.json (10 min)
  nginx-scanner  Jail fail2ban qui ban les IPs hit le trap 444 nginx
  recidive       Jail meta : ban 1 semaine les IPs deja bannies 3x/24h
  tls-errors     Erreurs handshake TLS nginx -> tls-errors.json (15 min)
  all            Tous les modules

Exemples :
  sudo bash setup-host-security.sh fail2ban
  sudo bash setup-host-security.sh all
HELP
            ;;
    esac
}

main "$@"
