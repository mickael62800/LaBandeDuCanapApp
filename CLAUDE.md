# CLAUDE.md

Repères pour travailler dans ce dépôt. Le [README](README.md) décrit le produit ; ce fichier décrit **comment coder ici**.

## Le dépôt en une phrase

Monorepo Rust : **trois plateformes hexagonales** — `sentinel-*` (modération Discord), `nexus-*` (serveurs de jeux + casino), `atrium-*` (parcours d'accueil assisté par IA) — partageant `web/` (Vue 3), `infrastructure/` (Docker/Grafana/Prometheus), Postgres et Redis.

Chaque plateforme suit le même découpage : `-core` (métier), `-api`, `-bot`, `-worker`, `-proto`, et `-gateway` pour sentinel/nexus. Deux dossiers hors crates : `sentinel-ml/` (modèles ONNX `text/`, `vision/`) et `persona/` (fiches de rôle en Markdown pour la skill `party-mode`).

Atrium est la plus jeune et la plus petite (≈30 fichiers) : elle n'a ni gateway ni sous-états d'API. Ne pas la prendre comme modèle de structure — la référence reste `sentinel-*`.

## Règles d'or

1. **Le métier va dans `sentinel-core` / `nexus-core` / `atrium-core`.** `*-api`, `*-bot`, `*-worker`, `*-gateway` ne sont que des adaptateurs. Si tu écris une règle de décision dans un handler HTTP ou dans un module du bot, c'est au mauvais endroit.
2. **Sens des dépendances** : `domain` ← `application` ← `ports` ← adapters. `domain/` ne dépend d'aucune infra (pas de `sqlx`, pas de `serenity`, pas de `reqwest`).
3. **Le bot est une interface légère.** Il rend, il écoute, il appelle l'API. Il ne décide pas et n'a **pas d'accès DB**.
4. **Un adaptateur inbound ne fait pas d'I/O sortante.** Pas de `reqwest::Client` dans un handler : passer par le port (`DiscordApi::send_channel_embed`, `get_user_me`, …). Un handler qui appelle le réseau lui-même est intestable sans réseau, et chaque appelant réimplémente les contrôles de sécurité — c'est ainsi que la validation du snowflake s'est retrouvée copiée dans trois fichiers. Seule exception documentée : l'échange de jetons OAuth2 (`handlers/system/oauth.rs`), indissociable du flux CSRF/cookies.
5. **Config par serveur d'abord.** Un nouveau réglage se déclare dans `bot_definitions.config_schema` et se lit dans `bot_guild_config` — pas en variable d'env. Les env vars sont des fallbacks / réglages globaux lus au démarrage. La sémantique de référence est `sentinel-core/src/domain/entities/system/config_parsers.rs` : `parse_bool_str` (`true`/`1`/`yes`, insensible à la casse) et surtout **`parse_enabled_flag` : clé absente = module DÉSACTIVÉ** (fail-closed, miroir de `parseBoolConfig` côté web). Ne pas réécrire ces parsers ailleurs — `nexus-core` en a une copie inline dont le défaut est inversé (absent = activé) ; elle n'est appelée par personne, à supprimer plutôt qu'à imiter.
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

Domaines de `sentinel-core/src/application/` : `ai`, `audit`, `community`, `guild_backup`, `moderation`, `system`.
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

**Migration terminée.** Six sous-états : `ai`, `moderation`, `audit`, `community`, `system`, `guild_backup`. `AppState` est passé de **100 à 14 champs**, tous légitimes : infrastructure partagée (`broadcaster`, `redis_client`, `cache`, `discord_api`, `job_client`, `log_repo`, `bot_config_repo`, `pg_pool`), config lue par les middlewares (`api_key`, `guild_id`, `superadmin_user_ids`, `metrics_token`, `discord_bot_token`) et `nexus_games`.

Deux fichiers restent volontairement sur `AppState`, faute d'appartenir à un domaine unique : `handlers/moderation/purge.rs` (audit-logs + logs système) et `handlers/community/voice_channels.rs` (réclame `tickets_uc`, `audit_logs_uc` et `superadmin_user_ids`). Les forcer dans un sous-état aurait reconstitué un god-object en miniature.

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
| `platform-common` | Bus d'événements Redis Streams (`EventBus`, paramétré par la clé de stream), erreurs communes | les trois `-core` et les trois `-bot` | redis, tokio — aucun framework |
| `platform-common-api` | Rate limit par IP, métriques Prometheus, CORS, en-têtes de sécurité, mapping d'erreurs HTTP | les trois `-api` | axum, tower-http, metrics |
| `platform-common-bot` | Embeds normalisés (`embeds.rs`) et helpers d'interaction Discord (`discord_helpers.rs` : `defer_ephemeral`, `option_str`, …) | les trois `-bot` | serenity |
| `platform-common-worker` | Boucle de scheduler, client API, client gRPC, helpers Redis, métriques | les trois `-worker` | tokio, reqwest, tonic |

**Le critère d'entrée est la preuve, pas l'intuition** : on ne mutualise que du code mesuré identique. L'event bus l'était à 352 lignes sur 353. À l'inverse, les `api_client.rs` des bots ne partagent que 118 lignes sur 517 : ils restent dupliqués, parce qu'une abstraction inventée pour deux besoins différents coûte plus cher que la duplication qu'elle supprime.

Restent propres à chaque plateforme : l'authentification, le verrou mono-serveur et les règles métier.

**Consommer un crate socle, ce n'est pas le recopier.** Le pont se fait par ré-export, pas par duplication du fichier :

```rust
// sentinel-bot/src/shared/discord_helpers.rs — la bonne forme
pub use platform_common_bot::discord_helpers::{defer_ephemeral, option_str, /* … */};
// puis les helpers propres à Sentinel en dessous.
```

Ce module-là est correct ; `sentinel-bot/src/shared/embeds.rs` est encore une copie octet pour octet des 178 lignes de `platform-common-bot/src/embeds.rs`, à convertir en `pub use`. Une copie ne se voit pas au `cargo check` : elle se voit le jour où l'on corrige un seul des deux exemplaires.

## Deux chemins pour agir sur Discord depuis le web

Choisir en connaissance de cause :

- **Synchrone** — l'API appelle l'API Discord directement (`DiscordApi` dans les adapters outbound). Pour ce qui doit répondre immédiatement et rapporter un résultat à l'appelant.
- **Asynchrone** — l'API publie sur Redis Stream `sentinel:events` (`XADD MAXLEN ~ 10000`, champ `payload = {"event":…, "data":…}`), un module du bot consomme via `XREADGROUP` + `XACK` et rapporte le résultat à l'API. **Obligatoire** quand le message doit venir du bot (identité, avatar, permissions) : voir `modules/{announcements,embeds,messages}`.

La gateway lit la même stream en `XREAD $` (live-tail, sans group) pour le relay WebSocket.

## Migrations

- Sentinel : `sentinel-api/migrations/` — `001_init.sql` (base vierge) + migrations incrémentales numérotées. Historique pré-refonte archivé dans `migrations_legacy/`.
- Nexus : `nexus-api/migrations/`.
- Atrium : `atrium-api/migrations/`.

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
