# Architecture cible - état de reprise

Dernière mise à jour : **13 août 2026**.

Ce document décrit l'état réel du dépôt après les consolidations et indique
comment poursuivre sans devoir reconstituer l'historique des migrations.

## 1. Décision d'architecture

La plateforme conserve les processus qui ont une vraie contrainte d'exécution
ou une frontière de sécurité distincte :

- trois bots Discord : `sentinel-bot`, `nexus-bot`, `atrium-bot` ;
- `auth-api`, qui détient les sessions et les jetons OAuth ;
- `docker-agent`, seul composant autorisé à accéder au socket Docker ;
- `ops-agent`, chargé des sondes de l'hôte ;
- `platform-scheduler`, planificateur sans logique métier ;
- la surface applicative portée par `platform-api`.

Le découpage Sentinel, Nexus, Atrium et Ops reste présent **dans le code**, sous
forme de modules. Il ne doit plus produire des crates `*-core`, `*-proto`,
`*-worker` ou `*-api` indépendantes.

## 2. État actuel

### 2.1 Consolidations terminées

| Chantier | État | Résultat |
|---|---:|---|
| Workers | Terminé | Les workers métier ont été absorbés par `platform-scheduler` ; les anciens workers ont été supprimés |
| Cores | Terminé | `platform-core` contient `sentinel`, `nexus`, `atrium`, `ops` et `shared` |
| Protos | Terminé | `platform-proto` contient les contrats gRPC de la plateforme ; les anciens crates proto ont été supprimés |
| APIs | Terminé au niveau Cargo | `platform-api` contient les quatre domaines ; les anciens dossiers `sentinel-api`, `nexus-api`, `atrium-api` et `ops-api` ont été supprimés |

Arborescence de référence :

```text
platform-core/src/
  atrium/
  nexus/
  ops/
  sentinel/
  shared/

platform-api/src/
  atrium/
  nexus/
  ops/
  sentinel/
  bin/
    atrium-api.rs
    nexus-api.rs
    ops-api.rs
    sentinel-api.rs

platform-scheduler/src/
  domains/

platform-proto/
  proto/
  src/
```

### 2.2 Point important : crate unique, processus encore séparés

`platform-api` est un seul crate Rust, mais il produit encore quatre binaires :

- `sentinel-api` ;
- `nexus-api` ;
- `atrium-api` ;
- `ops-api`.

Docker continue donc à lancer quatre services API. Cette compatibilité a été
conservée volontairement pour migrer le code sans changer en même temps les
ports, les healthchecks, nginx, les secrets et les dépendances Compose.

La consolidation des crates API est terminée. La consolidation en **un seul
processus API** ne l'est pas encore.

## 3. Responsabilités des crates consolidées

### `platform-core`

Contient uniquement le métier, les ports et les cas d'usage :

- `platform_core::sentinel` : modération, audit, communauté, sécurité, IA,
  sauvegardes et fonctions système ;
- `platform_core::nexus` : jeux, serveurs, RCON, économie et Grand Salon ;
- `platform_core::atrium` : accueil, mémoire, génération et apaisement ;
- `platform_core::ops` : exploitation, alertes, sécurité hôte et journaux ;
- `platform_core::shared` : types réellement transverses.

Une règle métier ne doit pas être ajoutée dans un handler HTTP, un bot ou le
scheduler.

### `platform-proto`

Source unique des contrats gRPC. Toute modification de contrat doit être faite
ici, puis propagée aux consommateurs. Ne pas recréer un crate proto par produit.

### `platform-scheduler`

Worker unique et volontairement thin. Il sait :

- quand déclencher un job ;
- quel endpoint interne appeler ;
- comment journaliser le résultat et appliquer le retry prévu.

Il ne doit pas contenir de repository métier, de requête SQL, d'accès direct à
Redis, Docker, Discord ou `/proc`. `ops-agent` reste séparé pour les sondes hôte.

### `platform-api`

Contient les adaptateurs entrants/sortants, la composition des états et les
migrations des quatre domaines. Les modules sont séparés :

- `platform_api::sentinel` ;
- `platform_api::nexus` ;
- `platform_api::atrium` ;
- `platform_api::ops`.

Les migrations sont isolées sous :

```text
platform-api/migrations/atrium/
platform-api/migrations/nexus/
platform-api/migrations/sentinel/
platform-api/migrations_legacy/sentinel/
```

Sentinel possède actuellement **34 migrations actives** et **370 fichiers de
migrations historiques**. Ne pas supprimer l'historique avant d'avoir vérifié
la stratégie de restauration et les environnements déjà déployés.

## 4. Prochaine étape recommandée

### Estimation du chantier restant

La fusion est complète au niveau du code et de Cargo, mais reste partielle au
niveau de l'exécution. Le travail restant représente environ **une journée**
pour une transition prudente, et jusqu'à **deux jours** avec une validation
complète en environnement réel.

| Travail | Estimation |
|---|---:|
| Créer un binaire unifié en conservant les quatre ports actuels | 2 à 4 heures |
| Adapter Docker et valider les quatre domaines | 2 à 4 heures |
| Passer ensuite sur un port unique et nettoyer nginx | 3 à 6 heures |
| Exécuter les tests PostgreSQL et corriger les éventuels écarts | 2 à 4 heures, selon l'environnement |

Le chemin le moins risqué consiste à lancer les quatre surfaces depuis un seul
processus tout en conservant temporairement leurs ports actuels. Cela évite de
modifier simultanément nginx, les bots, le scheduler et toutes les URL internes.
Le passage à un port unique vient seulement après validation de cette étape.

### Étape 1 - Créer un vrai binaire API unifié

Ajouter un binaire, par exemple `platform-api`, qui démarre les quatre domaines
dans un seul processus. Deux options sont possibles :

1. un serveur HTTP unique avec quatre routeurs imbriqués ;
2. plusieurs listeners dans le même processus pour conserver temporairement les
   ports actuels.

L'option 2 est la transition la moins risquée : nginx, les bots, le scheduler et
les healthchecks continuent d'utiliser les mêmes ports. Une fois validée, les
routes peuvent être réunies sur un port unique.

Le processus unifié doit :

- initialiser tracing et les métriques une seule fois ;
- construire les pools PostgreSQL et clients Redis nécessaires ;
- exécuter chaque groupe de migrations avec son chemin explicite ;
- démarrer les serveurs HTTP/gRPC sans masquer une erreur de démarrage ;
- arrêter tous les domaines proprement sur SIGTERM/Ctrl+C ;
- conserver les limites de taille, rate limits, middlewares d'authentification
  et en-têtes de sécurité propres à chaque surface.

### Étape 2 - Basculer Docker sans changer les routes publiques

Remplacer les quatre conteneurs API par un service `platform-api`, tout en
conservant les noms DNS historiques avec des alias réseau si nécessaire :

- `sentinel-api` ;
- `nexus-api` ;
- `atrium-api` ;
- `ops-api`.

Nginx doit continuer à exposer `/api/`, `/nexus-api/`, `/atrium-api/` et
`/ops-api/` pendant cette phase. Ne pas unifier les préfixes en même temps que
le changement de processus.

### Étape 3 - Supprimer les quatre binaires de compatibilité

Après validation en environnement réel :

- supprimer `src/bin/sentinel-api.rs` ;
- supprimer `src/bin/nexus-api.rs` ;
- supprimer `src/bin/atrium-api.rs` ;
- supprimer `src/bin/ops-api.rs` ;
- retirer leurs targets Docker/Bake ;
- conserver uniquement le nouveau binaire `platform-api`.

### Étape 4 - Nettoyer les dépendances devenues communes

Une fois le runtime unifié, factoriser uniquement ce qui est réellement commun :

- initialisation tracing/métriques ;
- arrêt gracieux ;
- clients d'infrastructure partagés ;
- authentification de service ;
- construction des réponses d'erreur.

Ne pas fusionner les `AppState` métier en une structure plate. Le nouvel état
racine doit rester un agrégat de quatre sous-états.

### Étape 5 - Valider puis nettoyer Compose et Bake

Contrôler les dépendances, healthchecks, secrets, volumes, réseaux et cibles de
build avant de retirer les anciennes définitions. Les aliases temporaires ne
doivent être supprimés qu'après migration de tous les consommateurs.

## 5. Invariants de sécurité

1. Le socket Docker reste monté uniquement dans `docker-agent`.
2. `ops-agent` reste le seul composant chargé de lire les métriques de l'hôte.
3. Une configuration TLS invalide doit arrêter le serveur ou le client ; aucun
   fallback HTTP clair n'est autorisé.
4. Les secrets absents, vides ou trop courts provoquent un échec fermé.
5. L'identité de l'acteur vient d'une frontière serveur de confiance, jamais du
   navigateur.
6. Les permissions Discord sensibles sont revérifiées dans les handlers.
7. Les routes Nexus de cycle de vie Docker conservent leur rate limit strict.
8. Les routes publiques Nexus restent limitées au préfixe public prévu.
9. Les bots n'accèdent pas directement aux bases de données.
10. Le scheduler ne reçoit aucun privilège métier ou hôte supplémentaire.
11. Les sous-états Sentinel, Nexus, Atrium et Ops restent isolés même dans un
    processus commun.

## 6. État des validations au 13 août 2026

Validations réussies après la dernière migration :

```powershell
cargo fmt -p platform-api -- --check
cargo check --workspace --quiet
cargo clippy -p platform-api --all-targets -- -D warnings
docker compose -f infrastructure/docker/compose.core.yml `
  -f infrastructure/docker/compose.nexus.yml `
  -f infrastructure/docker/compose.atrium.yml `
  config --quiet --no-interpolate
docker buildx bake -f infrastructure/docker/docker-bake.hcl --print
```

Résultats des tests `platform-api` :

- 718 tests Sentinel réussis ;
- 3 tests ignorés ;
- 13 tests SQLx nécessitent `DATABASE_URL` et une instance PostgreSQL de test ;
- les tests Atrium, Nexus et Ops passent dans l'environnement local.

Dette connue, antérieure à la consolidation :

- `platform-api/src/sentinel/adapters/inbound/http/handlers/system/internal_jobs.rs`
  accède directement à `state.pg_pool` ;
- le test `sentinel_architecture_state_test` le signale. Il faut déplacer cette
  orchestration derrière un port/use case plutôt que désactiver le garde-fou.

## 7. Commandes de reprise

Commencer par vérifier l'état local :

```powershell
cargo metadata --no-deps --format-version 1
cargo check --workspace
cargo test -p platform-api --lib
cargo clippy -p platform-api --all-targets -- -D warnings
```

Pour exécuter les tests SQLx, définir une base de test jetable :

```powershell
$env:DATABASE_URL = "postgres://USER:PASSWORD@localhost:5432/TEST_DATABASE"
cargo test -p platform-api --lib
```

La base doit être dédiée aux tests : les attributs `#[sqlx::test]` peuvent
créer, migrer ou nettoyer des bases et schémas temporaires.

## 8. Critère de fin

La migration d'architecture sera réellement terminée quand :

- `platform-core`, `platform-proto`, `platform-scheduler` et `platform-api`
  seront les seules briques applicatives consolidées ;
- `platform-api` ne produira plus qu'un binaire applicatif ;
- Docker ne lancera plus qu'un conteneur API ;
- nginx, les bots et le scheduler utiliseront ce service sans alias historique ;
- tous les tests, y compris PostgreSQL, passeront ;
- le garde-fou d'architecture Sentinel repassera au vert.

Les trois bots, `auth-api`, `docker-agent` et `ops-agent` resteront séparés : ce
sont des frontières d'exécution ou de sécurité, pas des duplications métier.

## 9. Chantiers restant après la consolidation des APIs

### 9.1 Fusion runtime des APIs

La priorité immédiate reste le passage des quatre binaires et conteneurs API à
un seul processus `platform-api`. Le crate est déjà fusionné ; il faut encore
unifier le démarrage, les listeners, Docker, Bake et progressivement nginx.

### 9.2 Gateways

Deux gateways existent encore :

- `sentinel-gateway` ;
- `nexus-gateway`.

Avant de les fusionner, cartographier leurs connexions, protocoles, états et
contraintes de disponibilité. Si elles sont seulement des adaptateurs réseau,
elles peuvent devenir des modules d'un même crate ou processus. Si leur
isolement protège une connexion longue durée ou limite l'impact d'un plantage,
conserver des processus distincts mais mutualiser leur code commun.

Atrium ne possède pas de gateway dédiée.

### 9.3 Composants communs

Les crates suivantes restent à évaluer :

- `platform-common` ;
- `platform-common-api` ;
- `platform-common-bot`.

Leur séparation est actuellement défendable car leurs graphes de dépendances
sont différents : le socle générique ne doit pas tirer Axum, et les bots ne
doivent pas tirer les dépendances serveur. Ne les regrouper que si cela réduit
réellement la duplication sans mélanger ces frontières.

### 9.4 Adaptateurs Ops

`ops-adapters` est encore partagé par `platform-api` et les composants Ops.
Vérifier ses consommateurs avant toute suppression. Les adaptateurs réellement
communs peuvent rester dans cette crate ; ceux utilisés uniquement par un
composant doivent rejoindre le module propriétaire.

### 9.5 Nettoyage du déploiement

Après les migrations runtime :

- retirer les anciens services et targets de Compose/Docker Bake ;
- retirer les anciens noms DNS lorsqu'aucun consommateur ne les utilise ;
- simplifier nginx sans changer plusieurs frontières dans une même bascule ;
- supprimer les healthchecks, volumes, variables et secrets devenus inutiles ;
- vérifier que les images sont construites depuis les crates consolidées.

### 9.6 Validation finale

La validation de fin doit couvrir :

- les tests PostgreSQL avec `DATABASE_URL` ;
- la suppression de l'accès direct à `state.pg_pool` dans `internal_jobs.rs` ;
- les routes HTTP publiques et internes ;
- les services gRPC et leur mTLS ;
- les trois bots Discord ;
- tous les jobs du scheduler ;
- les opérations Docker via `docker-agent` ;
- l'arrêt gracieux du processus API unifié.

### 9.7 Processus qui doivent rester séparés

Sauf nouvelle décision explicitement justifiée, ne pas absorber :

- `sentinel-bot`, `nexus-bot` et `atrium-bot` : identités Discord et connexions
  gateway différentes ;
- `auth-api` : sessions et jetons OAuth, frontière de sécurité dédiée ;
- `docker-agent` : accès privilégié au socket Docker ;
- `ops-agent` : accès aux sondes et métriques de l'hôte.

Ces processus sont des frontières de sécurité ou de disponibilité. Leur
séparation ne constitue pas une duplication métier à éliminer.
