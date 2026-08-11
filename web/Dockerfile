# syntax=docker/dockerfile:1.7
# Multi-stage : build Vite (Node) -> serve static via nginx (HTTP + HTTPS).
# Build context = racine du repo (cf. docker-compose.yml).

# ----- Stage 1 : build SPA -----
FROM node:22-alpine AS builder
WORKDIR /app

# Cache des deps : copie juste les manifests d'abord.
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund

# Sources
COPY web/ ./

# URL de l'API injectee au build. Vide par defaut => fetch en relatif,
# donc les appels /api/* atterrissent sur le reverse-proxy nginx local.
# Override possible via `--build-arg VITE_API_URL=https://api.example.com`.
ARG VITE_API_URL=""
ENV VITE_API_URL=${VITE_API_URL}

RUN npm run build

# ----- Stage 2 : nginx runtime -----
FROM nginx:1.27-alpine AS runtime

# openssl : generation du cert self-signed en dev si aucun LE cert trouve.
RUN apk add --no-cache openssl

# Config nginx (HTTP -> HTTPS redirect + reverse-proxy TLS /api et /ws).
COPY web/nginx.conf /etc/nginx/conf.d/default.conf
# Headers de securite partages (inclus dans server + locations, cf. piege
# add_header documente dans le fichier).
COPY web/security-headers.inc /etc/nginx/snippets/security-headers.inc

# Entrypoint : prepare les certs (self-signed fallback) puis lance nginx.
# `sed` strip des CR eventuels (si edite sous Windows) — sinon shebang casse.
COPY web/docker-entrypoint.sh /docker-entrypoint.d/40-init-tls.sh
RUN sed -i 's/\r$//' /docker-entrypoint.d/40-init-tls.sh \
    && chmod +x /docker-entrypoint.d/40-init-tls.sh

# Genere le snippet d'injection de la cle Nexus (proxy /nexus-api/). Doit
# tourner AVANT nginx : l'include serait sinon introuvable.
COPY web/docker-entrypoint-nexus.sh /docker-entrypoint.d/30-nexus-key.sh
RUN sed -i 's/\r$//' /docker-entrypoint.d/30-nexus-key.sh \
    && chmod +x /docker-entrypoint.d/30-nexus-key.sh

# Idem pour le jeton Atrium (proxy /atrium-api/). Meme contrainte d'ordre :
# l'include de nginx.conf doit exister avant le demarrage de nginx.
COPY web/docker-entrypoint-atrium.sh /docker-entrypoint.d/31-atrium-key.sh
RUN sed -i 's/\r$//' /docker-entrypoint.d/31-atrium-key.sh \
    && chmod +x /docker-entrypoint.d/31-atrium-key.sh

# Idem pour le jeton Exploitation (proxy /ops-api/).
COPY web/docker-entrypoint-ops.sh /docker-entrypoint.d/32-ops-key.sh
RUN sed -i 's/\r$//' /docker-entrypoint.d/32-ops-key.sh \
    && chmod +x /docker-entrypoint.d/32-ops-key.sh

# Jeton de l'API d'identite. Il alimente les sous-requetes d'autorisation des
# TROIS passerelles (/nexus-api/, /ops-api/, /atrium-api/) : sans lui, aucune
# ne laisse passer.
COPY web/docker-entrypoint-auth.sh /docker-entrypoint.d/33-auth-key.sh
RUN sed -i 's/\r$//' /docker-entrypoint.d/33-auth-key.sh \
    && chmod +x /docker-entrypoint.d/33-auth-key.sh

# Assets statiques
COPY --from=builder /app/dist /usr/share/nginx/html

# Configuration PUBLIQUE du site (guilde, invitation Discord), ecrite a
# l'execution. Doit passer APRES la copie des assets : elle ecrit dans le
# meme repertoire, que le COPY ci-dessus remplacerait.
COPY web/docker-entrypoint-site.sh /docker-entrypoint.d/35-site-config.sh
RUN sed -i 's/\r$//' /docker-entrypoint.d/35-site-config.sh \
    && chmod +x /docker-entrypoint.d/35-site-config.sh

EXPOSE 80 443

# Healthcheck : endpoint HTTP /healthz expose par nginx sur le port 80
# (evite le probleme de cert self-signed non trustable en HTTPS).
HEALTHCHECK --interval=30s --timeout=3s --retries=3 --start-period=5s \
  CMD wget --spider -q http://localhost/healthz || exit 1
