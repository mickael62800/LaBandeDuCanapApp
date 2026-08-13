# Migration Docker — checklist avant prod

Ce document recense **les vérifications à faire** et les **risques connus** suite
au refactor Docker (commit `0e91486`). Lis-le entièrement avant de lancer
`docker compose up` sur ton serveur de prod.

> Contexte : consolidation de 16 Dockerfiles per-service en 2 Dockerfiles
> génériques + factorisation du `docker-compose.yml` avec des YAML anchors +
> hardening (user non-root, `init: true`, log rotation, healthchecks `/metrics`).

---

## 🟢 Ce qui est sécurisé par construction

### 1. Volumes existants préservés
J'ai ajouté `name: discordsentinel` au top du `docker-compose.yml`. Conséquence :
peu importe d'où tu lances `docker compose up` (depuis la racine du repo, depuis
`infrastructure/docker/`, ou ailleurs), Docker utilisera **toujours** le project name
`discordsentinel`. Tes volumes existants restent retrouvables :

- `discordsentinel_postgres_data` — DB Postgres
- `discordsentinel_redis_data` — cache Redis
- `discordsentinel_prometheus_data`
- `discordsentinel_grafana_data`
- `discordsentinel_letsencrypt_etc`
- `discordsentinel_letsencrypt_www`

### 2. Workers non-root sans risque d'écriture disque
Vérifié par `grep` sur tout le code Rust : **aucun** worker ni le bot n'utilise
`File::create`, `fs::write`, `tokio::fs::write` ou `tempfile`. Tous écrivent
uniquement vers Postgres/Redis/stdout. Le passage à `USER sentinel` (uid 1000)
ne casse rien.

### 3. Metriques hote isolees dans Ops
`sentinel-api` ne partage plus le namespace PID de l'hote. `ops-agent` monte
`/proc` sous `/host/proc` en lecture seule, collecte CPU/RAM puis publie un
snapshot ephemere dans Redis (`ops:host-metrics`). L'endpoint
`/api/system/info` lit ce snapshot et conserve seulement la mesure locale de
son propre processus.

### 4. Healthchecks workers via `/metrics`
`worker-common::spawn_metrics_server` expose `/metrics` sur le port 9100
(configurable via `METRICS_PORT`). Le healthcheck `wget --spider -q
http://localhost:9100/metrics` détecte les freezes de process (contrairement à
`pidof` qui passe sur un process bloqué).

### 5. Bot healthcheck cohérent avec le binaire renommé
Le binaire est désormais installé en `/usr/local/bin/sentinel-app` dans tous les
images alpine. Le healthcheck `pidof sentinel-app` matche bien (tini PID 1 +
sentinel-app PID 2).

---

## ⚠️ À vérifier AVANT de stopper la prod actuelle

### Étape 1 — Sauvegarde Postgres (paranoïa raisonnable)

Même si les volumes sont préservés, fais un dump avant tout changement majeur :

```bash
docker exec sentinel-postgres pg_dumpall -U sentinel > backup-$(date +%Y%m%d-%H%M%S).sql
```

Si `pg_dumpall` plante (ex: extension `pg_stat_statements` qui ne se dump pas),
fais un `pg_dump` ciblé sur la DB applicative :

```bash
docker exec sentinel-postgres pg_dump -U sentinel -d discord_sentinel \
  > backup-discord_sentinel-$(date +%Y%m%d-%H%M%S).sql
```

### Étape 2 — Vérifier les volumes existants

```bash
docker volume ls | grep discordsentinel
```

Tu dois voir **au minimum** `discordsentinel_postgres_data` et
`discordsentinel_redis_data`. Si tu vois aussi `docker_postgres_data` ou un
préfixe différent, c'est que des commandes précédentes ont créé des volumes
parallèles — il faudra les fusionner manuellement (cf. section
[Récupération](#-récupération-si-les-volumes-ont-divergé)).

### Étape 3 — Build à blanc (sans déployer)

```bash
# Depuis la racine du repo
docker compose -f infrastructure/docker/docker-compose.yml build
```

Compte ~5-15 min au premier build (cargo-chef cache vide). Si **aucune erreur**,
tu peux passer à l'étape 4. Si erreur, **NE TOUCHE PAS** à la prod actuelle et
revert le commit (cf. section [Rollback](#-rollback-si-ça-part-en-vrille)).

Alternative parallèle (gain ~50% si CPU dispo) :

```bash
docker buildx bake -f infrastructure/docker/docker-bake.hcl
```

### Étape 4 — Comparer les nouvelles images aux anciennes

```bash
docker images | grep -E "sentinel|discordsentinel"
```

Vérifie que les nouvelles images ont une taille raisonnable :
- API (debian-trixie-slim) : ~80-120 MB
- Workers / bot / gateway (alpine) : ~25-40 MB
- Web (nginx-alpine) : ~50 MB

Si une image fait > 500 MB, il y a un souci (cargo-chef qui n'a pas trim, ou
contexte de build pollué).

### Étape 5 — Smoke test isolé (recommandé)

Tu peux tester le nouveau stack **en parallèle** de l'ancien sans rien casser,
en utilisant un project name différent :

```bash
docker compose -f infrastructure/docker/docker-compose.yml \
  -p discordsentinel-staging \
  --env-file .env.staging \
  up -d postgres redis
```

(préparer un `.env.staging` avec des ports différents et un autre `POSTGRES_PASSWORD`
si tu veux vraiment bien isoler).

---

## 🚀 Déploiement en prod

```bash
# Depuis la racine du repo

# 1. Pull la dernière version
git pull

# 2. Stop l'ancien stack (volumes preserves)
docker compose -f infrastructure/docker/docker-compose.yml down

# 3. Re-build et up
docker compose -f infrastructure/docker/docker-compose.yml up -d --build

# 4. Surveille les logs sur les premieres minutes
docker compose -f infrastructure/docker/docker-compose.yml logs -f --tail=50
```

Si tu utilises Prometheus/Grafana :

```bash
docker compose -f infrastructure/docker/docker-compose.yml --profile monitoring up -d
```

Si tu utilises le profile TLS (Let's Encrypt) :

```bash
docker compose -f infrastructure/docker/docker-compose.yml --profile tls up -d
```

---

## ✅ Checklist post-déploiement

À cocher dans l'ordre, **immédiatement après le `up -d`** :

### Infrastructure
- [ ] `docker compose ps` montre `postgres`, `redis`, `pgbouncer` en `healthy`
- [ ] `docker exec sentinel-postgres psql -U sentinel -d discord_sentinel -c "SELECT count(*) FROM users;"` retourne le bon nombre (≈ même qu'avant migration)
- [ ] `docker exec sentinel-redis redis-cli -a $REDIS_PASSWORD DBSIZE` retourne un nombre cohérent

### Services Rust
- [ ] `docker compose ps` montre `api` healthy après ~30s
- [ ] `curl -s http://localhost:3000/health` retourne 200
- [ ] `docker compose ps` montre `gateway` healthy
- [ ] Le bot Discord se reconnecte (vérifier dans le serveur Discord : il passe online)
- [ ] Les 13 workers passent healthy en moins d'1 min
- [ ] `docker compose logs sentinel-bot --tail=20` ne montre pas de boucle d'erreur
- [ ] `docker compose logs api --tail=20 | grep -i error` est vide

### Web
- [ ] `https://<TON_DOMAINE>/` charge le dashboard
- [ ] OAuth Discord login fonctionne
- [ ] Les graphes chargent (analytics endpoints OK)

### Volumes
- [ ] `docker volume ls | grep discordsentinel` montre tous les volumes attendus
- [ ] Aucun volume `docker_*` ou autre préfixe parasite (sinon = data orpheline)

### Logs / observabilité
- [ ] Les logs sont rotés (vérifier sur les fichiers JSON dans
  `/var/lib/docker/containers/<id>/<id>-json.log`, ils ne dépassent pas 10 MB)
- [ ] Si profile monitoring activé : `http://localhost:3002` (Grafana) montre
  les métriques workers (port 9100 chacun)

---

## 🔄 Rollback si ça part en vrille

Le rollback est **non-destructif** (tes volumes ne sont pas touchés) :

```bash
git revert 0e91486
docker compose -f infrastructure/docker/docker-compose.yml down
docker compose -f infrastructure/docker/docker-compose.yml up -d --build
```

⚠️ **Attention** : avant le revert, l'ancienne stack utilisait les compose files
à la racine (commit antérieur à `e249ed3`). Si tu reverts plus loin, vérifie
que `docker-compose.yml` est bien là où ton ancien runbook l'attend.

---

## 🛟 Récupération si les volumes ont divergé

Cas problématique : tu lances `docker compose up` après le refactor mais les
volumes ne sont pas trouvés (création de volumes vides parallèles).

### Diagnostic
```bash
docker volume ls
```

Si tu vois à la fois :
- `discordsentinel_postgres_data` (l'ancien, plein)
- `docker_postgres_data` (le nouveau, vide créé par mégarde)

### Procédure de récupération

1. Stop tout :
   ```bash
   docker compose -f infrastructure/docker/docker-compose.yml down
   ```

2. Supprime le volume vide créé par erreur :
   ```bash
   docker volume rm docker_postgres_data docker_redis_data
   ```

3. Vérifie que le compose a bien `name: discordsentinel` :
   ```bash
   grep "^name:" infrastructure/docker/docker-compose.yml
   # doit afficher : name: discordsentinel
   ```

4. Relance :
   ```bash
   docker compose -f infrastructure/docker/docker-compose.yml up -d
   ```

   Le project name `discordsentinel` re-attache les anciens volumes.

### Si tu as déjà perdu de la donnée (volume vide écrasé)

Restaure le dump créé à l'**Étape 1** :

```bash
cat backup-discord_sentinel-*.sql | docker exec -i sentinel-postgres \
  psql -U sentinel -d discord_sentinel
```

---

## 📋 Résumé des changements appliqués (commit `0e91486`)

| Optimisation | Statut | Notes |
|---|:---:|---|
| #1 Consolidation 15 Dockerfiles Rust | ✅ | `Dockerfile.rust-alpine` + `Dockerfile.rust-debian` |
| #2 Compose YAML anchors | ✅ | -325 lignes de duplication |
| #3 Log rotation 10m × 3 | ✅ | Sur tous les services |
| #4 .dockerignore nettoyé | ✅ | Refs mortes retirées, coverage-html ajouté |
| #5 Image base Rust pré-buildée | ❌ | Pas fait — la consolidation #1 + cache BuildKit suffit en pratique |
| #6 Planner cargo-chef optimisé | ❌ | Approche conservatrice gardée (`COPY . .`), à itérer si rebuilds trop lents |
| #7 User non-root (uid 1000) | ✅ | Sur les deux Dockerfiles génériques |
| #8 `init: true` (tini PID 1) | ✅ | Tous les services Rust |
| #9 Healthchecks workers `/metrics` | ✅ | Plus fiable que pidof |
| #10 docker-bake.hcl | ✅ | `infrastructure/docker/docker-bake.hcl` |
| #11 Distroless API | ❌ | **Bloqué** : ONNX Runtime requiert glibc ≥ 2.38, distroless cc-debian12 = 2.36. Alternative `cgr.dev/chainguard/glibc-dynamic` à explorer si vraiment besoin de gain d'image size |
| #12 Pin versions images base | ⚠️ | `latest-rust-alpine` et `latest-rust-slim-trixie` toujours en place ; `CHEF_IMAGE` est une build-arg donc surchargeable. À pinner sur un tag concret après validation |
| #13 `sharing=private` cache | ✅ | Permet builds parallels (`docker buildx bake`) |

---

## 📚 Références

- [Compose Spec — `name`](https://docs.docker.com/compose/compose-file/04-version-and-name/#name-top-level-element)
- [Compose Spec — `init`](https://docs.docker.com/compose/compose-file/05-services/#init)
- [BuildKit cache mounts](https://docs.docker.com/build/cache/backends/)
- [docker buildx bake](https://docs.docker.com/build/bake/)
- [cargo-chef](https://github.com/LukeMathWalker/cargo-chef)
