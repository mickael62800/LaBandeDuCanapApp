# Architecture du projet

## Vue d'ensemble

Le projet est un monorepo Rust organisé autour de quatre univers fonctionnels :

- **Sentinel :** administration, modération et sécurité Discord.
- **NEXUS :** serveurs de jeux, économie et animations Discord.
- **Atrium :** assistant IA et accueil des membres.
- **Ops :** supervision de la machine hôte et des services.

Le frontend commun se trouve dans `web/`. Il affiche plusieurs univers mais ne remplace pas leurs APIs respectives.

## Structure actuelle

| Dossier | Responsabilité |
|---|---|
| `platform-core` | Cœur métier commun, séparé en modules `sentinel`, `nexus`, `atrium` et `ops` |
| anciens `*-core` | Façades temporaires ou sources en attente de migration vers `platform-core` |
| `platform-api` | API HTTP/gRPC unifiee, persistance et adaptateurs externes |
| `*-bot` | Connexion Discord, commandes et publication des messages |
| `platform-scheduler` | Planification HTTP commune des tâches périodiques |
| `*-gateway` | Point d'entrée ou relais réseau lorsqu'il existe |
| `platform-proto` | Contrats protobuf/gRPC partages |

## Dossiers principaux

### Sentinel

- `platform-core/src/sentinel` : modération, infractions, AutoMod, communauté et règles.
- `platform-api` : API principale et accès Discord/PostgreSQL/Redis.
- `sentinel-bot` : commandes et événements Discord Sentinel.
- `ops-agent` : collecte hôte, surveillance Docker et monitoring stateful des services.

### NEXUS

- `platform-core/src/nexus` : serveurs de jeu, wallets, roue, Coussin et jeux.
- `platform-api` : API NEXUS, base dédiée et orchestration du runtime de jeu.
- `nexus-bot` : portail de jeux et commandes NEXUS.
- `platform-scheduler` : déclenche les jobs NEXUS via `platform-api`.

### Atrium

- `platform-core/src/atrium` : règles métier de réponse, accueil et apaisement.
- `platform-api` : API d'administration, génération IA, quotas, RAG et mémoire.
- `atrium-bot` : réception des messages Discord et publication des réponses.
- `platform-scheduler` : déclenche les jobs Atrium via `platform-api`.

### Ops

- `platform-core/src/ops` : sondes machine, contrats Docker, sécurité hôte et logs.
- `platform-api` : API de supervision et d'administration technique.
- `ops-agent` : collecte hôte, surveillance Docker et monitoring des services.
- `platform-scheduler` : déclenche l'évaluation des alertes dans `platform-api`.
- `docker-agent` : accès contrôlé aux opérations Docker.

## Services partagés

- `platform-core/` : règles métier unifiées, avec une frontière de module par entité.
- `web/` : frontend Vue/TypeScript et navigation multi-univers.
- `platform-common/` : contrats et composants communs sans règle métier de plateforme.
- `platform-api/src/shared/` : utilitaires HTTP communs aux domaines API.
- `platform-common-bot/` : utilitaires communs aux bots Discord.
- `platform-scheduler/` : planificateur HTTP thin commun aux plateformes.
- `DOC/REFERENCE-IA/scheduler.md` : contrat de sécurité, verrouillage et observabilité des jobs.
- `ops-agent/` : runtime des collectes et monitorings nécessitant Redis ou l'accès hôte.
- `auth-core` et `auth-api` : identité, OAuth Discord et sessions web.
- `infrastructure/` : Docker, PostgreSQL, Redis, Nginx, Prometheus et Grafana.

## Flux principaux

### Dashboard

`Navigateur → web/Nginx → passerelle de la plateforme → API → platform-core::<entité> → base ou service externe`.

Le frontend utilise une passerelle spécifique pour NEXUS, Atrium et Ops. Les secrets backend ne doivent jamais être embarqués dans le navigateur.

### Discord

`Discord → bot → API ou gRPC → core → base → bot → Discord`.

Un bot ne doit pas accéder directement à la base de données.

### Tâches en arrière-plan

`platform-scheduler → endpoint interne de l'API → traitement métier → événement, log ou notification`.

Une tâche doit pouvoir être relancée sans créer de doublon lorsque le métier l'exige.

## Bases et bus

- Chaque plateforme possède sa base logique et son rôle applicatif.
- `sentinel:events` est réservé à Sentinel et à ses consommateurs autorisés.
- `nexus:events` est réservé à NEXUS.
- Les événements ne doivent pas être envoyés sur le bus d'une autre plateforme.
- Redis, PostgreSQL et les tokens sont des dépendances d'infrastructure, pas des règles métier.

## Rules d'orientation pour une IA

- Une règle de décision va dans `platform-core` sous le module du domaine concerné (`sentinel`, `nexus`, `atrium`, `ops`).
- Une route HTTP ou gRPC appartient à `platform-api`, dans le module de domaine correspondant.
- Une interaction Discord appartient au bot concerné (`sentinel-bot`, `nexus-bot`, `atrium-bot`).
- Une tâche périodique appartient au domaine dans `platform-scheduler` (qui déclenche les endpoints internes de `platform-api`).
- Une opération machine appartient à Ops, même si elle sert NEXUS ou Atrium.
- L'identité et les sessions appartiennent à `auth-api` / `auth-core`.



