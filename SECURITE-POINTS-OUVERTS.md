# Sécurité — points ouverts

Ce qui reste après les audits des plateformes `sentinel-*`, `atrium-*`, `ops-*` et `nexus-*`.

**Deux catégories, à ne pas confondre :**

- **Ouvert par décision** (S1, S2, A1–A4, O1–O5) — le point est connu, la raison de ne pas le traiter est écrite, et chacun porte la condition qui le rendrait urgent.
- **Ouvert faute d'avoir été traité** (N1–N4, W1–W4) — trouvé lors des audits Nexus et Web, non corrigé à ce jour. Ce ne sont pas des arbitrages, ce sont des correctifs en attente.

> ### ✅ Corrigés le 13/08/2026
>
> **N1, N2, A4, W1, O4.** Les deux failles réellement exploitables sont fermées : la passerelle `/nexus-public/` ne relaie plus que `/api/public/`, et `nexus-api` refuse de démarrer sans clé d'au moins 16 caractères au lieu de servir ouvert. Le détail de chacun reste plus bas, annoté du correctif appliqué — l'historique du raisonnement vaut d'être conservé.
>
> Restent ouverts : S1, S2, A1, A2, A3, O1, O2, O3, O5, W2, W3, W4, N3, N4.

## Sentinel

| # | Sujet | Pourquoi c'est encore ouvert |
|---|---|---|
| S1 | Mots de passe à défaut fonctionnel dans le compose | Le correctif fait échouer `docker compose up` si une variable manque sur le serveur — décision de déploiement, pas de code |
| S2 | `guild_id` dans le corps, hors du verrou mono-serveur | Le corriger proprement coûte un buffer par requête, pour un gain nul tant que l'installation sert une seule guilde |

## Atrium

| # | Sujet | Pourquoi c'est encore ouvert |
|---|---|---|
| A1 | `forget_member` existe mais n'est joignable par aucune route | **Dette introduite pendant le correctif** — la capacité a été écrite, pas exposée |
| A2 | `atrium_calming_requested` n'est pas signé | Le signer demande un secret partagé inter-plateformes qui n'existe pas encore |
| A3 | Contenu des membres envoyé à DeepSeek | Choix de produit, pas défaut de code — mais sans information ni base légale explicite |
| ~~A4~~ | ~~`DEEPSEEK_API_KEY` garde un défaut vide dans le compose~~ | ✅ **Corrigé le 13/08** — `:?` sur le dernier repli |

## Exploitation (ops)

Audit de `ops-api` / `ops-core`. Onze points relevés, huit corrigés dans la foulée (acteur d'audit forgeable, clé privée TLS montée, `openssl` bloquant sans délai, lecture de logs non auditée, purge des logs au ban, IPv6 privée bannissable, bornes SQL mal placées, chemins d'hôte dans les 404). Ce qui suit est ce qui **reste**.

| # | Sujet | Pourquoi c'est encore ouvert |
|---|---|---|
| O1 | `ops-api` cumule la base Sentinel complète et le jeton d'administration de l'hôte | Structurel : c'est le périmètre du produit, pas un défaut — mais c'est la concentration de pouvoir la plus forte du dépôt |
| O2 | `/metrics` ouvert par défaut | Comportement identique sur les quatre APIs ; le changer pour ops seul casserait la cohérence sans gain |
| O3 | GeoIP en clair si on l'active | Le palier gratuit d'ip-api n'accepte que `http://` — le correctif est un changement de fournisseur, pas de code |
| ~~O4~~ | ~~`deleted_logs` vaut désormais toujours `0`~~ | ✅ **Corrigé le 13/08** — champ, message et port morts retirés |
| O5 | Quatre modules non audités | Périmètre non couvert, pas un défaut constaté |

## Web — **non corrigés**

Audit statique du frontend Vue, de sa chaîne de build et de la configuration nginx. Aucun secret réel ou jeton codé en dur n'a été trouvé. Le build, ESLint et les 89 tests passent ; aucun source map n'est publié. Les en-têtes CSP, HSTS, anti-frame et `nosniff` sont présents, et les passerelles Nexus, Ops et Atrium utilisent `auth_request`.

| # | Sujet | Gravité |
|---|---|---|
| ~~W1~~ | ~~`nanoid 3.3.11` et `postcss 8.5.13`~~ | ✅ **Corrigé le 13/08** — `nanoid 3.3.18`, `postcss 8.5.26` ; `npm audit --omit=dev` renvoie 0 |
| W2 | Le rendu Markdown utilisé avec `v-html` n'échappe pas les guillemets avant de construire les liens | Moyenne — injection d'attribut HTML possible, actuellement contenue en production par la CSP |
| W3 | Le callback OAuth continue si la vérification `check-access` échoue autrement que par un 403 | Moyenne — fail-open côté interface ; les API restent l'autorité et refusent les appels non autorisés |
| W4 | L'ancien champ `api_key` peut encore être stocké dans `localStorage` et envoyé comme Bearer | Faible — vide en production actuelle, mais exfiltrable par une XSS s'il était renseigné |

## Nexus

Contrairement aux sections précédentes, ces quatre points ne sont pas des arbitrages : ils ont été relevés et attendent leur correctif. **N1 et N2 ont été corrigés le 13/08** ; N3 et N4 restent ouverts.

| # | Sujet | Gravité |
|---|---|---|
| ~~N1~~ | ~~`/nexus-public/` relaie toute l'API avec la clé injectée~~ | ✅ **Corrigé le 13/08** — préfixe descendu à `/api/public/` |
| ~~N2~~ | ~~`nexus-api` s'ouvre entièrement si `NEXUS_API_KEY` est vide~~ | ✅ **Corrigé le 13/08** — `exit(1)`, `:?` au compose, mode fail-open supprimé du socle |
| N3 | L'acteur de l'audit est un paramètre d'URL | Moyenne — traçabilité falsifiable |
| N4 | RCON transmis sans liste blanche | Redescendue à un choix de produit depuis la correction de N1 : la commande n'est plus accessible sans authentification |

---

## S1 — Mots de passe avec une valeur par défaut publiée

### Le problème

Dix secrets ont une valeur de repli **écrite dans le dépôt**. Un déploiement dont le `.env` ne définit pas l'un d'eux démarre normalement, sans avertissement, avec un mot de passe que tout lecteur du dépôt connaît.

| Variable | Défaut publié | Ce qu'il protège | Occurrences | Fichiers |
|---|---|---|---|---|
| `POSTGRES_PASSWORD` | `sentinel_secret` | **Superuser du cluster** — ignore tous les `GRANT`, donc les quatre bases | 6 | atrium, auth, core, nexus |
| `REDIS_PASSWORD` | `sentinel_redis` | Redis commun — le bus `sentinel:events` et les caches | 10 | atrium, core, observability |
| `SENTINEL_DB_PASSWORD` | `sentinel_app_secret` | Base `discord_sentinel` | 6 | core |
| `AUTH_DB_PASSWORD` | `auth_secret` | Base de l'**identité** (access/refresh tokens des administrateurs) | 3 | auth |
| `AUTH_REDIS_PASSWORD` | `auth_redis_secret` | Cache de l'identité (`state` CSRF, `token → identité`) | 3 | auth |
| `NEXUS_REDIS_PASSWORD` | `nexus_redis_secret` | Redis Nexus (réservations de port) | 4 | nexus |
| `NEXUS_DB_PASSWORD` | `nexus_secret` | Base `nexus` | 3 | nexus |
| `ATRIUM_DB_PASSWORD` | `atrium_secret` | Base `atrium` | 3 | atrium |
| `PGADMIN_PASSWORD` | `admin` | Console pgAdmin | 1 | observability |
| `GRAFANA_PASSWORD` | `admin` | Console Grafana | 1 | observability |

Les trois premiers sont les plus graves : `POSTGRES_PASSWORD` est le compte qui ignore le cloisonnement par rôle mis en place par les `*-db-init`, et `REDIS_PASSWORD` donne accès au bus sur lequel transitent les events de modération.

`PGADMIN_PASSWORD` et `GRAFANA_PASSWORD` valent littéralement `admin`.

### Ce qui n'est pas concerné

Le dépôt applique **déjà** le bon motif à quatre variables, avec l'opérateur `:?` qui refuse de démarrer plutôt que de retomber sur un défaut :

```yaml
DOCKER_AGENT_TOKEN: ${DOCKER_AGENT_TOKEN:?DOCKER_AGENT_TOKEN est requis}
OPS_API_TOKEN:      ${OPS_API_TOKEN:?OPS_API_TOKEN est requis}
AUTH_API_TOKEN:     ${AUTH_API_TOKEN:?AUTH_API_TOKEN est requis}
```

`SENTINEL_API_KEY` obtient le même résultat côté code : `sentinel-api/src/config.rs` fait `std::process::exit(1)` si elle est vide ou fait moins de 16 caractères.

**Corollaire utile** : si le stack tourne aujourd'hui, ces quatre-là sont forcément définies dans le `.env` du serveur. Ce sont les dix mots de passe ci-dessus dont la présence reste à vérifier.

### Le correctif

Étendre l'idiome existant. Pour chaque occurrence :

```diff
-  POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-sentinel_secret}
+  POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:?POSTGRES_PASSWORD est requis}
```

40 occurrences au total, réparties sur `compose.core.yml`, `compose.auth.yml`, `compose.nexus.yml`, `compose.atrium.yml` et `compose.observability.yml`.

### Avant d'appliquer

Le changement est **fail-closed** : `docker compose up` s'arrête en nommant la variable manquante. C'est le comportement voulu, mais il faut donc que le `.env` du serveur soit complet d'abord.

1. Vérifier sur le serveur quels noms sont présents :

   ```bash
   for v in POSTGRES_PASSWORD REDIS_PASSWORD SENTINEL_DB_PASSWORD \
            AUTH_DB_PASSWORD AUTH_REDIS_PASSWORD \
            NEXUS_DB_PASSWORD NEXUS_REDIS_PASSWORD ATRIUM_DB_PASSWORD \
            PGADMIN_PASSWORD GRAFANA_PASSWORD; do
     grep -q "^$v=" .env && echo "$v ok" || echo "$v ABSENT"
   done
   ```

2. Pour chaque `ABSENT`, générer une valeur et l'ajouter au `.env` :

   ```bash
   echo "NOM_DE_LA_VARIABLE=$(openssl rand -base64 32 | tr -d '/+=' | head -c 32)"
   ```

3. **Attention** : changer un mot de passe déjà en service ne suffit pas à le faire prendre. Postgres et Redis ne relisent pas la variable au redémarrage du conteneur applicatif —
   - Postgres n'applique `POSTGRES_PASSWORD` qu'à l'initialisation du volume. Sur un cluster existant, il faut un `ALTER ROLE <role> WITH PASSWORD '<nouveau>'` puis mettre à jour le `.env`.
   - Redis lit `--requirepass` au démarrage du conteneur `redis` : un `docker compose up -d redis` suffit, mais tous les clients doivent redémarrer avec la nouvelle URL.

   Autrement dit : si un service tourne aujourd'hui sur le défaut publié, le corriger est une **rotation de secret**, pas une simple édition de fichier.

4. Appliquer le `:?` et vérifier à vide : `docker compose config >/dev/null`.

### Priorité suggérée

Si tout faire d'un coup est trop risqué, l'ordre par gravité décroissante est : `POSTGRES_PASSWORD`, `REDIS_PASSWORD`, `AUTH_DB_PASSWORD` + `AUTH_REDIS_PASSWORD`, `SENTINEL_DB_PASSWORD`, puis le reste. `PGADMIN_PASSWORD` et `GRAFANA_PASSWORD` sont triviaux à changer (pas de volume à réinitialiser) et valent `admin` : autant les traiter tout de suite.

---

## S2 — `guild_id` dans le corps, hors du verrou mono-serveur

### Le constat

`sentinel-api/src/adapters/inbound/http/middleware/single_guild.rs` refuse toute requête dont le `guild_id` diffère de `GUILD_ID`. Il ne lit que **l'URL** : le paramètre de route `{guild_id}` matché par axum, plus une heuristique de repli sur le chemin.

Une trentaine de handlers reçoivent leur `guild_id` dans le **corps** de la requête et passent donc sans être confrontés à la configuration. Exemples :

- `POST /api/exports/jobs` — `CreateExportJobDto.guild_id`
- `POST /api/security/events` — `ReportEventDto.guild_id`
- la configuration par bot — `validate_bot_config(guild_id, …)`
- `bot_persistence`, `discord_action_messages`, `age_bans`, `monthly_ranking`, …

### Pourquoi ça n'a pas été corrigé

Deux options, toutes deux mauvaises en l'état :

- **Patcher chaque handler** reconstitue exactement le « contrôle de sécurité recopié dans chaque appelant » que `CLAUDE.md` proscrit (règle 4), et que ce middleware existe pour éviter. Trente `if` à maintenir, dont le trente-et-unième sera oublié.
- **Bufferiser le corps dans le middleware** pour le désérialiser demande de lire intégralement chaque requête avant de la router — y compris les images base64 de `/analyze/image`, jusqu'à `MAX_BODY_SIZE` (1 MiB par défaut). Un coût sur toutes les écritures.

Pour un gain actuellement nul : l'installation ne sert qu'une guilde, la base n'en contient qu'une, et le seul appelant web est l'administrateur unique déjà filtré par `superadmin_middleware`. Un `guild_id` étranger dans un corps ne désigne aucune donnée existante.

La limite est documentée dans l'en-tête du module, pour que personne ne lise « point de passage UNIQUE » en croyant la couverture complète.

### Quand ça devient sérieux

Deux déclencheurs, l'un ou l'autre suffit :

- **L'installation sert plusieurs guildes.** Un `guild_id` de corps désigne alors des données réelles d'un autre serveur.
- **Plusieurs comptes web coexistent.** `SUPERADMIN_USER_IDS` en liste plusieurs, ou un rôle applicatif est réintroduit.

### La forme du correctif, le jour venu

Un **extracteur axum typé**, pas un `if` recopié. Quelque chose comme :

```rust
/// Extrait le `guild_id` du corps JSON et le confronte à l'installation.
/// Un handler qui le déclare ne peut pas oublier le contrôle ; un handler
/// qui ne le déclare pas se voit à la relecture.
struct BodyGuild(GuildId);
```

Le handler déclare `BodyGuild` au lieu de lire `dto.guild_id`, et le compilateur porte la garantie. C'est le même principe que les sous-états de `AppState` : rendre la dépendance visible dans la signature plutôt que de la vérifier à la main.

À faire en même temps : reprendre `ValidatedGuild` (`http/extractors.rs`) pour que les deux extracteurs partagent la même règle de comparaison.

---

## A1 — `forget_member` n'est joignable par aucune route

**C'est une dette introduite par le correctif lui-même**, pas un reste d'audit : la méthode a été écrite et jamais exposée.

`atrium-api/src/memory.rs:176` définit :

```rust
pub async fn forget_member(&self, guild_id: &str, member_id: &str) -> Result<u64, sqlx::Error>
```

Elle n'a **aucun appelant** dans tout le dépôt. Le compilateur ne dit rien (c'est une méthode publique d'un type public), et clippy non plus. Effacer les propos d'une personne à sa demande reste donc impossible autrement qu'en `DELETE` manuel dans Postgres.

La purge automatique des 90 jours (`purge_old`, branchée sur `job_retention`) fonctionne, elle — c'est la rétention qui est traitée, pas l'effacement sur demande.

### Ce qu'il reste à faire

Une route d'administration, sur le modèle des six existantes :

```rust
// atrium-api/src/lib.rs, dans `protected`
.route(
    "/admin/guilds/{guild_id}/members/{member_id}/memory",
    delete(admin::forget_member),
)
```

Le handler valide les deux identifiants avec `valider_guild_id` (et son équivalent membre), appelle `memory.forget_member`, et journalise l'auteur — un effacement sans trace de qui l'a demandé pose le même problème que la bascule d'état sans `actor_id`.

Prévoir aussi le bouton correspondant dans l'écran d'administration Atrium, sinon la route existera sans que personne ne sache s'en servir.

---

## A2 — `atrium_calming_requested` n'est pas signé

### Le problème

`sentinel-bot/src/modules/automod/backend.rs:344` publie cet événement sur `sentinel:events`. `atrium-bot/src/main.rs` le consomme et déclenche un appel DeepSeek puis une publication Discord.

Ce bus est l'instance Redis **commune** : les trois bots, les trois workers et la gateway en détiennent l'URL. Y écrire ne demande aucun privilège particulier, et rien dans l'événement n'atteste qu'il vient bien de l'AutoMod de Sentinel.

### Ce qui a déjà été fait (et qui suffit pour l'impact immédiat)

L'événement reste forgeable, mais ce qu'il permet a été borné :

- **Dépense plafonnée** — `CalmingGrpc` passe par `BudgetGuard` (`atrium-api/src/grpc.rs`), imputée à `system:calming`. Le contournement du cooldown en faisant varier `channel_id` ne donne plus d'appels payants illimités.
- **Destination validée** — `salon_de_la_guilde` vérifie que le salon appartient bien à la guilde de l'événement, en filtre fermant.
- **Mentions bornées** — `publier_texte_modele` interdit `@everyone`, `@here` et les rôles.

Reste possible avec un accès en écriture à Redis : provoquer la publication d'un rappel d'apaisement légitime au mauvais moment, dans un salon réel de la guilde, jusqu'au plafond quotidien.

### Pourquoi ce n'est pas signé

Le motif existe déjà dans le dépôt — `guild_reset` et `guild_backup:*` sont signés en HMAC, secret `SENTINEL_API_KEY` (voir `sentinel-api/src/adapters/inbound/http/event_signing.rs`). Il n'est pas transposable ici : le consommateur est `atrium-bot`, d'une **autre plateforme**. Lui donner `SENTINEL_API_KEY` lui ouvrirait toute l'API Sentinel — on échangerait un problème contre un pire.

### La forme du correctif

Un secret **dédié aux événements inter-plateformes**, distinct des clés d'API :

```
PLATFORM_EVENTS_HMAC_KEY=<32+ caractères aléatoires>
```

Distribué à `sentinel-bot` (producteur) et `atrium-bot` (consommateur), et à eux seuls. Le message canonique suivrait la convention posée pour `guild_backup` : `atrium_calming:{guild_id}:{channel_id}:{kind}` — le `channel_id` **dans** le message signé, sinon un événement légitime se rejoue vers un autre salon.

C'est la même décision que celle qui a séparé `DOCKER_AGENT_TOKEN` de `DOCKER_AGENT_GAME_TOKEN` : un jeton par surface, pour qu'un porteur ne puisse pas déborder.

### Quand ça devient urgent

Le jour où un composant tiers, moins maîtrisé, obtient `REDIS_URL` de l'instance commune. Aujourd'hui les sept porteurs sont tous des binaires du dépôt.

---

## A3 — Contenu des membres envoyé à DeepSeek

### Ce qui sort de l'infrastructure

Vers `https://api.deepseek.com/chat/completions` (`atrium-api/src/lib.rs:40`) :

| Donnée | Source | Portée |
|---|---|---|
| Message du membre | `member_message`, jusqu'à 1 500 caractères | La personne qui écrit |
| Pseudonyme affiché | `member_display_name` | idem |
| Historique conversationnel | `ConversationMemory::history`, 10 derniers messages | idem |
| **Activité récente du serveur** | `get_recent_activity(guild_id, 50)` | **50 derniers messages, tous membres confondus** |

La dernière ligne est la plus large : la « météo d'ambiance » quotidienne (`job_generate_summary`) envoie les propos de membres qui n'ont jamais interagi avec Atrium et n'ont aucun moyen de le savoir.

DeepSeek est un fournisseur hors UE. Les conditions d'utilisation de l'API déterminent si ces contenus servent à l'entraînement — à vérifier plutôt qu'à supposer.

### Ce n'est pas un défaut de code

C'est le produit : un accueil assisté par IA suppose d'envoyer les messages à un modèle. Le point ouvert n'est pas « supprimer l'envoi » mais **le rendre explicite et borné**.

### Pistes, par coût croissant

1. **Informer** — une mention dans le règlement ou le message d'accueil : « Atrium utilise un service d'IA externe pour répondre ». Coût quasi nul, et c'est le minimum attendu.
2. **Réduire la portée du résumé** — `get_recent_activity` est le seul flux qui expose des tiers. Le restreindre aux échanges *avec Atrium* (`role = 'atrium'` et son message appairé) supprimerait la collecte de propos non concernés, au prix d'un résumé moins riche.
3. **Permettre le retrait** — une clé `bot_guild_config` par membre, ou un opt-out sur simple commande, en s'appuyant sur A1 une fois la route d'effacement en place.
4. **Rapatrier le modèle** — Ollama tourne déjà dans le stack pour les embeddings (`atrium-ollama`). Un modèle de génération local supprimerait tout transfert, contre une qualité moindre et de la RAM.

### Rétention, pour mémoire

Traité : `purge_old` efface messages et résumés au-delà de 90 jours (`ATRIUM_MEMORY_RETENTION_DAYS`), via le job quotidien. Cette variable n'est **pas déclarée dans `compose.atrium.yml`** — elle retombe sur 90 en dur. L'ajouter au compose la rendrait visible à l'exploitant plutôt que découvrable dans le code.

---

## A4 — `DEEPSEEK_API_KEY` garde un défaut vide dans le compose

`compose.atrium.yml:52` :

```yaml
DEEPSEEK_API_KEY: ${ATRIUM_DEEPSEEK_API_KEY:-${DEEPSEEK_API_KEY:-}}
```

La chaîne de replis se termine sur une valeur vide. Ce n'est **plus exploitable** : `AppConfig::from_env` refuse désormais un secret vide et l'API s'arrête au démarrage avec `variable DEEPSEEK_API_KEY vide`.

Reste que l'échec survient au démarrage du binaire plutôt qu'au montage du compose, et que la ligne voisine (`ATRIUM_API_TOKEN`, `ATRIUM_GRPC_TOKEN`) utilise `:?`. Aligner la forme :

```yaml
DEEPSEEK_API_KEY: ${ATRIUM_DEEPSEEK_API_KEY:-${DEEPSEEK_API_KEY:?DEEPSEEK_API_KEY est requis}}
```

Le message d'erreur nomme alors directement la variable, sans avoir à lire les logs du conteneur.

Même remarque que pour S1 : `:?` et non `:-` pour tout secret.

---

## O1 — `ops-api` cumule la base de Sentinel et le pouvoir sur l'hôte

### Le constat

Deux capacités se rejoignent dans un seul processus :

- **`OPS_DATABASE_URL`** pointe sur `discord_sentinel` avec le rôle `sentinel_app`, propriétaire de la base — le même que `sentinel-api` et `sentinel-worker`. Ce n'est pas un rôle restreint : `sentinel_ops` (migration 024) a été abandonné en 028 parce que ses droits étaient faux et qu'il n'a jamais servi.
- **`DOCKER_AGENT_TOKEN`** ouvre la surface hôte de `docker-agent` : arrêter, supprimer ou purger n'importe quel conteneur de la machine, `postgres` et `auth-api` compris.

Compromettre `ops-api`, c'est donc lire et écrire toute la base de Sentinel **et** disposer de la machine.

### Pourquoi c'est ouvert

C'est le périmètre du produit : l'écran « Exploitation » sert précisément à administrer l'hôte et à lire les logs techniques. Le séparer en deux services (un lecteur de logs, un pilote de conteneurs) diviserait le pouvoir mais doublerait la surface HTTP, la passerelle et la configuration — pour un gain réel seulement si les deux moitiés étaient exposées à des opérateurs différents. Elles ne le sont pas : il y a un seul superadmin.

Les réductions déjà faites : le socket Docker n'est plus monté (tout passe par l'agent en liste blanche), le jeton de l'agent est séparé de celui de Nexus, et `ops-api` n'est plus sur le réseau `data`.

### Quand ça devient sérieux

Le jour où l'exploitation est déléguée à quelqu'un qui ne doit pas voir les données Discord — un prestataire d'astreinte, par exemple. À ce moment, la coupure passe entre « lire les logs et les sondes » et « agir sur les conteneurs », et c'est cette ligne-là qu'il faudra matérialiser.

---

## O2 — `/metrics` ouvert par défaut

`ops-api/src/config.rs` : `OPS_METRICS_TOKEN` retombe sur une chaîne vide, et `handlers/metrics.rs` laisse alors passer. L'endpoint expose les noms de routes, les compteurs d'appels et les latences.

C'est **le même comportement sur les quatre APIs** : Prometheus scrape sans authentification sur le réseau interne, où il est le seul à pouvoir atteindre le port. Fermer ops-api seul créerait une exception à retenir sans supprimer l'exposition ailleurs.

Le correctif, si on le veut, est global et tient en deux gestes : définir `OPS_METRICS_TOKEN` (et ses équivalents) dans le `.env`, puis ajouter le même jeton au job Prometheus correspondant dans `infrastructure/prometheus/prometheus.yml`. À traiter avec S1, dont c'est la même famille.

---

## O3 — La résolution GeoIP circule en clair

La résolution est désormais **désactivée par défaut** (`OPS_GEOIP_ENABLED=false`) : sans opt-in explicite, aucune adresse IP de visiteur ne quitte l'infrastructure. C'était le point principal — une IP est une donnée personnelle au sens du RGPD, et le transfert vers un tiers ne doit pas être un défaut silencieux.

Ce qui reste : **si on l'active**, le défaut `OPS_GEOIP_URL=http://ip-api.com/batch` est en HTTP simple. Requête et réponse circulent en clair, donc un observateur du réseau voit quelles adresses l'administrateur enquête. Ce n'est pas corrigeable dans le code : le palier gratuit d'ip-api n'expose pas de TLS.

Trois sorties, par coût croissant :

1. Laisser désactivé — l'écran Sécurité affiche les IP sans pays, il ne casse pas.
2. Pointer `OPS_GEOIP_URL` sur un service TLS (palier payant d'ip-api, ou une instance auto-hébergée).
3. Une base locale type **GeoLite2** : supprime le transfert *et* le trafic sortant, au prix d'un fichier à mettre à jour périodiquement.

Le lot est borné à 100 adresses et un 429 remonte en `RateLimited` nommé — insister sur le palier gratuit (45 req/min) fait bannir l'IP de l'hôte par le fournisseur.

---

## O4 — `deleted_logs` décrit un comportement qui n'existe plus

**Dette introduite par le correctif**, au même titre que A1.

Bannir une IP ne purge plus ses logs : la mesure détruisait les preuves qui la justifiaient, et un aller-retour ban/déban suffisait à nettoyer l'historique de n'importe quelle adresse sans que le journal en garde trace.

Mais le contrat de sortie n'a pas suivi :

- `BanIpOutcome.deleted_logs` (`ops-core`) existe encore et vaut désormais toujours `0` ;
- `handlers/security/bans.rs` le reporte dans l'événement d'audit et dans le message rendu à l'interface, qui annonce donc « 0 logs purgés » ;
- le port `IpBanRepository::delete_api_logs_for_ip` n'a plus d'appelant.

Je ne l'ai pas nettoyé dans le même lot pour ne pas mêler un changement de contrat d'API à un correctif de sécurité. À faire : retirer le champ, la mention dans le message, et le port devenu mort — en vérifiant au passage que l'écran Sécurité ne l'affiche pas côté web.

---

## O5 — Quatre modules non audités

Le passage a couvert le routeur et l'authentification, les adaptateurs Postgres, les sondes fichiers de l'hôte, la file de bans, TLS, GeoIP, l'audit Docker, la configuration et le bloc nginx correspondant.

N'ont **pas** été relus :

- `ops-api/src/container_monitor.rs` — surveillance de fond, écrit des `server_events`
- `ops-api/src/adapters/redis_log_stream.rs` — flux de logs temps réel
- `ops-api/src/handlers/alert_rules.rs` et `adapters/alert_rule_repository.rs` — règles d'alerte, avec webhook Discord
- `ops-api/src/handlers/docker/{prune,images,volumes,networks}.rs` — opérations destructives sur l'hôte

Les deux derniers groupes sont les plus intéressants pour une prochaine passe : les règles d'alerte comportent un envoi sortant (donc une exfiltration possible vers un webhook contrôlé par celui qui écrit la règle), et les handlers de purge appellent des opérations irréversibles.

Sur ce qui a été vérifié : les `format!` SQL de `security_log_repository.rs` et `security_audit_repository.rs` n'interpolent que des entiers typés et des constantes — aucun vecteur d'injection. La file de bans est durcie contre l'injection de ligne (`ban_queue.rs`), et les chemins des sondes viennent d'un enum fermé, donc pas de traversée.

---

## W1 — Dépendances frontend signalées par `npm audit`

Le lockfile fixe `nanoid` à `3.3.11` et `postcss` à `8.5.13`. `npm audit --omit=dev` remonte deux vulnérabilités de gravité élevée avec correctifs disponibles : boucles infinies dans certains générateurs Nano ID et lecture arbitraire de fichiers `.map` par PostCSS lors du traitement d'un `sourceMappingURL` contrôlé.

L'exploitabilité dans le navigateur est faible : ces paquets interviennent surtout pendant la compilation et aucun source map n'est publié dans `dist`. Le risque concerne davantage un pipeline qui compilerait du CSS ou des source maps fournis par un tiers.

Correctif : mettre à jour le lockfile vers `nanoid >= 3.3.17` et `postcss > 8.5.22`, puis rejouer `npm run build`, `npm run lint`, `npm test` et `npm audit`.

## W2 — Échappement incomplet du Markdown rendu avec `v-html`

`web/src/utils/discordMarkdown.ts` échappe `&`, `<` et `>`, mais pas `"` ni `'`. La règle des liens réinjecte ensuite directement l'URL capturée dans `href="$2"`. Une URL Markdown contenant un guillemet peut donc sortir de l'attribut et introduire un nouvel attribut HTML.

La CSP de production (`script-src 'self'`, sans `unsafe-inline`) empêche actuellement l'exécution d'un gestionnaire comme `onmouseover`. Ce n'est toutefois qu'une seconde barrière : le HTML produit par le sanitizer reste incorrect, le serveur de développement n'applique pas cette CSP, et une future relaxation de celle-ci réactiverait immédiatement le vecteur.

Correctif : échapper aussi les deux types de guillemets et construire/valider chaque URL avec `new URL`, idéalement sans générer le HTML par substitutions regex. Ajouter un test avec une URL contenant `"onmouseover=`.

## W3 — Le callback OAuth échoue en s'ouvrant côté interface

`web/src/components/pages/auth/AuthCallbackPage.vue` traite explicitement le 403 comme un refus, mais toute autre erreur de `GET /api/auth/check-access` produit `check-access failed, proceeding anyway`. Le profil issu du fragment OAuth est alors placé dans Pinia et les routes d'administration deviennent navigables.

Les API et les passerelles nginx restent protégées côté serveur : ce point ne donne pas à lui seul accès aux données. Il affaiblit toutefois la défense en profondeur, affiche une interface privilégiée avant validation et transforme toute route backend oubliée en fuite potentielle.

Correctif : ne finaliser le store et la navigation qu'après un 200. Une panne réseau ou un 5xx doit afficher une erreur réessayable, sans conserver de session locale considérée comme autorisée.

## W4 — Clé Bearer historique conservable dans `localStorage`

`web/src/api/config.ts` conserve encore `{ api_url, api_key }` sous `ds.api.config`, et `web/src/api/http.ts` transforme toute valeur non vide en `Authorization: Bearer ...`. Le déploiement actuel initialise cette clé à vide et les secrets Nexus/Ops/Atrium sont correctement injectés par nginx, côté serveur.

Si cette capacité historique était réutilisée, la clé resterait lisible par tout JavaScript exécuté sur l'origine et survivrait aux fermetures de navigateur. Une XSS aurait alors un secret interne durable à exfiltrer.

Correctif : retirer `api_key` du contrat frontend et supprimer l'ajout du Bearer. Si un mode développeur en a réellement besoin, le rendre explicite, limité à `localhost` et stocké au maximum en mémoire ou `sessionStorage`.

---

## N1 — `/nexus-public/` relaie toute l'API de Nexus, sans authentification, clé attachée

**Critique. Exploitable depuis Internet, sans compte, sans information préalable.**

### Le mécanisme

`web/nginx.conf`, bloc de la vitrine publique :

```nginx
location /nexus-public/ {                      # pas d'auth_request — volontaire
    rewrite ^/nexus-public/(.*)$ /$1 break;    # relaie N'IMPORTE QUEL chemin
    proxy_pass $nexus_pub_upstream;
    include /etc/nginx/snippets/nexus-auth.inc;  # injecte le Bearer NEXUS_API_KEY
}
```

Côté Rust, l'intention est respectée : le router `public` (`nexus-api/src/adapters/inbound/http/mod.rs`) ne contient **qu'une seule** route, `/api/public/games/{guild_id}/servers`. Tout le reste vit dans le router `api`, derrière le Bearer.

Mais `location` est un **préfixe**, et le `rewrite` se contente de retirer `/nexus-public/`. Le chemin qui suit est transmis tel quel — y compris s'il désigne une route protégée. Et nginx y ajoute lui-même une clé valide, puisque le snippet est inclus dans ce bloc.

Résultat : le Bearer de `nexus-api` ne protège rien de ce qui passe par cette porte.

```
POST   /nexus-public/api/games/servers/<id>/command    → RCON sur un serveur de jeu
POST   /nexus-public/api/games/<guild_id>/servers      → création de conteneur sur l'hôte
DELETE /nexus-public/api/games/servers/<id>            → suppression
POST   /nexus-public/api/wallet/<guild_id>/transfer    → transfert de monnaie
PUT    /nexus-public/api/config/<guild_id>/<bot_name>  → configuration par serveur
```

### Pourquoi la chaîne est complète

Aucun secret n'est nécessaire pour démarrer : la route légitime de la vitrine **publie les UUID des serveurs** (`PublicGameServerDto.id`, `handlers/game/public_servers.rs`). Un attaquant lit les identifiants là où c'est prévu, puis les rejoue sur les routes d'administration.

Sur Minecraft, RCON donne `op`, `ban`, `stop` — l'administration complète du serveur de jeu.

### Ce qui ne protège pas

- **Le verrou mono-serveur** : les routes `/api/games/servers/{server_id}/...` ne portent pas de `guild_id` dans l'URL, il ne s'applique donc pas. Et là où il s'applique, le `guild_id` est public de toute façon.
- **Le rate limit strict** (2 req/s sur le cycle de vie) : il ralentit, il n'interdit pas. Et il n'est pas atteint par une poignée de requêtes ciblées.
- **L'absence de `ports:` sur `nexus-api`** : le service n'est pas publié sur l'hôte, mais nginx l'est — et c'est nginx qui ouvre la porte.

### Le correctif

Restreindre le `location` au préfixe réellement public, pour que le relais ne puisse pas déborder :

```nginx
location /nexus-public/api/public/ {
    rewrite ^/nexus-public/(.*)$ /$1 break;
    # … reste inchangé
}
```

Le SPA appelle déjà `/nexus-public/api/public/games/{guild}/servers` : le chemin ne change pas côté client.

À vérifier après application : `curl -i https://<domaine>/nexus-public/api/games/<guild>/servers` doit répondre **404** et non 200/401.

---

## N2 — `nexus-api` s'ouvre entièrement quand la clé est absente

`nexus-api/src/bootstrap/mod.rs` :

```rust
let api_key = std::env::var("NEXUS_API_KEY").ok().filter(|k| !k.is_empty());
// …
if api_key.is_none() {
    tracing::warn!("NEXUS_API_KEY absente — API SANS auth (dev uniquement)");
}
```

`None` alimente `OptionalBearerToken`, dont le middleware `require_optional` laisse alors passer **toutes** les routes `/api` — cycle de vie des conteneurs compris. Le compose renforce le risque : `NEXUS_API_KEY: ${NEXUS_API_KEY:-}`, défaut vide.

C'est le défaut déjà corrigé pour Atrium et consigné dans `CLAUDE.md` (« Un secret vide n'est pas un secret absent »). Nexus est passé au travers — alors que c'est la **seule** API capable de lancer des conteneurs sur l'hôte.

Les trois autres échouent en se fermant :

| Service | Comportement sans jeton |
|---|---|
| `ops-api` | `exit(1)` — refuse aussi un jeton < 16 caractères |
| `auth-api` | `AUTH_API_TOKEN` requis par le compose (`:?`) |
| `docker-agent` | refuse de démarrer, et refuse deux jetons identiques |
| **`nexus-api`** | **démarre ouvert, avec un `warn!`** |

### Le correctif

Aligner sur `ops-api` : refuser le démarrage plutôt que de servir ouvert.

```rust
let api_key = std::env::var("NEXUS_API_KEY").ok().filter(|k| k.trim().len() >= 16)
    .unwrap_or_else(|| { tracing::error!("NEXUS_API_KEY manquante ou trop courte"); std::process::exit(1) });
```

Et `NEXUS_API_KEY: ${NEXUS_API_KEY:?NEXUS_API_KEY est requis}` dans `compose.nexus.yml`.

Même précaution que pour S1 : le `.env` du serveur doit contenir la variable **avant** d'appliquer, sinon `docker compose up` s'arrête.

---

## N3 — L'acteur de l'audit est un paramètre d'URL

`nexus-api/src/adapters/inbound/http/handlers/game/servers.rs` :

```rust
pub struct ActorQuery {
    /// Discord user id de l'acteur (audit). Si absent, fallback sur l'owner.
    pub actor_id: Option<String>,
}

async fn resolve_actor(…, explicit: Option<&str>) -> Result<String, ApiError> {
    if let Some(s) = explicit { return Ok(s.to_string()); }   // repris tel quel
    …
}
```

Toute action tracée — RCON, arrêt, suppression de serveur — peut donc être attribuée à n'importe qui, en ajoutant `?actor_id=<autre_personne>`.

C'est la même classe que le point corrigé côté OPS (l'en-tête `x-actor-id` que nginx ne posait pas), en plus facile à exploiter : un paramètre d'URL suffit, sans même forger un en-tête.

### Le correctif

Le même que pour OPS, et il gagne à être fait en même temps : faire descendre l'identité par la passerelle depuis `auth_request`, et **ignorer** ce que le client propose.

```nginx
auth_request_set $nexus_actor $upstream_http_x_auth_user_id;
proxy_set_header X-Actor-Id $nexus_actor;
```

Côté handler, lire l'en-tête et supprimer `ActorQuery`. Le repli sur le propriétaire du serveur reste légitime pour les appels internes du bot, qui n'ont pas d'utilisateur web.

---

## N4 — RCON transmis sans liste blanche

`nexus-core/src/application/game/manage_game_servers_service.rs` : la commande reçue est passée telle quelle au serveur de jeu, après vérification que le serveur tourne et que `rcon_enabled` est vrai. Aucun filtrage du contenu.

Pris isolément, c'est défendable : un panneau d'administration de serveur de jeu sert précisément à exécuter des commandes, et en restreindre la liste reviendrait à réimplémenter la console.

Ça cessait de l'être **combiné à N1** : la commande devenait accessible sans authentification. **N1 étant corrigé (13/08), ce point est redescendu à un choix de produit** — c'est bien la correction de N1, et non une décision sur RCON, qui a retiré le vecteur d'impact.

Si une restriction est souhaitée un jour, la placer dans le domaine (`nexus-core`) et non dans le handler — sinon le bot Discord, qui appelle le même use case, passera à côté.

---

## Périmètre de l'audit Nexus

Couvert : routeur et posture d'authentification, vitrine publique, passerelle nginx, RCON, résolution de l'acteur, verrou mono-serveur.

**Non couvert** — un second passage reste à faire sur :

- l'économie : `wallet`, `coussin`, `casino`, `wheel` (manipulation de soldes, rejeu, courses)
- le Grand Salon (`grand_salon.rs`, motions et votes)
- les adaptateurs Postgres de `nexus-api`
- l'allocateur de ports Redis
- les événements de session (`session_events.rs`) et la surface gRPC
