#!/bin/sh
# Genere une CA + cert serveur + cert client pour mTLS gRPC inter-services.
# Idempotent : ne regenere pas si les fichiers existent deja (evite de
# casser les conteneurs deja deployes).
#
# Sortie : /grpc-certs/{ca.pem, server.pem, server.key, client.pem, client.key}
# Mounts attendus : volume nomme `grpc_certs` partage en read-only entre
# l'API (serveur) et les workers/bot (clients).

set -eu

CERT_DIR="${CERT_DIR:-/grpc-certs}"
CA_DAYS="${CA_DAYS:-3650}"   # 10 ans pour la CA interne
CERT_DAYS="${CERT_DAYS:-825}" # 825j max pour les certs (limite navigateurs/Apple, ok pour Rust aussi)
CN_SERVER="${CN_SERVER:-api}"

mkdir -p "$CERT_DIR"
cd "$CERT_DIR"

# ── 1. CA auto-signee (si pas deja la) ──
if [ ! -f ca.pem ]; then
    echo "[gen-grpc-certs] Generating CA..."
    openssl genrsa -out ca.key 4096
    openssl req -x509 -new -nodes -key ca.key -sha256 -days "$CA_DAYS" \
        -subj "/CN=DiscordSentinel-Internal-CA" \
        -out ca.pem
fi

# ── 2. Cert serveur (CN = "api", SAN = "api","localhost") ──
if [ ! -f server.pem ]; then
    echo "[gen-grpc-certs] Generating server cert (CN=$CN_SERVER)..."
    openssl genrsa -out server.key 2048

    cat > server.cnf <<EOF
[req]
distinguished_name = req_distinguished_name
prompt = no
req_extensions = v3_req
[req_distinguished_name]
CN = $CN_SERVER
[v3_req]
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names
[alt_names]
DNS.1 = $CN_SERVER
DNS.2 = localhost
IP.1 = 127.0.0.1
EOF

    openssl req -new -key server.key -out server.csr -config server.cnf
    openssl x509 -req -in server.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
        -days "$CERT_DAYS" -sha256 -extfile server.cnf -extensions v3_req \
        -out server.pem
    rm -f server.csr server.cnf
fi

# ── 3. Cert client (commun a tous les services clients : workers + bot) ──
# Pour simplifier, un seul cert client partage. Suffit pour le mTLS basique
# (encryption + auth mutuelle). Si on veut un cert par service plus tard,
# generer un .pem + .key par worker et router via CN.
if [ ! -f client.pem ]; then
    echo "[gen-grpc-certs] Generating client cert..."
    openssl genrsa -out client.key 2048

    cat > client.cnf <<EOF
[req]
distinguished_name = req_distinguished_name
prompt = no
req_extensions = v3_req
[req_distinguished_name]
CN = sentinel-internal-client
[v3_req]
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = clientAuth
EOF

    openssl req -new -key client.key -out client.csr -config client.cnf
    openssl x509 -req -in client.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
        -days "$CERT_DAYS" -sha256 -extfile client.cnf -extensions v3_req \
        -out client.pem
    rm -f client.csr client.cnf
fi

# Permissions : lecture pour les conteneurs (les conteneurs Rust s'exécutent en uid 1000,
# et le volume interne grpc_certs est isolé et monté en read-only :ro).
chmod 755 "$CERT_DIR" 2>/dev/null || true
chmod -R a+rX "$CERT_DIR" 2>/dev/null || true
chmod 644 "$CERT_DIR"/* 2>/dev/null || true
echo "[gen-grpc-certs] Done. Files in $CERT_DIR:"
ls -la "$CERT_DIR"

