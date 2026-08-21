# DiscordSentinel

Monorepo Rust hébergeant quatre contextes fonctionnels qui partagent la même infrastructure, le même dashboard web et un cœur métier unifié :

- **Sentinel** — modération, sécurité et animation de serveurs Discord (bot unifié + API + worker + gateway).
- **Nexus** — plateforme de serveurs de jeux, économie et animations.
- **Atrium** — accueil et assistance IA.
- **Ops** — supervision et sécurité de l'hôte.

Architecture **hexagonale** : `platform-core` contient les contextes `sentinel`, `nexus`, `atrium` et `ops`. Les crates `*-api`, `*-bot`, `*-gateway`, `platform-scheduler` et les agents sont des adaptateurs ou processus spécialisés.

---

## Vue d'ensemble du workspace

```
DiscordSentinel/
├── platform-common/     # Socle partagé — bus d'événements Redis Streams (aucun framework)
├── platform-core/       # Cœur hexagonal unifié — atrium / nexus / sentinel / ops
├── platform-scheduler/  # Planificateur HTTP thin commun
│
├── sentinel-api/        # API Axum 0.8 — adapters inbound (HTTP + gRPC) & outbound (Postgres, Redis, Discord)
├── sentinel-bot/        # Bot Discord unifié (Serenity 0.12) — 25 modules, un seul process
├── platform-gateway/    # Relay Redis Streams → WebSocket
├── sentinel-proto/      # Définitions gRPC (tonic + prost)
│
├── nexus-api/           # API HTTP Nexus (axum) + runtime Docker des serveurs de jeux
├── nexus-bot/           # Bot Discord Nexus (portail de jeux, roue, coussin)
├── nexus-proto/         # Protos gRPC Nexus (stub)
├── atrium-api/ / atrium-bot/ / atrium-proto/
├── ops-api/ / ops-agent/ / docker-agent/
│
├── web/                 # Dashboard Vue 3 + TS + Vite + Pinia (partagé Sentinel/Nexus, multi-marque)
├── infrastructure/      # docker/ (compose, bake, Dockerfiles), grafana/, prometheus/, scripts/
├── platform-ml/         # Configs d'entraînement YAML + points de montage des exports ONNX
├── persona/             # 17 fiches de personas markdown (utilisées par la skill /party-mode)
├── DOC/                 # Documentation fonctionnelle, technique et référence IA
└── Cargo.toml           # Workspace Rust
```

### Poids relatif des composants

| Crate / dossier | Fichiers | Lignes | Rôle |
|---|---:|---:|---|
| `sentinel-api` | 837 | ~93 k | 21 fichiers de routes, 108 handlers, adapters Postgres/Redis |
| `sentinel-bot` | 202 | ~48 k | 25 modules Discord |
| `platform-core` | 608 | — | domaines Atrium, Nexus, Sentinel et Ops |
| `web` | 410 | ~44 k | dashboard Vue 3 (atomic design) |
| `nexus-api` | 77 | ~7,4 k | HTTP + `game_runtime` (bollard) |
| `nexus-bot` | 7 | ~3,8 k | portail de jeux, roue, coussin |
| `platform-gateway` | 7 | ~0,8 k | broadcaster WebSocket transverse |
| `platform-scheduler` | — | — | déclenchement HTTP des jobs de tous les contextes |

---

## Architecture globale

**Deux stacks symétriques**, chacune avec son bot, son API, son cœur métier et son worker. Elles ne se parlent pas directement : tout ce qu'elles partagent est en bas du schéma (base, cache, dashboard, crates socles).

```
                          Discord (messages · events · images · slash commands)
                                    │                              │
        ── SENTINEL ────────────────┼──────────    ── NEXUS ───────┼────────────
                                    ▼                              ▼
        ┌───────────────────────────────────────┐   ┌────────────────────────────┐
        │ sentinel-bot (Serenity 0.12)          │   │ nexus-bot (Serenity 0.12)  │
        │ 25 modules, un seul process           │   │ portail de jeux · roue ·   │
        │ automod · moderation · security ·     │   │ coussin · casino           │
        │ guild_backup · tickets · voice · …    │   │                            │
        └──────┬─────────────────────────┬──────┘   └──────┬──────────────┬──────┘
               │ gRPC (tonic)            │ Redis           │ HTTP         │ Redis
               │ + HTTP résiduel         │ sentinel:events │ (reqwest)    │ nexus:events
               ▼                         │                 ▼              │
        ┌────────────────────────┐       │          ┌────────────────────────────┐
        │ sentinel-api (Axum 0.8)│       │          │ nexus-api (Axum 0.8)       │
        │ inbound  http + grpc   │       │          │ inbound  http              │
        │ outbound pg · redis ·  │       │          │ outbound pg · redis ·      │
        │          discord · ws  │       │          │          game_runtime      │
        │ inférence ONNX         │       │          │ (bollard → Docker)         │
        └───────────┬────────────┘       │          └───────────┬────────────────┘
                    │ délègue à          │                      │ délègue à
                    ▼                    │                      ▼
        ┌────────────────────────┐       │          ┌────────────────────────────┐
        │ sentinel-core   (pur)  │       │          │ nexus-core      (pur)      │
        │ domain / application / │       │          │ domain / application /     │
        │ ports — zéro infra     │       │          │ ports — zéro infra         │
        └────────────────────────┘       │          └────────────────────────────┘
                                         │
        ┌────────────────────────┐       │          ┌────────────────────────────┐
        │ sentinel-worker        │       │          │ nexus-worker               │
        │ 16 domaines périodiques│       │          │ jobs serveurs de jeux      │
        └────────────────────────┘       │          └────────────────────────────┘
                                         │
        ┌────────────────────────┐       │          ┌────────────────────────────┐
        │ platform-gateway       │◄──────┘          │ aucun gateway Nexus requis │
        │ XREAD $ → WebSocket    │                  │ (ancien stub supprimé)       │
        └───────────┬────────────┘                  └────────────────────────────┘
                    │
        ════════════╪═══════════ PARTAGÉ ══════════════════════════════════
                    ▼
   ┌────────────────────────┐  ┌───────────────┐  ┌───────────────┐
   │ web (Vue 3 + Pinia)    │  │ PostgreSQL 16 │  │ Redis 7       │
   │ dashboard multi-marque │  │ + PgBouncer   │  │ cache +       │
   │ OAuth2 Discord + WS    │  │               │  │ Streams       │
   └────────────────────────┘  └───────────────┘  └───────────────┘

   ┌──────────────────────────────────────────────────────────────────┐
   │ platform-common      bus d'événements Redis Streams (sans infra) │
   │ platform-api::shared rate limit · métriques · CORS · en-têtes    │
   └──────────────────────────────────────────────────────────────────┘
```

**Philosophie** : `platform-core::<entité>` = règles métier · API = adaptateurs + IA + persistance · Bot = interface Discord légère · Gateway = temps réel découplé · Scheduler = minuteries HTTP sans métier · Web = administration.

**Asymétries réelles, à ne pas lire comme des oublis du schéma** :

- **`sentinel-proto` est complet, `nexus-proto` est un stub vide** (7 lignes, aucun `.proto`). Seule la stack Sentinel parle gRPC ; `nexus-bot` appelle son API en HTTP via `reqwest`.
- **`platform-gateway` est l'unique gateway réseau.** Nexus n'avait qu'un stub sans listener, désormais supprimé.
- **Le transport `sentinel-bot` → `sentinel-api` est majoritairement gRPC**, mais pas exclusivement : les modules sans service proto passent encore par HTTP. Le détail est dans [RESTE-A-FAIRE.md](RESTE-A-FAIRE.md).
- **Le bot n'a jamais d'accès direct à la base.** Ni Sentinel ni Nexus : c'est l'API qui persiste, dans les deux stacks.

---

## Stack technique

| Composant | Technologie | Détails |
|---|---|---|
| Socles partagés | Rust pur / axum | `platform-common` (sans framework) et `platform-api::shared` (middlewares HTTP) |
| Cœur métier | Rust pur | `platform-core/src/{sentinel,nexus,atrium,ops}` avec `domain`, `application` et `ports` |
| API backend | Rust / Axum 0.8 / Tokio / sqlx 0.8 | `sentinel-api` réduit aux adapters : `adapters/inbound/{http,grpc}`, `adapters/outbound/{postgres,ws,audit,batching,host_security,system}`, `bootstrap/` |
| Bot Discord | Rust / Serenity 0.12 | Process unique, 25 modules chargés selon la config per-guild ; helpers dans `src/shared/` (api_client, circuit_breaker, event_bus, grpc_client, shard_launcher, …) |
| Scheduler | Rust / Tokio / HTTP | `platform-scheduler`, sans accès métier direct ni base |
| Gateway | Rust / Axum / Redis | Relay `XREAD $` → WebSocket, auto-reconnect exponential backoff |
| gRPC | `tonic` 0.13 + `prost` 0.13 | Crate `sentinel-proto`, serveur dans `adapters/inbound/grpc/` |
| PostgreSQL | Postgres 16 + **PgBouncer** | 17 migrations Sentinel + 24 migrations Nexus, partitionnement RANGE mensuel, vues matérialisées |
| Cache / Bus | Redis 7 | `maxmemory=2gb allkeys-lru`, **Redis Streams** (`sentinel:events`, consumer groups durables) |
| Inférence IA | ONNX Runtime (`ort` 2.0-rc) / ndarray / tokenizers | Vision (NSFW/illicite) + Text (sentiments multilingues) |
| Serveurs de jeux | `bollard` 0.18 (Docker API) | `nexus-api/adapters/outbound/game_runtime` — provisioning conteneurisé |
| Middlewares HTTP | `tower-http` 0.6 | Pile identique sur les deux APIs : CORS, en-têtes de sécurité, trace + request-id, limite de corps, compression zstd/gzip, rate limit par IP, métriques |
| Web dashboard | Vue 3 + TS + Vite + Pinia + Chart.js | `web/` — atomic design (`atoms/molecules/organisms/templates/pages`), servi par Nginx |
| Observabilité | Prometheus + Grafana + tokio-metrics | Middleware Axum metrics, dashboards provisionnés |
| Containerisation | Docker Alpine multi-stage + Compose + Bake | `infrastructure/docker/` |

Profil release partagé : `lto = "thin"`, `codegen-units = 16`, `strip = true`. Deps communes déclarées une seule fois dans le `Cargo.toml` racine (`[workspace.dependencies]`), lints clippy partagés via `[workspace.lints]`.

---

## Sentinel — modules du bot (25)

Chaque module est activable/configurable par serveur (table `bot_guild_config`, schéma de formulaire dans `bot_definitions.config_schema`, éditable depuis le dashboard web). Chaque commande slash est filtrée par module activé **et** par permission Discord.

### 🤖 automod — Modération automatique + vote des modérateurs

Analyse chaque message (texte + images) : détecteurs locaux (spam, insulte, juron, lien, phishing, caps, flood, emoji, mentions, unicode, fichiers suspects) **+** IA ONNX (texte multilingue, vision NSFW/illicite). Chaque flag a un **poids** ; la somme donne un **score** comparé aux **seuils** (warn / delete / mute / ban).

Selon le score et la config, le message est traité **automatiquement** (warn / delete / mute ; le ban n'est jamais automatique) **ou** déclenche une **carte de review/vote** dans le salon de review.

**Système de vote** (`vote_enabled`) : boutons **Warn / Delete / Mute / Ban / Ignorer**. À l'échéance (`vote_deadline_hours`) le domaine `automod` du worker dépouille (quorum + tie-break) ; un **administrateur finalise** (seule voie d'un ban réel).

- **Regroupement par utilisateur** (`vote_aggregate_enabled`) : les nouveaux signalements s'agrègent dans la carte ouverte.
- **Salon de discussion** (`discussion_channel_enabled`) : bouton créant un salon privé membre + modo sous une catégorie configurable.

| Commande | Permission |
|---|---|
| `/automod status` · `/automod test` | Gérer le serveur |

### ⚖️ moderation — Modération manuelle

Sanctions (`/warn`, `/unwarn`, `/mute`, `/unmute`, `/ban`, `/unban`, `/massmute`, `/massban`), dossiers (`/history`, `/note`, `/context`, `/evidence`, `/expirations`, `/compare`, `/call`), outils (`/appeal`, `/review`, `/template`, `/transcript`, `/export`, `/modstats`).

- **`/card`** — crée manuellement une carte de vote quand une détection est passée au travers (ciblage par lien Discord cross-salon ou par ID).
- **`/context`** — affiche les messages autour d'un message.
- Côté core : sursis, strikes/escalade, copilote de modération, évaluation du risque d'une cible.

### 🔐 security — Anti-raid / alt accounts

Détection de raids (pics de joins), comptes récents/alt, captcha, quarantaine, lockdown, slowmode adaptatif, bans IP, GeoIP. `/security status`, `/security history`.

### 💾 guild_backup — Sauvegarde / restauration de serveur

Capture la structure complète d'un serveur (rôles, catégories, salons + overwrites, réglages, bans, emojis, rôles par membre) vers l'API, et la restaure via Serenity avec **remapping d'IDs**. Action massive/destructive : réservée à l'**owner** ou à un Administrateur, avec confirmation par bouton. Domaine worker dédié (`guild_backup`) + service core `manage_snapshots_service`.

### 💡 ideas — Boîte à idées

Panneau public → bouton « Proposer une idée » → catégorie → modale (titre + description) → salon privé auteur + staff, avec carte de décision réservée au staff. Messages synchronisés vers l'API pour relecture depuis le web (event Redis `idea_decided`).

### 🎫 tickets — Support

`/ticket close` (membre), `/ticket-admin panel|invite` (staff). SLA (domaine worker `appeal_sla`), fermeture sur inactivité, transcripts.

### 🔎 audit — Journal d'audit Discord

`/audit search`, `/audit stats`. Ingestion des audit-logs (domaine worker `discord_audit_sync`), surveillance d'utilisateurs, rapports hebdomadaires, détection d'anomalies de modération.

### 🎉 welcome — Accueil & onboarding

Messages de bienvenue / départ / retour (rich embeds), validation du règlement (bouton → rôle), anniversaires d'arrivée, **compteur de membres** et **compteur de présence vocale** (salons renommés).

### 🔊 voice — Salons vocaux temporaires

Création à la volée, panneau de contrôle (renommer, lock, limite, visibilité, kick, ban, co-admins, transfert, file d'attente, vote-kick), thèmes réutilisables, cleanup auto.

- **Whitelist persistante par propriétaire**, réappliquée à chaque création.
- **Preset de paramètres par propriétaire** (« Sauvegarder params ») + mode **caché-sauf-whitelist**.

### 👥 community — Rôles, parrainage, vie du serveur

Panels de rôles auto-assignables (`/roles-panel`), groupes exclusifs, auto-rôles, **rôles temporaires**, parrainage (`/parrain`). Côté core : événements, sondages, LFG, news, spotlight, classement mensuel, éligibilité, déclaration d'âge.

### 📈 progression — Niveaux & XP

XP par activité (messages, vocal, ancienneté), rôles de niveau et paliers. `/level user|top`, `/stats`, `/progression-resync`.

### 🧹 cleanup — Nettoyage

`/purge last|user|contains`, `/cleanup logs|infractions|audit`, autopurge planifiée par salon.

### 📣 announcements · 🖼️ embeds · ✉️ messages — Publication depuis le web

Trois consumers Redis Streams complémentaires :
- **announcements** — annonces planifiées, publiées par le worker puis postées par le bot (résultat rapporté à l'API).
- **embeds** — builder d'embeds style Carl-bot : poste **ou édite** une carte selon `message_id`, puis rapporte `(channel_id, message_id)`.
- **messages** — pendant dépouillé : poste du markdown brut, sans carte, quand le message doit ressembler à celui d'un membre.

### 🛰️ presence — Présence en direct (page membre publique)

Publie qui est dans quel salon vocal, **uniquement** pour les salons visibles par `@everyone` — filtre **fermant** : en cas de doute (salon absent du cache, guilde inconnue), rien n'est publié. Seul le bot peut trancher, l'API n'ayant pas de vue sur les permissions Discord.

### 🌌 nasa_apod — Astronomy Picture of the Day

Publie chaque jour l'APOD de la NASA dans un salon configuré, traduite en français via DeepL si une clé est fournie (repli en anglais sinon). Idempotent : ne republie pas si la photo du jour est déjà présente.

### 🤫 confessions · ⬆️ bump · 💬 command_channel · 🧠 ai_dataset · 😀 emoji · ❓ help_panel · 🗂️ logs_setup

- **confessions** — `/confess` (anonyme), `/confess-admin deploy-panel|delete|reveal`.
- **bump** — détecte un `/bump` Disboard réussi (providers configurables), récompense en coins selon le nombre de bumps de la semaine, rappelle après cooldown.
- **command_channel** — supprime en silence les messages texte classiques dans les salons « commandes uniquement » (owner et bots épargnés).
- **ai_dataset** — désactivé par défaut ; alimente `ai_dataset_messages` pour l'étiquetage manuel et l'export CSV depuis le web.
- **emoji** / **help_panel** / **logs_setup** — utilitaires : emojis du serveur, panneau d'aide (off par défaut), configuration guidée des salons de logs.

---

## Nexus — plateforme de jeux

Le contexte `platform-core/src/nexus` suit la structure hexagonale commune (`domain`, `application`, `ports`).

### Serveurs de jeux (`application/game`)

Provisioning de serveurs de jeux dédiés en conteneurs Docker, piloté depuis le dashboard :

- **Templates** (`manage_templates_service`) — catalogue de jeux (7 Days to Die, Valheim, Palworld, …), covers, réglages exposés, mods/plugins, limites CPU/RAM.
- **Serveurs** (`manage_game_servers_service`) — cycle de vie, sessions, événements de session, jobs de fond, génération de mots de passe, chargement de config.
- **Runtime** (`nexus-api/adapters/outbound/game_runtime`) — pilotage Docker via `bollard`.
- Routes : `servers`, `public_servers`, `templates`, `sessions`, `session_events`, `jobs`, `games`.

### Casino & économie (`domain/entities/casino`)

`wallet`, `wheel` (roue du destin — voir [`docs/roue-du-destin.md`](docs/roue-du-destin.md)), `coussin` / `coussin_shop` (voir `COUSSIN_PIEGE.md`). Le bot Nexus expose le portail de jeux (`game_portal.rs`), la roue (`wheel_panel.rs`) et le catalogue (`games.rs`).

> Historique : les modules de jeux (blackjack, slot, wheel, games, tamagotchi) vivaient auparavant dans `sentinel-bot`. Ils ont été extraits vers la stack Nexus.

---

## Base de données

**PostgreSQL 16** derrière **PgBouncer** (transaction pooling).

- **Sentinel** — `sentinel-api/migrations/` : `001_init.sql` (base vierge requise, historique archivé dans `migrations_legacy/`) puis 16 migrations incrémentales (community events, paliers de rôles, embeds builder, idées, autopurge par salon, scoring automod, …).
- **Nexus** — `nexus-api/migrations/` : 24 migrations (wallet, coussin, game portal, templates, mods/plugins, économie, roue configurable, …).

### Optimisations structurelles

- **Vues matérialisées** (`mv_wallet_leaderboard`, `mv_level_leaderboard`) refreshées toutes les 5 min par le domaine `cache` du worker.
- **Partitionnement RANGE mensuel** sur `infractions`, `audit_logs`, `user_activity_log`, `logs` — génération automatique M+1/M+2 par le domaine `cache`.
- **Enums Postgres** (`moderation_gravity`, `voice_channel_kind`) mappés en Rust via `#[derive(sqlx::Type)]`.
- **Index GIN** sur `infractions.flags`, `security_events.user_ids`, `bot_definitions.config_schema` ; partials soft-delete sur `voice_channels` et `tickets` ; Discord IDs en `VARCHAR(20)`.
- **`user_cache`** : source de vérité des usernames Discord, agrégée périodiquement.
- **`ai_jobs`** : file d'attente asynchrone pour l'inférence IA.

### Tables principales (extrait)

| Table | Description |
|---|---|
| `rules` / `infractions` | Règles de modération + infractions (flags JSONB + GIN) |
| `moderation_actions` | Historique modération manuelle (enum `moderation_gravity`) |
| `automod_reviews` / `automod_review_votes` | Cartes de review + votes (incidents agrégés, score cumulé) |
| `automod_discussion_channels` | Salons de discussion liés à une review |
| `tickets` / `ticket_messages` | Tickets support + SLA |
| `security_events` | Détection raid / alt accounts |
| `audit_logs` **(partitionné)** | Audit-logs Discord ingérés |
| `user_activity_log` / `logs` **(partitionnés)** | Activité utilisateur, logs applicatifs |
| `user_stats` / `user_levels` / `user_wallets` | Stats, XP, wallets (vues matérialisées) |
| `voice_channels` + sub-tables | Salons vocaux (enum `voice_channel_kind`), whitelists, bans, invites, presets |
| `bot_guild_config` / `bot_definitions` | Config per-guild + schéma de config par module |
| `sanction_reminders` / `temp_roles` | Rappels d'expiration + rôles temporaires |
| `ai_jobs` / `ai_dataset_messages` | Queue IA async + dataset collecté |
| `welcome_config` | Config bienvenue + rich embeds |

### Flag types supportés

| Type | Source | Poids défaut |
|---|---|---|
| `spam` / `insult` / `link` / `phishing` | Détecteurs automod | 3.0 / 5.0 / 1.0 / 7.0 |
| `nsfw` / `illicit` | IA Vision ONNX | 8.0 / 9.0 |
| `anger` / `rage` / `threat` / `harassment` | IA Text ONNX | 3.0 / 6.0 / 8.0 / 7.0 |

---

## API Sentinel

**Authentification** : `Authorization: Bearer <API_KEY>` obligatoire (sauf `/health` et `/metrics`). Le middleware `guild_auth_middleware` filtre en plus par `X-Discord-Token` si présent (multi-tenant OAuth2). **108 handlers** répartis sur **21 fichiers de routes**.

| Fichier de routes | Domaine couvert |
|---|---|
| `analytics.rs` · `dashboard.rs` · `stats.rs` | Heatmaps, trends, KPIs, top infractors |
| `automod.rs` | Règles, scoring, reviews & votes |
| `moderation.rs` | Infractions, strikes, sursis, notes, rappels, purges |
| `security.rs` | Events de détection, quarantaine, lockdown, IP bans |
| `audit.rs` | Audit-logs, utilisateurs surveillés, rapports |
| `ticket.rs` · `idea.rs` | Support + boîte à idées |
| `community.rs` | Panels de rôles, événements, sondages, LFG, news, spotlight, parrainage |
| `progression.rs` | XP, niveaux, paliers |
| `voice_channels.rs` | Salons vocaux (presets, whitelists, bans, invites, thèmes) |
| `welcome` (via `community`) | Config bienvenue + embeds |
| `guild_backup.rs` · `guild_structure.rs` | Snapshots de serveur, structure Discord |
| `members.rs` | Membres, guilds, salons, présence |
| `bump.rs` | Récompenses Disboard |
| `bot.rs` · `bot_persistence.rs` | Config per-guild, définitions de modules, persistance bot |
| `system.rs` | Health, metrics, exports, OAuth, logs, alertes, GeoIP, TLS, host probe |

**gRPC** : serveur `tonic` dans `adapters/inbound/grpc/` (crate `sentinel-proto`).

### Inférence IA (ONNX)

| | Vision | Text |
|---|---|---|
| Architecture | EfficientNetV2-S | DistilBERT multilingual |
| Classes | `safe`, `nsfw`, `illicit` | `neutral`, `anger`, `rage`, `threat`, `harassment` |
| Input | 224×224 normalisé ImageNet | Tokens (max 256) + attention mask |
| Format | ONNX (opset 17) | ONNX + tokenizer HuggingFace (Rust) |

Modèles chargés au démarrage de l'API, **mode dégradé** automatique s'ils sont absents (scoring règles seulement). Configs d'entraînement dans `platform-ml/{text,vision}/configs/`, exports attendus dans `platform-ml/{text,vision}/exports/` (montés en `/models/*`). Le pipeline d'entraînement est externe au repo.

**Mode async** : `POST /api/ai/jobs` retourne `202 Accepted` + `job_id` ; le domaine `ai` du worker dépile la file et publie sur Redis `ai_result:{job_id}` (TTL 600 s). Alternative au `POST /analyze` synchrone (timeout 5 s côté bot).

**Config IA per-guild** : centralisée dans `bot_guild_config` (bot `automod-bot`) — `text_enabled`, `text_threshold`, `vision_enabled`, `vision_threshold`, `context_dampening`, `context_format`, `context_max_messages`, `context_max_chars`, `channel_tension_*`, et tout le bloc `vote_*` / `discussion_*`.

### Middleware (ordre de traversée)

```
Request
  → CORS
  → SetRequestId + TraceLayer
  → BodyLimit (10 MB par défaut)
  → CompressionLayer (zstd + gzip)
  → metrics_middleware (Prometheus)
  → api_logger (→ table logs)
  → [si route protégée]
      → rate_limit (token bucket IP)
      → auth (Bearer API key)
      → guild_auth (X-Discord-Token)
  → Handler
```

**Rate limit inférence ONNX** : semaphore (`INFERENCE_MAX_CONCURRENT=4`) + token bucket (`INFERENCE_MAX_PER_SEC=20`). HTTP 429 si dépassement.

### Multi-tenant

1. Le web fait OAuth2 Discord (scopes `identify` + `guilds`) → `access_token`.
2. Le client envoie ce token dans `X-Discord-Token` sur toutes les requêtes.
3. `guild_auth_middleware` extrait le `guild_id` de l'URI, interroge `/users/@me/guilds` (cache Redis 5 min par hash de token), et refuse `403` si le guild n'est pas autorisé.
4. Si `X-Discord-Token` est absent (appel bot/worker interne) le middleware est **pass-through** — `auth_middleware` (Bearer) reste obligatoire.

---

## Bus d'événements (Redis Streams)

**Une stream par plateforme** — `sentinel:events` et `nexus:events` — jamais partagée : un event Nexus n'a rien à faire dans les consumer groups de Sentinel. Les deux suivent le même contrat, porté par `EventBus` (`platform-common`), paramétré par la clé de stream.

Tous les producers (API, workers) publient en `XADD MAXLEN ~ 10000`. Format d'entrée : un champ `payload` = `{"event": ..., "data": ...}`.

- **Consumers durables** (bot) : `XREADGROUP` + `XACK` (at-least-once), un consumer group par feature, auto-claim `XAUTOCLAIM` des pending > 60 s après un crash.
- **Consumers live-tail** (gateway) : `XREAD $` sans group → relay WebSocket.

**Events** : `infraction_new`, `ticket_*`, `idea_decided`, `security_event`, `moderation_action`, `voice_channel_updated/closed`, `announcement_publish`, `embed_publish`, `message_send`, `sanction_expiry_reminder`, `temp_role_expire`, `bot_log`, etc.

### Gateway WebSocket

Le relay est porté par l'unique processus `platform-gateway`.

| Propriété | Valeur |
|---|---|
| Port | 3001 |
| Auth | `?token=<api_key>` |
| Max connexions | 1000 (configurable) |
| Reconnexion Redis | Exponential backoff |
| Healthcheck | `GET /health` |

---

## Dashboard web

`web/` — Vue 3 + TypeScript + Vite + Pinia + vue-router + Chart.js + `@vueuse/motion`, servi par Nginx.

```
web/src/
├── components/{atoms,molecules,organisms,templates,pages}   # atomic design
├── api/          # http.ts, nexusHttp.ts (les deux backends), events, store, config
├── stores/       # Pinia
├── composables/ · services/ · utils/ · types/ · data/ · styles/
├── games/catalog.ts
├── branding.ts · siteConfig.ts · entrySpace.ts   # thématisation multi-marque
└── router/
```

Le dashboard adresse **les deux APIs** (Sentinel via `http.ts`, Nexus via `nexusHttp.ts`).

---

## Observabilité

- **Prometheus** — `/metrics` sur `sentinel-api` (port 3000), `nexus-api` (port 3100) et le worker (port 9100). Les deux APIs exposent les **mêmes noms de métriques**, donc les dashboards Grafana se réutilisent en filtrant sur le label `service`. Compteurs `http_requests_total{route,method,status}`, histogrammes `http_request_duration_seconds`, gauges `tokio_busy_ratio`, `tokio_live_tasks_count`, `tokio_global_queue_depth`.
- **Grafana** — dashboards auto-provisionnés dans `infrastructure/grafana/`. UI sur `http://localhost:3002` (admin/admin).
- **pg_stat_statements** — `SELECT * FROM pg_stat_statements ORDER BY total_exec_time DESC`.
- **Tracing structuré** — `tracing-subscriber` JSON en prod, correlation IDs `X-Request-ID` via `tower_http::request_id`.

---

## Déploiement

### Docker Compose

```bash
# Stack complète, tous les services sont standards
docker compose -f infrastructure/docker/docker-compose.yml up -d
```

**Services** : infrastructure PostgreSQL/Redis, `api`, `gateway`,
`platform-scheduler`, les trois bots, `web`, `certbot`, `pgadmin`,
`redis-commander`, `prometheus` et `grafana`.

Images construites depuis `infrastructure/docker/Dockerfile.rust-alpine` (ou `.rust-debian`), orchestration multi-images via `docker-bake.hcl`.

### Variables d'environnement (.env)

```env
# Infrastructure
POSTGRES_PASSWORD=sentinel_secret
REDIS_PASSWORD=sentinel_redis

# API
API_KEY=your_api_key_here
REQUIRE_API_KEY=true

# IA / Inference ONNX
VISION_MODEL_PATH=/models/vision/vision_sentinel.onnx
TEXT_MODEL_PATH=/models/text/text_sentinel.onnx
TEXT_TOKENIZER_PATH=/models/text/tokenizer.json
TEXT_MAX_LENGTH=256

# Bot Discord unifié (réutilisé par l'API et discord_audit_sync)
SENTINEL_DISCORD_TOKEN=...

# Module security (seuils & toggles globaux, lus au démarrage du bot)
RAID_JOIN_THRESHOLD=10
MIN_ACCOUNT_AGE_SECS=86400
QUARANTINE_ENABLED=false
CAPTCHA_ENABLED=false
LOCKDOWN_ENABLED=false
ALT_DETECTION_ENABLED=false

# Module voice (fallbacks ; la config par serveur prime via le dashboard)
VOICE_GUILD_ID=...
VOICE_PUBLIC_CREATOR_CHANNEL_ID=...
VOICE_PRIVATE_CREATOR_CHANNEL_ID=...
VOICE_LOG_CHANNEL_ID=...

# Module nasa_apod
NASA_API_KEY=...
DEEPL_API_KEY=...          # optionnel : sans clé, publication en anglais

# Économie (fallback pour le solde de départ des wallets)
WALLET_STARTING_COINS=100

# Nexus
NEXUS_API_PORT=3100
NEXUS_API_KEY=...            # Bearer exigé sur /api/* ; absent = API ouverte (dev)
NEXUS_DISCORD_TOKEN=...      # sans cette variable, nexus-bot ne se connecte pas
NEXUS_METRICS_TOKEN=         # optionnel : protège /metrics
NEXUS_ALLOWED_ORIGINS=       # CORS ; vide = origines de dev uniquement
NEXUS_MAX_BODY_SIZE=10485760
NEXUS_RATE_LIMIT_PER_SEC=50        # routes de lecture
NEXUS_HEAVY_RATE_LIMIT_PER_SEC=2   # routes qui lancent un conteneur
TRUST_PROXY_HOPS=1                 # proxies de confiance devant l'API
```

> **Note** — La plupart des réglages sont désormais éditables **par serveur** depuis le dashboard (`bot_guild_config` + `config_schema`). Les variables d'env restent des **fallbacks** ou des réglages **globaux lus au démarrage** (un changement impose alors un redémarrage).

### Développement local

```bash
bash infrastructure/scripts/dev.sh              # Lance API + bot + web
bash infrastructure/scripts/build-all.sh        # Build release de tous les crates
bash infrastructure/scripts/start-all.sh        # Démarre la stack complète
bash infrastructure/scripts/health-check.sh     # Vérifie que tous les services répondent
bash infrastructure/scripts/seed-rules.sh       # Seed de règles de dev
bash infrastructure/scripts/setup-host-security.sh
bash infrastructure/scripts/tls-issue.sh        # Émission des certificats

# Ou composant par composant :
cd sentinel-api && cargo run
cd sentinel-worker && cargo run
cd sentinel-bot && cargo run
cd web && npm run dev
```

---

## Tests

```bash
cargo test --workspace                          # Rust
bash infrastructure/scripts/run-tests.sh        # (ou run-tests.ps1 sous Windows)
cd web && npm run test                          # Vitest
```

Couverture principale : services applicatifs et value objects de `platform-core`, middlewares et repositories des APIs, modules des bots, scheduler et agents. Stack de tests dédiée : `infrastructure/docker/docker-compose.test.yml`. CI : `.github/workflows/ci.yml`.

### Mesurer la couverture

```powershell
.\scripts\coverage.ps1            # Rust + web, résumé au terminal
.\scripts\coverage.ps1 -Html      # + rapports détaillés, ouverts dans le navigateur
.\scripts\coverage.ps1 -Rust      # un seul des deux
.\scripts\coverage.ps1 -Seuil 40  # échoue sous 40 % de lignes couvertes
```

Sous Linux, macOS ou Git Bash : `./scripts/coverage.sh` avec `--html`, `--rust`, `--web`, `--seuil 40`.

Première utilisation, côté Rust :

```powershell
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

`cargo-llvm-cov` plutôt que `tarpaulin` : il s'appuie sur l'instrumentation de LLVM, identique sur les trois systèmes, alors que tarpaulin s'accommode mal de Windows — le chiffre obtenu localement est donc celui de la CI.

En local, la mesure Rust porte sur `--lib --bins`. Les tests d'intégration qui exigent PostgreSQL sont écartés : sans `DATABASE_URL` ils échouent, et un échec d'environnement ferait passer la couverture pour un problème de code. La CI, elle, dispose d'une base et les inclut — son chiffre est donc plus élevé, et c'est normal.

Le job `coverage` de la CI conserve les rapports en artefacts **même quand une suite échoue** : c'est souvent quand ça casse qu'on veut voir ce qui n'était pas couvert. Il ne bloque pas la fusion — mesurer n'est pas valider, et c'est `rust-check` qui juge.

La mesure se fait en deux temps (`cargo llvm-cov --no-report`, puis `cargo llvm-cov report`). En une seule commande, un test rouge fait sortir cargo en erreur **avant** la génération du rapport — exactement le moment où l'on en a besoin.

Point de départ mesuré le 21 août 2026 : **17,4 % de lignes** côté Rust (hors tests PostgreSQL) et **6,3 %** côté web. Aucun seuil n'est imposé en CI pour l'instant : un seuil posé au-dessus du réel bloquerait toutes les fusions dès le lendemain. `-Seuil` / `--seuil` existe pour en fixer un quand la couverture aura été remontée là où elle compte.

Les tests web tournent sur le même stockage que le navigateur quelle que soit la version de Node : `src/test/setup.ts` réinstalle `localStorage` et `sessionStorage` du DOM sur `globalThis`. Depuis Node 24, la plateforme en expose une version native qui masque celle de happy-dom et refuse de fonctionner sans `--localstorage-file` — d'où des suites vertes en CI (Node 22) et rouges sur une machine à jour. Un test ne doit pas raconter une histoire différente selon la machine qui le lance.

---

## Bonnes pratiques du projet

- **Le métier vit dans `platform-core::<entité>`** — APIs, bots, scheduler et agents sont des adaptateurs ; aucune règle métier ne doit y être écrite.
- **Architecture hexagonale stricte** — `domain` ne dépend de rien, `application` ne dépend que des `ports`, les adapters implémentent les ports.
- **Bot = interface légère** — décisions et persistance côté API, jamais dans les modules du bot.
- **Workers = jobs périodiques DB-bound** — via `spawn_periodic` + Redis Streams, pas de gateway Discord (exception : `discord_audit_sync`).
- **Gateway découplé** — absorbe les bursts WebSocket indépendamment de l'API.
- **Inférence IA gracieuse** — si les modèles sont absents, repli sur le scoring par règles.
- **Multi-tenant** — filtre `guild_auth` avec pass-through pour les appels internes.
- **Config par serveur d'abord** — `config_schema` + `bot_guild_config` éditables sur le web ; les variables d'env ne sont que des fallbacks.
- **Filtres fermants sur les données publiques** — en cas de doute sur une permission Discord, on ne publie pas (cf. module `presence`).
- **Observabilité first** — métriques Prometheus, traces JSON, `pg_stat_statements`.
- **Workspace partagé** — deps et lints déclarés une seule fois à la racine, `lto = "thin"` pour garder les builds rapides.
