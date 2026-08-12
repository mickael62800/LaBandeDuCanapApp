# CLAUDE.md

Repères pour travailler dans ce dépôt. Le [README](README.md) décrit le produit ; ce fichier décrit **comment coder ici**.

## Le dépôt en une phrase

Monorepo Rust : **trois plateformes hexagonales** — `sentinel-*` (modération Discord), `nexus-*` (serveurs de jeux + casino), `atrium-*` (parcours d'accueil assisté par IA) — partageant `web/` (Vue 3), `infrastructure/` (Docker/Grafana/Prometheus), Postgres et Redis.

Chaque plateforme suit le même découpage : `-core` (métier), `-api`, `-bot`, `-worker`, `-proto`, et `-gateway` pour sentinel/nexus. Deux dossiers hors crates : `sentinel-ml/` (modèles ONNX `text/`, `vision/`) et `persona/` (fiches de rôle en Markdown pour la skill `party-mode`).

Atrium est la plus jeune et la plus petite (≈30 fichiers) : elle n'a ni gateway ni sous-états d'API. Ne pas la prendre comme modèle de structure — la référence reste `sentinel-*`.

**L'identité est une plateforme, pas une fonction de Sentinel.** `auth-core` + `auth-api` servent l'OAuth2 Discord, les sessions web et le gate superadmin, avec leur propre base (`auth`) et leur rôle Postgres dédié — c'est celle qui contient les access/refresh tokens des administrateurs. Les trois passerelles nginx (`/nexus-api/`, `/ops-api/`, `/atrium-api/`) interrogent `auth-api:8096/access`, et `sentinel-api` est un **consommateur comme les autres** via `platform_common_api::auth_client`. Avant, l'identité vivait dans `sentinel-api` : Nexus et Atrium ne savaient pas qui appelle, et Sentinel était la dépendance d'exécution qui, en tombant, fermait tout le back-office. Ne pas réintroduire de résolution d'identité locale — c'est le point où deux implémentations de la même règle divergent.

`AccessOutcome::Unavailable` (→ **503**) est distinct de `Denied` (→ **403**) : confondre les deux fait passer une panne réseau pour une révocation de droits.

**Trois instances Redis, pas une.** `redis` (commun), `auth-redis`, `nexus-redis`. Le découpage suit le **périmètre du bus**, pas l'organigramme : Nexus est séparable parce que `nexus:events` et l'allocation de ports sont internes à la plateforme. Atrium **reste sur l'instance commune** parce qu'il *consomme* `sentinel:events` (les demandes d'apaisement publiées par l'AutoMod) — un bus dont le producteur et le consommateur sont sur deux Redis différents ne transporte rien, et l'échec est silencieux. C'est le critère à appliquer avant toute nouvelle séparation.

`nexus-redis` est en `noeviction` **et** persistant : les réservations de port (`game:port:*`) sont le garde-fou anti-collision, et sous `allkeys-lru` — le réglage de l'instance commune — Redis peut en évincer une encore active. Deux serveurs de jeu se voient alors attribuer le même port, et le symptôme apparaît au démarrage du second, très loin de la cause.

L'identité a aussi **son propre Redis** (`auth-redis`), sans persistance et en `noeviction`. Sur l'instance commune, tout porteur de `REDIS_URL` — les trois bots, les trois workers, la gateway — pouvait lire les `state` CSRF (`oauth:web:state:*`) et le cache `token → identité` (`user_id:*`), donc résoudre n'importe quel jeton et forger un callback OAuth valide. `noeviction` et non `allkeys-lru` : sous LRU, Redis peut évincer un `state` encore valide, et le login échoue alors avec « state invalide » sans rien qui l'explique dans les logs.

## Règles d'or

1. **Le métier va dans `sentinel-core` / `nexus-core` / `atrium-core`.** `*-api`, `*-bot`, `*-worker`, `*-gateway` ne sont que des adaptateurs. Si tu écris une règle de décision dans un handler HTTP ou dans un module du bot, c'est au mauvais endroit.
2. **Sens des dépendances** : `domain` ← `application` ← `ports` ← adapters. `domain/` ne dépend d'aucune infra (pas de `sqlx`, pas de `serenity`, pas de `reqwest`).
3. **Le bot est une interface légère.** Il rend, il écoute, il appelle l'API. Il ne décide pas et n'a **pas d'accès DB**.
4. **Un adaptateur inbound ne fait pas d'I/O sortante.** Pas de `reqwest::Client` dans un handler : passer par le port (`DiscordApi::send_channel_embed`, `get_user_me`, …). Un handler qui appelle le réseau lui-même est intestable sans réseau, et chaque appelant réimplémente les contrôles de sécurité — c'est ainsi que la validation du snowflake s'est retrouvée copiée dans trois fichiers. Seule exception documentée : l'échange de jetons OAuth2 (`handlers/system/oauth.rs`), indissociable du flux CSRF/cookies.
5. **Config par serveur d'abord.** Un nouveau réglage se déclare dans `bot_definitions.config_schema` et se lit dans `bot_guild_config` — pas en variable d'env. Les env vars sont des fallbacks / réglages globaux lus au démarrage. La sémantique de référence est `sentinel-core/src/domain/entities/system/config_parsers.rs` : `parse_bool_str` (`true`/`1`/`yes`, insensible à la casse) et surtout **`parse_enabled_flag` : clé absente = module DÉSACTIVÉ** (fail-closed, miroir de `parseBoolConfig` côté web). Ne pas réécrire ces parsers ailleurs. `nexus-core` a longtemps porté une copie inline au défaut inversé (absent = activé) ; elle a été supprimée (cf. en-tête de `nexus-core/.../system/bot_config.rs`), le `cfg_bool` restant est appelé fail-closed. Ne pas la réintroduire.
6. **Filtre fermant sur tout ce qui est public.** En cas de doute sur une permission Discord (cache froid, guilde inconnue), on ne publie pas. Voir `sentinel-bot/src/modules/presence`.
7. **Le ban n'est jamais automatique.** Toute évolution de l'automod doit préserver ça : seul un administrateur finalise un ban.

## `#[allow(dead_code)]`

Passés de **121 à 7**, et chacun des 7 porte sa justification en commentaire. Un `allow` neuf sans explication masque autant les vrais oublis que le faux positif qu'il vise. Résolutions par ordre de préférence :

1. **Supprimer le code** s'il est vraiment mort.
2. **`#[cfg(test)]`** si l'élément n'existe que pour les tests (accesseur de vérification) — préserve la couverture sans mentir sur l'usage.
3. **`allow` justifié**, en dernier recours et toujours commenté.

Les 7 restants : `tests/test_helpers.rs` (inclus dans ~40 binaires, chacun n'en consommant qu'une partie) et 4 DTO miroirs de contrats d'API dont le bot ne lit qu'une partie des champs.

**Deux fonctionnalités annoncées mais non implémentées** ont été découvertes en retirant ces `allow` — elles étaient exactement ce qu'il masquait :
- `guild-backup-bot` : « Sauvegarde automatique », son intervalle, et « Rôles autorisés à restaurer » sont exposés dans l'interface. Aucun n'est lu. Le contrôle d'accès au restore n'est **pas** appliqué (seule la gate Owner côté API protège).
- `welcome` : les 6 champs `anniversary_*` sont configurables mais aucun handler du bot ne les rend.
- `moderation` : `list_reminders` est appelé, son résultat jamais consommé.

## Ne pas faire sans demande explicite

- **Ne pas lancer `cargo test`** — les builds sont longs. `cargo check` et `cargo clippy` suffisent pour valider une modif.
- **Ne pas redémarrer / arrêter les services** (docker compose, bot, API) — l'environnement de l'utilisateur tourne.
- Ne pas créer de migration « corrective » sans regarder d'abord les dernières migrations existantes.

## Vérifier son travail

```bash
cargo check --workspace
cargo clippy --workspace --all-targets    # lints partagés définis dans le Cargo.toml racine
cd web && npm run lint && npm run build   # build = vue-tsc --noEmit + vite build
```

## Où va quoi

| Tu veux… | Va dans |
|---|---|
| Ajouter une règle métier Sentinel | `sentinel-core/src/application/<domaine>/<verbe>_service.rs` |
| Exposer ça en HTTP | `sentinel-api/src/adapters/inbound/http/{routes,handlers,dto}/` |
| Persister | `sentinel-core/src/ports/outbound/` (trait) + `sentinel-api/src/adapters/outbound/postgres/` (impl) |
| Câbler un nouveau port dans l'API | le sous-état de son domaine dans `sentinel-api/src/bootstrap/state/` |
| Ajouter une commande slash | `sentinel-bot/src/modules/<module>/` + `sentinel-bot/src/command_registry.rs` |
| Un job périodique | `sentinel-worker/src/domains/<domaine>/` + `scheduler.rs` |
| Un écran d'admin | `web/src/components/pages/` + store Pinia + `web/src/api/http.ts` (ou `nexusHttp.ts`), route dans `router/adminRoutes.ts` sous son univers, entrée dans `useDashboardSections` |
| Un réglage éditable par serveur | migration `config_schema` + lecture via `bot_guild_config` |
| Toucher aux serveurs de jeux | `nexus-core/src/application/game/` + `nexus-api/src/adapters/outbound/game_runtime/` |

Domaines de `sentinel-core/src/application/` : `ai`, `audit`, `community`, `guild_backup`, `moderation`, `system` (le métier `ops` vit désormais dans le crate `ops-core`, cf. plus bas).

**Le socket Docker n'est monté que par `docker-agent`.** `/var/run/docker.sock` équivaut à un accès root sur l'hôte ; il n'a rien à faire dans une API qui sert aussi l'OAuth, la modération ou la vitrine publique du portail de jeux. `docker-agent` est un crate minimal — pas de base, pas de session, pas de route nginx, joignable seulement sur le réseau `internal` avec `DOCKER_AGENT_TOKEN`.

Il expose **deux surfaces**, toutes deux en liste blanche stricte, jamais un passe-plat vers l'API Docker :

| Surface | Port | Implémentation | Client |
|---|---|---|---|
| `/version`, `/containers`, `/prune/*`… | `ops_core::DockerHost` | `bollard_host.rs` | `ops-api` → `HttpDockerHost` |
| `/game/*` (cycle de vie des serveurs de jeu) | `ops_core::GameContainerRuntime` | `bollard_game.rs` | `nexus-api` → `HttpGameRuntime` |

Les deux clients sont de simples adaptateurs : les handlers et les use cases ignorent que Docker est passé de l'autre côté d'un appel réseau. C'est tout l'intérêt d'avoir eu un port plutôt qu'un client bollard appelé directement.

**Un jeton par surface**, et l'agent refuse de démarrer s'ils sont identiques : `DOCKER_AGENT_TOKEN` (hôte, porté par `ops-api`) et `DOCKER_AGENT_GAME_TOKEN` (`/game/*`, porté par `nexus-api`). Avec un jeton unique, les identifiants de `nexus-api` ouvraient l'arrêt et la purge de **tous** les conteneurs de l'hôte, `postgres` et `auth-api` compris — on lui avait retiré l'accès direct au socket pour lui confier la clé du processus qui l'a. La séparation est stricte dans les deux sens, et les macros `guarded!` / `guarded_game!` rendent le choix visible à la relecture.

**Ne pas remonter le socket ailleurs**, et ne pas ajouter `bollard` (ni `tar`, qui ne sert qu'à `upload_file_to_container`) à un autre crate : le mapping bollard → domaine n'existe qu'une fois, dans `docker-agent/`. `cargo tree -e normal -i bollard` doit toujours ne montrer qu'un seul dépendant. Nexus a longtemps été la contre-épreuve — il portait `bollard`, montait le socket, et dupliquait un second mapping de 537 lignes.

Corollaire de rangement : le port du cycle de vie vit dans `ops-core` (domaine neutre de la machine hôte), pas dans `nexus-core` — c'est une opération sur le daemon de l'hôte, pas une règle du portail. `nexus-core` le ré-exporte sous le nom court `ContainerRuntime`.

**Le nommage `sentinel.*` de Nexus : ce qui a bougé et ce qui ne bougera pas.** Les **labels** sont passés à `nexus.*` — écrits dans les deux générations, lus dans les deux (`list_managed_containers` fait deux passes, Docker combinant les filtres `label` en ET et non en OU). La sortie de transition se fait quand `docker ps -a --filter label=sentinel.managed=game-portal` ne renvoie plus rien.

Les **noms** restent : `sentinel-game-{id}` (persisté en base, comparé par le reconciler), `sentinel-game-vol-{id}` (Docker ne renomme pas un volume — changer la formule monterait un volume neuf et vide, le monde de jeu paraîtrait effacé), `sentinel-games` et `/var/lib/sentinel/games` (défauts décrivant l'installation en place). Ce sont des identifiants portant des données, pas des étiquettes. Les renommer demande une migration explicite, jamais une édition de ligne.

**`ops` vs `system`** — la frontière est « est-ce que ça parle de Discord ? ». `ops` couvre la **machine hôte** : sondes système, conteneurs Docker, logs techniques des services, sécurité de l'hôte (TLS, IP bannies, journal d'administration), règles d'alerte. Cette machine héberge aussi Nexus et Atrium : ces écrans ne sont pas « du Sentinel », ils sont transverses. `system` garde le métier de la plateforme : tickets, OAuth, reset de guilde, lockdown, slowmode, quarantaine, exports. Le métier `ops` **a été extrait en plateforme autonome** — `ops-core` (domaine + ports, aucune dépendance de plateforme) et `ops-api` (adapters + gateway nginx, façon nexus/atrium). `sentinel-api` n'en garde qu'un **consommateur** : le sous-état `OpsState` (`bootstrap/state/ops.rs`) branché sur les ports `ops-core` (sondes santé, registre des services, logs). Il n'y a plus de `application/ops/`, `ports/ops/` ni `domain/entities/ops/` dans `sentinel-core`. L'univers « Exploitation » du back-office est la contrepartie web.
Domaines de `sentinel-worker/src/domains/` (16) : `ai`, `analytics`, `announcements`, `appeal_sla`, `audit_cache`, `automod`, `cache`, `cleanup`, `discord_audit_sync`, `export`, `guild_backup`, `moderation`, `monitoring`, `security`, `temp_roles`, `tickets`.

## État de l'API : sous-états par domaine

`AppState` vit dans **`sentinel-api/src/bootstrap/state/`** — c'est la composition root, pas un détail de l'adaptateur HTTP. (`adapters/inbound/http/state.rs` n'est plus qu'une ré-exportation de compatibilité ; ne l'utilise pas dans du code neuf.)

Un handler déclare **le sous-état de son domaine**, jamais le god-object.

```rust
// ✅ à faire — le compilateur interdit à ce handler de toucher au reste
async fn restore(State(st): State<GuildBackupState>, ...) { st.guild_snapshots_uc... }

// ❌ forme héritée, à ne plus écrire dans du code neuf
async fn restore(State(st): State<AppState>, ...) { st.guild_snapshots_uc... }
```

Chaque sous-état implémente `FromRef<AppState>`, donc les deux formes coexistent dans un même `Router<AppState>` : la migration se fait fichier par fichier, avec un code qui compile à chaque étape.

**Migration terminée.** Sept sous-états : `ai`, `moderation`, `audit`, `community`, `system`, `ops`, `guild_backup`. `AppState` est passé de **100 à 14 champs**, tous légitimes : infrastructure partagée (`broadcaster`, `redis_client`, `cache`, `discord_api`, `job_client`, `log_repo`, `bot_config_repo`, `pg_pool`), config lue par les middlewares (`api_key`, `guild_id`, `metrics_token`, `discord_bot_token`) et `nexus_games`.

`superadmin_user_ids` n'y figure plus. `SUPERADMIN_USER_IDS` reste la variable qui décide qui entre dans le back-office, mais **seul `auth-api` la lit**. La copie locale servait un scope par rôle dans `list_tickets` et `list_all_channels` : identité comparée à la liste locale, sinon repli sur `moderated_guilds`, qui faisait un `SELECT` sur `api_user_guilds` — table supprimée par la migration 007. Le moindre écart entre les deux listes transformait donc ces deux écrans en 500. Le port `moderated_guilds`, l'outbound `find_user_guild_roles` et leurs implémentations ont été retirés avec.

Un fichier reste volontairement sur `AppState`, faute d'appartenir à un domaine unique : `handlers/moderation/purge.rs` (audit-logs + logs système). Le forcer dans un sous-état aurait reconstitué un god-object en miniature.

`handlers/community/voice_channels.rs` en est sorti : il réclamait `tickets_uc` et `superadmin_user_ids` uniquement pour le scope par rôle supprimé ci-dessus. `VoiceChannelsState` ne porte plus que `voice_channels_uc`, `audit_logs_uc` (il trace ses propres actions) et `broadcaster`.

**Règle de rangement** : si un fichier réclame plus de 2-3 ports étrangers à son domaine, c'est le fichier qui est mal rangé, pas le sous-état qui est trop étroit.

**Écriture de fichiers en masse** : n'utilise pas `Get-Content -Raw` + `Set-Content` sous PowerShell 5.1 — `Get-Content` décode un fichier sans BOM en ANSI, ce qui double-encode tous les accents au réenregistrement. Utilise `[System.IO.File]::ReadAllText/WriteAllText`, ou l'outil Edit pour tout texte accentué.

Pour ajouter un domaine (ou en déplacer un fichier) :
1. Créer/compléter `bootstrap/state/<domaine>.rs` (struct + `FromRef<AppState>`).
2. Le construire dans `bootstrap/app_state.rs` **avant** le littéral `AppState`, et faire pointer les champs plats correspondants sur des clones du sous-état — jamais deux instanciations du même port.
3. Dans les handlers : remplacer l'import et `State<AppState>` → `State<XState>`.
4. Le compilateur liste alors les dépendances transverses réellement utilisées (souvent `broadcaster`, `bot_config_repo`) : les **ajouter explicitement** au sous-état, c'est l'information qu'on cherchait.
5. `tests/test_helpers.rs` : hisser en variables locales tout port que les tests inspectent (`broadcaster` **doit** être une instance unique, sinon les assertions d'événements portent sur un canal que personne n'écoute).
6. Supprimer les champs plats du domaine seulement quand plus aucun site ne les lit.

## Code partagé entre les plateformes

Quatre crates socles, séparés par **surface de dépendances** — un bot n'a aucune raison de compiler axum, une API aucune raison de compiler serenity. Le suffixe dit qui a le droit de dépendre du crate :

| Crate | Contenu | Consommé par | Dépendances |
|---|---|---|---|
| `platform-common` | Bus d'événements Redis Streams (`EventBus`, paramétré par la clé de stream), erreurs communes, `config_flags` (sémantique de référence de `enabled`) | les trois `-core`, les trois `-bot`, `platform-common-worker` | redis, tokio — aucun framework |
| `platform-common-api` | Rate limit par IP, métriques Prometheus, CORS, en-têtes de sécurité, mapping d'erreurs HTTP | les trois `-api` | axum, tower-http, metrics |
| `platform-common-bot` | Embeds normalisés (`embeds.rs`) et helpers d'interaction Discord (`discord_helpers.rs` : `defer_ephemeral`, `option_str`, …) | les trois `-bot` | serenity |
| `platform-common-worker` | Boucle de scheduler, client API, helpers Redis, métriques | les trois `-worker` | tokio, reqwest, sqlx — **aucune dépendance de plateforme** |

**Le suffixe est un contrat, pas une étiquette.** `platform-common-worker` a longtemps dépendu de `sentinel-core` et `sentinel-proto` — pour deux parsers et un helper mTLS. `nexus-worker` et `atrium-worker` compilaient donc tout le domaine de Sentinel sans jamais l'appeler, et une modif du domaine pouvait casser leur build. Les parsers sont remontés dans `platform-common::config_flags` (leur place : ils doivent être identiques partout), le helper mTLS est redescendu dans `sentinel-worker/src/grpc.rs`, son unique appelant. **Aucun crate socle ne dépend d'une plateforme** ; si c'est tentant, c'est que le code n'appartient pas au socle.

Corollaire côté logs : `send_worker_log` / `send_lifecycle_log` ne postent sur `POST /api/logs` que si la plateforme l'a déclaré via `enable_worker_log_push` (seul `sentinel-worker` le fait — c'est la seule API qui expose la route). Sans ça, le socle lisait `SENTINEL_API_KEY` en dur : les workers Nexus et Atrium POSTaient sans clé, à chaque tick, sur une route inexistante, l'échec avalé en `debug!`.

**Le critère d'entrée est la preuve, pas l'intuition** : on ne mutualise que du code mesuré identique. L'event bus l'était à 352 lignes sur 353. À l'inverse, les `api_client.rs` des bots ne partagent que 118 lignes sur 517 : ils restent dupliqués, parce qu'une abstraction inventée pour deux besoins différents coûte plus cher que la duplication qu'elle supprime.

Restent propres à chaque plateforme : l'authentification, le verrou mono-serveur et les règles métier.

**Consommer un crate socle, ce n'est pas le recopier.** Le pont se fait par ré-export, pas par duplication du fichier :

```rust
// sentinel-bot/src/shared/discord_helpers.rs — la bonne forme
pub use platform_common_bot::discord_helpers::{defer_ephemeral, option_str, /* … */};
// puis les helpers propres à Sentinel en dessous.
```

Ce module-là est correct ; `sentinel-bot/src/shared/embeds.rs` a été converti en `pub use platform_common_bot::embeds::*` (il ne reste que le chemin d'import historique pour les ~20 fichiers qui en dépendent). Une copie ne se voit pas au `cargo check` : elle se voit le jour où l'on corrige un seul des deux exemplaires — d'où la règle du ré-export.

## Deux chemins pour agir sur Discord depuis le web

Choisir en connaissance de cause :

- **Synchrone** — l'API appelle l'API Discord directement (`DiscordApi` dans les adapters outbound). Pour ce qui doit répondre immédiatement et rapporter un résultat à l'appelant.
- **Asynchrone** — l'API publie sur Redis Stream `sentinel:events` (`XADD MAXLEN ~ 10000`, champ `payload = {"event":…, "data":…}`), un module du bot consomme via `XREADGROUP` + `XACK` et rapporte le résultat à l'API. **Obligatoire** quand le message doit venir du bot (identité, avatar, permissions) : voir `modules/{announcements,embeds,messages}`.

La gateway lit la même stream en `XREAD $` (live-tail, sans group) pour le relay WebSocket.

**Les events destructifs sont signés (HMAC-SHA256, secret = `SENTINEL_API_KEY`).** `sentinel:events` vit sur l'instance Redis **commune** : les trois bots, les trois workers et la gateway en portent l'URL, donc y publier ne demande aucun privilège. C'est acceptable pour un event d'affichage, pas pour `guild_reset` (déban de tous les bannis, retrait des rôles) ni `guild_backup:restore_requested`, qui avec `wipe` supprime **tous** les salons, rôles et emojis avant de restaurer. Le bot rejette un event non signé ou mal signé ; secret vide (dev) = signature non exigée.

Le **message canonique** existe en trois exemplaires — `sentinel-api/src/adapters/inbound/http/event_signing.rs`, `sentinel-bot/src/shared/event_signing.rs`, et `sign_capture` dans `sentinel-worker/.../guild_backup/auto_backup.rs`. C'est un contrat inter-processus, pas du code mutualisable : aucun des trois crates ne peut dépendre des deux autres. Modifier un format sans le répercuter partout fait rejeter l'event — le sens de défaillance est le bon (rien n'est détruit), mais ça ne se voit que dans les logs du bot. Tout champ qui change l'effet de l'action doit être **dans** le message signé : `wipe` y est, sinon une restauration légitime se rejouerait en effacement.

`guild_backup:restore_requested` ne l'était pas alors que `guild_reset` l'était depuis le début — l'asymétrie protégeait la moins destructive des deux opérations.

## Deux pièges qui ne se voient pas à la relecture

**`into_make_service_with_connect_info` n'est pas optionnel.** Le rate limit commun extrait `ConnectInfo<SocketAddr>` ; servir un routeur avec `axum::serve(listener, router)` fait répondre **500 à toutes les routes**, `/health` compris — donc healthcheck en échec, conteneur jamais `healthy`, et tout ce qui l'attend en `depends_on` qui ne démarre pas. C'est arrivé à `atrium-api`, et ça n'a pas été vu parce que les tests appelaient le routeur en `oneshot` **en injectant `ConnectInfo` à la main**. Un test qui fabrique lui-même l'extension que la production oublie ne teste rien. `atrium_api::serve` centralise désormais le montage, et `health_repond_quand_l_api_est_servie_comme_en_production` ouvre une vraie socket.

**Un secret vide n'est pas un secret absent.** `std::env::var` rend `Ok("")` pour une variable déclarée mais vide : un contrôle qui ne teste que la présence laisse passer la chaîne vide. Et `bearer_auth::matches(h, "")` renvoyait alors `true` dès que le client envoyait `Authorization: Bearer ` — le préfixe seul. `ATRIUM_API_TOKEN: ${ATRIUM_API_TOKEN:-}` rendait le cas atteignable, ouvrant tout `/admin/*`. Trois barrières désormais : le compose exige la variable (`:?`), la config refuse une valeur vide, et `matches` refuse un jeton attendu vide. Utiliser `:?` et non `:-` pour tout secret.

## Migrations

- Sentinel : `sentinel-api/migrations/` — `001_init.sql` (base vierge) + migrations incrémentales numérotées. Historique pré-refonte archivé dans `migrations_legacy/`.
- Nexus : `nexus-api/migrations/`.
- Atrium : `atrium-api/migrations/`.

**Une base logique, un rôle, un pgbouncer.** La séparation en bases ne vaut que si chacune a son propre compte : `POSTGRES_USER: sentinel` est le **superuser** du cluster, et un superuser ignore tous les `GRANT`. Quatre rôles ordinaires, un par base, chacun propriétaire de la sienne et créé par son `*-db-init` : `sentinel_app` (`discord_sentinel`, partagée par sentinel-api, sentinel-worker et ops-api), `nexus`, `atrium`, `auth`. Aucune API n'est sur le réseau `data`, chacune passe par son pgbouncer. Le compte superuser ne sert plus qu'aux `*-db-init` et à pgadmin.

Deux extensions exigeaient le superuser et bloquaient la bascule — `pg_stat_statements` côté Sentinel, `pgvector` côté Atrium. Elles sont créées par le `db-init` **avant** les migrations : le `CREATE EXTENSION IF NOT EXISTS` de la migration devient alors un no-op et passe sous un rôle ordinaire. C'est le motif à réutiliser pour toute extension future, plutôt que de rendre le superuser à un service.

Les `db-init` révoquent aussi le `CONNECT` de `PUBLIC` sur les autres bases : sans ça, un rôle applicatif peut encore ouvrir une session ailleurs (sans rien y lire, mais c'est une surface).

Ne pas rebrancher une API sur le réseau `data` ni sur le compte `sentinel`. Et ne pas réintroduire de rôle « restreint » par service au sein d'une même base : `sentinel_ops` (migration 024) a été abandonné en 028 — jamais utilisé, droits incomplets, il donnait l'illusion d'un cloisonnement inexistant.

**Chaque plateforme a sa propre base logique**, donc ses propres tables `bot_definitions` / `bot_guild_config` : `nexus-api/migrations/007_game_portal.sql` et `atrium-api/migrations/007_config_par_serveur.sql` les répliquent. Un `-api` ne lit jamais la base d'une autre plateforme. Un nouveau réglage se déclare donc dans le `config_schema` **de sa plateforme**.
- Numéroter à la suite, nom en français descriptif, une préoccupation par fichier.

## Dépendances

Toute dépendance partagée se déclare dans `[workspace.dependencies]` du `Cargo.toml` racine, puis `dep = { workspace = true }` dans le crate. Restent inline : deps target-gated (jemalloc), `ort`, `bollard`, spécifiques d'un seul crate.

## Conventions

- Commentaires et doc en **français**, comme le reste du code. Les `//!` de tête de module expliquent le *pourquoi*, pas seulement le *quoi* — s'aligner sur ce style (cf. `modules/presence`, `modules/messages`).
- Discord IDs : `VARCHAR(20)` en base, `String` en Rust.
- Erreurs : `thiserror` dans le core, conversion en réponse HTTP dans `adapters/inbound/http/errors.rs`.
- Le web suit l'**atomic design** : `atoms` → `molecules` → `organisms` → `templates` → `pages`. Une *template* ne contient pas de markup métier : elle compose des organisms (cf. `MainLayout` = `Sidebar` + `TopBar`, `PublicLayout` = `SiteHeader`).

## Le web : univers, mises en page, navigation

**Quatre univers** (`web/src/universes.ts`, source unique) : `sentinel`, `nexus`, `atrium` et `ops`. Les trois premiers sont des produits Discord ; **`ops` (« Exploitation ») est la machine hôte** — Docker, disques, TLS, IP bannies, logs des services. Transverse aux trois plateformes, donc à sa place dans aucune.

Chaque univers déclare sa marque, sa **couleur d'accent** et sa page d'accueil. Les accents sont volontairement écartés (`#5865f2` / `#a855f7` / `#14b8a6` / `#f59e0b`) : Sentinel et Nexus étaient auparavant deux bleu-violets à un cran d'écart, et la couleur ne disait rien du produit. `MainLayout` pose `--universe-accent` sur la coque ; sidebar, topbar et `AdminPageShell` en héritent. Ne pas réintroduire de couleur par groupe de menu.

**L'univers se déclare, il ne se déduit pas.** Il vient de `route.meta.universe`, jamais d'une analyse d'URL. Le ternaire `path.startsWith("/nexus") ? "nexus" : "sentinel"` faisait de Sentinel le `else` de Nexus — d'où l'impossibilité d'un 3ᵉ univers, les pages publiques qui basculaient l'app en Sentinel, et le logo qui ramenait toujours sur `/dashboard`. Les routes d'administration passent par `inUniverse(...)` dans `adminRoutes.ts` ; une entrée de menu sans `universe` ne compile pas.

**Deux racines de routeur** : `publicRoutes.ts` (site communautaire + connexion) et `adminRoutes.ts` (back-office). `meta.public` ne dit QUE « accessible sans connexion » ; la mise en page vient de `meta.layout` (`site`, `bare`, ou absent = back-office). Les deux étaient confondus, ce qui laissait le site public sans navigation et obligeait chaque page à redessiner sa barre.

**Où mettre une page** :

| Type | Dossier | Mise en page |
|---|---|---|
| Écran d'administration | `components/pages/` | `AdminPageShell` (titre, `lede`, `actions`) |
| Page du site public | `components/pages/public/` | `PublicLayout` via `meta.layout: "site"` |
| Connexion / OAuth | `components/pages/auth/` | aucune (`meta.layout: "bare"`) |

**Les hubs à onglets** (`StatsHubPage`, `ModerationHubPage`, `RolesHubPage`, `VoiceHubPage`, `LevelsHubPage`) portent l'`AdminPageShell` : titre fixe, et `lede` qui suit l'onglet actif via un champ dans le tableau `tabs`. Leurs **contenus d'onglet** (`StatsPage`, `ReviewPage`, `LevelsConfigPage`… — sans route propre) n'ont donc **pas** de shell : ils commencent par un `<div class="…-tab">`, avec au besoin un `<p class="tab-note">` et une `.tab-toolbar` pour leurs actions. Un shell dans un onglet empile deux titres et fait changer la forme de l'en-tête à chaque clic.

Une page enchâssée qui garde une route propre gate son en-tête sur une prop (`NotesPage`, `EvidencePage` : `v-if="!props.embedded"`). Deux pages délèguent leur en-tête à un organism parce qu'il change selon la vue : `IdeasPage` et `TicketsPage` (liste ou détail).

**Ne pas recopier le titre dégradé.** Il existait en quatre exemplaires (`StatsPage`, `ModstatsPage`, `ModerationHubPage`, `ServerHealthPage`), chacun avec sa propre `@keyframes`. `AdminPageShell` en est la seule source.

**Trois clients HTTP, trois modèles d'authentification** — ne pas les fusionner : `api/http.ts` (bearer + token Discord, refresh de session), `api/nexusHttp.ts` et `api/atriumHttp.ts` (passerelle nginx, clé injectée côté serveur, jamais dans le SPA), `services/publicHttp.ts` (aucune credential, ne redirige jamais vers `/login`).

**Passerelles nginx** (`web/nginx.conf`) : `nexus-api` et `atrium-api` ne sont pas publiés sur l'hôte. Le SPA les atteint via `/nexus-api/` et `/atrium-api/`, qui font tous deux `auth_request` vers sentinel-api (seul composant sachant *qui* est connecté) puis injectent le secret depuis un snippet généré au démarrage par `/docker-entrypoint.d/3x-*-key.sh`. Ajouter une passerelle = un bloc `location`, un script d'entrypoint, un `COPY` dans `web/Dockerfile` et la variable dans le service `web` du compose. Le `set $upstream` doit **précéder** le `rewrite ... break`, sinon la variable reste vide et nginx répond 500.

**Attention au mot « serveur »**, qui désigne trois choses : le serveur Discord (guilde), le serveur de jeu (Nexus) et la machine hôte. Les libellés doivent lever l'ambiguïté — « Sauvegardes du serveur Discord », « État de la machine », « Sécurité de l'hôte ».
