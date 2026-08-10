# Audit architecture — Univers « Exploitation » (`ops`)

_Date : 2026-08-10 — périmètre : `ops-core`, `ops-api`, et les points de contact avec sentinel / nexus / atrium._

## Verdict global : architecture très propre ✅

Le domaine ops a été correctement **extrait en plateforme hexagonale autonome**
(`ops-core` + `ops-api`), au même rang que sentinel / nexus / atrium. Ce n'est
plus un sous-dossier de `sentinel-core`.

### Ce qui est bon

- **`ops-core` ne dépend d'aucune plateforme** : uniquement `platform-common`,
  `serde`, `chrono`, `uuid`, `async-trait`. Domaine pur — aucune infra
  (`sqlx` / `reqwest` / `serenity` / `axum`) dans `ops-core/src/domain/`.
- **Sens des dépendances correct** : `sentinel-api`, `sentinel-worker` et
  `docker-agent` dépendent d'`ops-core`, **jamais l'inverse**. Conforme à
  l'inversion documentée dans `ops-core/src/lib.rs`.
- **Aucun débordement de code vers nexus / atrium / sentinel**. Les seules
  occurrences de « sentinel » dans le code ops sont :
  - des chemins de fichiers (`/var/lib/sentinel/…`) partagés avec des crons
    hôte — legacy de nommage, pas de couplage ;
  - des commentaires historiques.
- **`ops-api` a son propre montage nginx** (`/ops-api/`, façon nexus/atrium) et
  sa propre base via un rôle SQL restreint. Cohérent avec le reste.
- La duplication de `validation` (2 constantes) dans
  `ops-core/src/application/mod.rs` est **volontaire et commentée** : justifiée
  pour éviter une dépendance ops → sentinel.

### Faux positifs vérifiés (RAS)

- Domaine worker `sentinel-worker/src/domains/security/` (lockdown, slowmode,
  quarantaine) = métier **Discord**, pas sécurité hôte. Bien rangé.
- `sentinel-worker/src/domains/monitoring/check_services.rs` = registre des
  services via Redis, pas de la supervision hôte. Bien rangé.

## Points à revoir 🔧

### 1. Dépendance morte — `ops-core` dans le worker

`sentinel-worker/Cargo.toml` (ligne 13) déclare :

```toml
ops-core = { workspace = true }
```

…mais **aucune source du worker ne l'utilise** (`grep ops_core sentinel-worker/src`
→ vide). **À supprimer.**

### 2. Dette de documentation — `CLAUDE.md` périmé

Le `CLAUDE.md` décrit toujours ops comme un domaine **interne** à
`sentinel-core`, ce qui n'est plus vrai depuis l'extraction :

- « Le découpage est le même à tous les étages — `application/ops/`,
  `ports/{inbound,outbound}/ops/`, `domain/entities/ops/` »
  → **ces dossiers n'existent plus** dans `sentinel-core`.
- « Sept sous-états : … `ops` … » et « Domaines de
  `sentinel-core/src/application/` : …, `ops`, `system` »
  → ops n'est plus un domaine de `sentinel-core`.

À corriger : refléter que le métier ops vit désormais dans `ops-core` / `ops-api`,
et que `OpsState` (dans `sentinel-api/src/bootstrap/state/ops.rs`) n'est plus
qu'un **consommateur** de ports `ops-core` (`SystemProbe`, `ServiceRegistry`,
`LogRepository`) côté sentinel-api — ce que le commentaire de ce fichier
explique déjà correctement.

## Résumé actions

| # | Action | Fichier | Risque |
|---|--------|---------|--------|
| 1 | Retirer la dépendance `ops-core` inutilisée | `sentinel-worker/Cargo.toml` | Nul |
| 2 | Mettre à jour la section ops (extraction en `ops-core`/`ops-api`) | `CLAUDE.md` | Nul (doc) |

---

# Audit architecture — Plateforme `sentinel` (référence)

_Ajout 2026-08-10 — mêmes critères : étanchéité hexagonale, sens des
dépendances, débordement vers nexus / atrium / ops._

## Verdict global : exemplaire ✅

Sentinel tient bien son rôle de plateforme de référence. Toutes les règles d'or
du `CLAUDE.md` sont respectées.

### Étanchéité hexagonale

- **Domaine pur** : aucun `use sqlx|serenity|reqwest|axum|redis` dans
  `sentinel-core/src/domain/`.
- **`application` ne dépend d'aucun adaptateur / framework**.
- **Le bot n'a pas d'accès DB** : aucun `sqlx` / `PgPool` dans `sentinel-bot/src`.
- **Aucune I/O sortante dans un handler inbound** : `reqwest::Client`
  n'apparaît que dans `handlers/system/oauth.rs` — la seule exception
  documentée (flux CSRF/cookies OAuth2).
- Pas de validation de snowflake dupliquée, pas de parser de config réécrit
  dans sentinel.

### Pas de débordement cross-plateforme

- Aucune référence `nexus` / `atrium` dans `sentinel-core/src`.
- `sentinel-api` ne dépend d'**aucun crate** nexus/atrium ; sa seule dépendance
  transverse est `ops-core` (consommation de ports ops : `SystemProbe`,
  `ServiceRegistry`, `LogRepository`) — sens de dépendance correct.
- Le lien vers Nexus passe par un **client HTTP** (`adapters/outbound/nexus_games.rs`),
  pas par un couplage de code — et c'est architecturalement justifié :
  sentinel-api est le seul à connaître l'identité de session, il dérive le
  `user_id` côté serveur au lieu de le laisser dans l'URL. Frontière d'auth
  correcte.

### Dettes du `CLAUDE.md` déjà résorbées

- **`sentinel-bot/src/shared/embeds.rs`** : le `CLAUDE.md` le décrit comme une
  « copie octet pour octet des 178 lignes ». Ce n'est **plus vrai** : c'est
  désormais un `pub use platform_common_bot::embeds::*` (15 lignes). Le
  `CLAUDE.md` est à corriger sur ce point.
- **`#[allow(dead_code)]`** : 6 occurrences, toutes dans les DTO miroirs des
  `api_client.rs` des modules bot — conforme à la justification documentée
  (contrats d'API dont le bot ne lit qu'une partie des champs).

## Points mineurs à revoir 🔧

### 3. Répertoire d'artefacts parasite — `sentinel-web/`

`sentinel-web/` ne contient que `dist/` + `node_modules/` (sorties de build).
Il est **untracked et gitignoré** — donc sans impact sur le dépôt, mais c'est
de l'encombrement local trompeur (la source web vit dans `web/`). À supprimer
localement si inutilisé.

### 4. (Hors sentinel, pour mémoire) copie inline du parser dans nexus-core

`nexus-core/src/domain/entities/system/bot_config.rs` contient toujours la copie
inline de `parse_enabled_flag` au défaut inversé (absent = activé) signalée par
la règle 5 du `CLAUDE.md` — « appelée par personne, à supprimer ». C'est une
dette **nexus**, hors périmètre sentinel, mentionnée ici seulement pour ne pas
la perdre.

## Résumé actions (sentinel)

| # | Action | Fichier | Risque |
|---|--------|---------|--------|
| 3 | Supprimer le répertoire d'artefacts local | `sentinel-web/` | Nul (untracked) |
| 4 | Corriger `CLAUDE.md` : `embeds.rs` est déjà un `pub use` | `CLAUDE.md` | Nul (doc) |

---

# Audit architecture — Plateforme `nexus`

_Ajout 2026-08-10 — mêmes critères._

## Verdict global : propre ✅

### Étanchéité hexagonale

- **Domaine pur** : aucun `use sqlx|serenity|reqwest|axum|redis` dans
  `nexus-core/src/domain/`.
- **`application` sans framework**, **bot sans accès DB**, **aucun `reqwest`
  dans les handlers inbound**.
- **Aucune dépendance de crate** vers sentinel / atrium / ops (la seule
  occurrence dans les `Cargo.toml` est un commentaire « mêmes features que
  sentinel-api »).

### Dette du `CLAUDE.md` déjà résorbée

- La règle 5 du `CLAUDE.md` dénonce dans `nexus-core/.../system/bot_config.rs`
  une « copie inline dont le défaut est inversé (absent = activé), appelée par
  personne, à supprimer ». **C'est déjà fait** : `parse_enabled_flag` /
  `cfg_enabled` ont été retirés, l'en-tête du module le documente, et le
  `cfg_bool` restant est appelé **fail-closed** (`cfg_bool(&cfg, "…_enabled",
  false)`). Le `CLAUDE.md` est à corriger sur ce point.

## Points mineurs à revoir 🔧

### 5. Nommage `sentinel.*` porté par nexus-core

`nexus-core/src/application/game/` étiquette les conteneurs de jeu avec des
labels `sentinel.server_id`, `sentinel.guild_id`, `sentinel.owner`…, un réseau
`sentinel-games` et un répertoire hôte `/var/lib/sentinel/games`
(`manage_game_servers_service.rs`, `config_loader.rs`, `worker_jobs.rs`).

Ce n'est **pas** un couplage de code (aucun crate sentinel importé) : c'est une
convention d'infrastructure hôte partagée. Mais le préfixe entretient
exactement l'ambiguïté du mot « serveur » que le `CLAUDE.md` demande de lever.
Piste : préfixe neutre (`games.*` / `nexus.*`) au prochain changement de
schéma de labels — non urgent, purement cosmétique.

### 6. Petit doublon de parser booléen

`config_loader.rs:70` définit son propre `parse_bool` (accepte en plus
`on/off`), là où la sémantique de référence est
`sentinel-core/.../config_parsers.rs`. Divergence mineure et volontaire
(parsing de config de jeu, valeurs admin), mais l'en-tête de `bot_config.rs`
recommande justement de « l'aligner sur ce module plutôt que d'en réinventer
une variante locale ». À harmoniser si l'occasion se présente.

## Résumé actions (nexus)

| # | Action | Fichier | Risque |
|---|--------|---------|--------|
| 5 | (Cosmétique) préfixe de labels neutre au prochain changement de schéma | `nexus-core/.../game/` | Faible |
| 6 | (Cosmétique) aligner `parse_bool` sur la sémantique de référence | `nexus-core/.../game/config_loader.rs` | Faible |
| — | Corriger `CLAUDE.md` règle 5 : le parser inline mort est déjà supprimé | `CLAUDE.md` | Nul (doc) |

---

# Audit architecture — Plateforme `atrium`

_Ajout 2026-08-10 — mêmes critères. Plus jeune et plus petite plateforme
(≈18 fichiers core)._

## Verdict global : irréprochable ✅

### Étanchéité hexagonale

- **Domaine pur**, **`application` sans framework**, **bot sans accès DB**,
  **aucun `reqwest` dans les handlers inbound**.
- **Aucun débordement** : zéro référence sentinel / nexus / ops dans
  `atrium-core/src`, et **aucune dépendance de crate** vers une autre
  plateforme dans les `Cargo.toml`.
- Structure minimale et correcte : `application/welcome/`,
  `domain/entities/welcome.rs`, `ports/inbound/welcome/`,
  `ports/outbound/ai/`. Un seul domaine métier (accueil assisté par IA),
  découpage hexagonal respecté.

## Points mineurs à revoir 🔧

### 7. Répertoires de crates totalement vides

`atrium-gateway/`, `atrium-web/` et `atrium-ml/` existent mais sont
**complètement vides** (aucun fichier, pas même un `Cargo.toml`) et **absents
du workspace** (`Cargo.toml` racine ne les liste pas). Ce sont des stubs
d'arborescence.

→ Confirme au passage que le `CLAUDE.md` a raison : atrium « n'a ni gateway ni
sous-états d'API » — le dossier `atrium-gateway/` n'est qu'une coquille vide.
À supprimer pour lever l'ambiguïté (un dossier `atrium-gateway/` laisse croire
à une gateway qui n'existe pas).

### 8. Modules de domaine en coquille vide

`atrium-core/src/domain/enums/mod.rs` et `domain/services/mod.rs` ne
contiennent qu'un commentaire de doc, aucun type. Scaffolding mort. À
supprimer, ou à laisser tel quel si l'on anticipe leur remplissage proche
(dans ce cas, l'intention gagnerait à être écrite dans le commentaire).

## Résumé actions (atrium)

| # | Action | Fichier | Risque |
|---|--------|---------|--------|
| 7 | Supprimer les répertoires de crates vides | `atrium-gateway/`, `atrium-web/`, `atrium-ml/` | Nul |
| 8 | Supprimer les mods de domaine vides | `atrium-core/.../domain/{enums,services}/mod.rs` | Faible |

---

# Constat transverse : `CLAUDE.md` en avance de phase inversée

Les quatre audits convergent : **plusieurs dettes signalées par le `CLAUDE.md`
sont déjà résolues dans le code**. Le fichier décrit un état antérieur.

| Dette annoncée dans `CLAUDE.md` | État réel |
|---|---|
| `ops` = domaine interne à `sentinel-core` (application/ports/domain) | Extrait en `ops-core` / `ops-api` |
| `sentinel-bot/.../embeds.rs` = copie octet pour octet | Déjà un `pub use` (15 lignes) |
| `nexus-core` parser inline mort au défaut inversé | Déjà supprimé (documenté) |
| Sept sous-états dont `ops` dans sentinel-api | `ops` est un consommateur de `ops-core`, plus un domaine sentinel |

**Recommandation** : une passe de resynchronisation du `CLAUDE.md` avec l'état
réel du code, pour qu'il reste une carte fiable et non un historique.
