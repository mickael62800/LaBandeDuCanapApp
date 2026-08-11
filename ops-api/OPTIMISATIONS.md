# Audit d'optimisation Ops

Date de l'audit : 11 aout 2026
Mise a jour : 11 aout 2026 — priorites P0 a P11 implementees.

## Etat d'avancement

| Priorite | Statut |
|---|---|
| P0 — metrique auth failures (JSONB `status_code`) | ✅ Fait |
| P0 — reemission des changements Docker | ✅ Fait (curseur Redis `alert:docker:cursor`) |
| P1 — `sysinfo` hors runtime async | ✅ Fait (`spawn_blocking`) |
| P2 — allers-retours Redis | ✅ Fait (worker : connexion partagee/tick + `EXISTS` pipelines ; API : `ConnectionManager` partage) |
| P3 — boucles periodiques | ✅ Fait (`tokio::time::interval` + `MissedTickBehavior::Skip`, premier tick immediat) |
| P4 — dispatcher de webhooks | ✅ Fait (concurrence bornee a 3, `Retry-After` respecte, resume par cycle) |
| P5 — monitor Docker | ✅ Fait (`record_batch` par lot, plus de clone complet, un seul verrou) |
| P6 — endpoints Docker | ✅ Fait (`try_join!` overview, total prune reseaux inclus, echec partiel expose) |
| P7 — audit Docker | ✅ Fait (resultat reel apres l'appel, awaite, wrapper `audited` mutualise) |
| P8 — cleanup explicite | ✅ Fait (transaction locale + erreurs propagees + **statut par cible** `deleted`/`skipped`/`failed`) |
| P9 — liveness/readiness | ✅ Fait (`/health` liveness, `/ready` readiness PG+Redis, healthcheck compose sur `/ready`) |
| P10 — decoupler `ops-worker` de `ops-api` | ✅ Fait (crate `ops-adapters`) — reste : sortir axum via `docker_agent_client` |
| P11 — decouper les god files | ✅ Fait (`handlers/docker/*` et `handlers/security/*`, sans re-export) |
| P12 — index des logs | ⏳ A mesurer (`EXPLAIN ANALYZE`) avant migration — candidats listes plus bas |
| Doc a remettre en coherence | ✅ Fait (monitor worker, role `sentinel_app`, socket Docker) |
| Couverture de tests d'integration | ⏳ Necessite un harnais Postgres de test |

Le detail d'origine de chaque priorite est conserve ci-dessous pour reference.

## Etat initial

- Les 33 tests des crates `ops-core`, `ops-api` et `ops-worker` passent.
- Clippy passe sur toutes les cibles sans avertissement.
- `ops-api` ne possede pratiquement aucun test de handler ou d'integration.
- Aucun changement fonctionnel n'a ete applique pendant cet audit.

## Priorite 0 - Reparations fonctionnelles

### Corriger la metrique des echecs d'authentification

Le dispatcher execute actuellement cette condition :

```sql
AND status_code IN (401, 403)
```

La table `logs` ne possede pas de colonne `status_code`. Le code HTTP range
cette valeur dans le document JSONB `details`. La requete echoue donc a chaque
cycle, puis `unwrap_or(0)` transforme silencieusement l'erreur en compteur nul.
L'alerte `auth_failures_1h` ne peut jamais se declencher.

### Proposition

- Utiliser `(details->>'status_code')::int IN (401, 403)`, comme le fait deja
  `PgSecurityLogRepository`.
- Ne plus masquer l'erreur SQL : journaliser l'echec et distinguer une metrique
  indisponible d'une valeur reellement egale a zero.
- Ajouter un test d'integration couvrant des logs 200, 401 et 403.

### Fichiers concernes

- `ops-worker/src/alerts_dispatcher.rs`
- `ops-api/src/adapters/security_log_repository.rs`

### Ne plus reemettre les anciens changements Docker

Le dispatcher parcourt les 200 changements conserves dans
`ContainerMonitorState` a chaque cycle. La cle Redis empeche les doublons
pendant le cooldown, mais une fois ce TTL expire, le meme evenement historique
peut etre envoye de nouveau tant qu'il n'a pas ete chasse de la liste.

### Proposition

- Memoriser le dernier timestamp ou identifiant traite par le dispatcher ; ou
- fournir au dispatcher uniquement les changements du dernier relevé ; ou
- publier les changements dans une file Redis consommable plutot que relire un
  historique servant avant tout a l'affichage.

Le curseur persistant est la solution la plus robuste aux redemarrages.

## Priorite 1 - Ne pas bloquer le runtime asynchrone

`collect_host_resources` attend 200 ms avec `std::thread::sleep` pour obtenir
deux echantillons CPU. Cette fonction est appelee directement depuis une tache
Tokio et bloque donc l'un des threads du runtime.

### Proposition

Executer toute la collecte `sysinfo` dans `tokio::task::spawn_blocking`. Le
delai entre les mesures reste necessaire, mais il ne doit pas immobiliser le
runtime asynchrone.

### Fichier concerne

- `ops-worker/src/alerts_dispatcher.rs`

## Priorite 2 - Reduire les allers-retours Redis

La collecte des services offline effectue actuellement :

1. une connexion Redis ;
2. un `SMEMBERS bots:known` ;
3. un `EXISTS bot:online:{name}` attendu sequentiellement pour chaque service.

Chaque tentative de deduplication ouvre ensuite une nouvelle connexion. Le
monitor Docker ouvre egalement une connexion a chaque publication, et l'API
fait de meme a chaque lecture du snapshot.

### Proposition

- Conserver une connexion multiplexee partagee dans le Worker et dans l'API.
- Pipeliner les `EXISTS` des services connus.
- Regrouper les reservations de deduplication quand plusieurs alertes sont
  produites dans le meme cycle.
- Conserver le comportement fail-open actuel pour une panne Redis, mais rendre
  la degradation observable par une metrique.

### Fichiers concernes

- `ops-worker/src/alerts_dispatcher.rs`
- `ops-worker/src/container_monitor.rs`
- `ops-api/src/container_monitor.rs`

## Priorite 3 - Stabiliser les boucles periodiques

Le dispatcher d'alertes et le monitor Docker commencent tous les deux par un
`tokio::time::sleep`. Leur premiere execution est donc retardee d'un intervalle
complet. La duree de chaque traitement s'ajoute ensuite a la periode et fait
deriver la cadence.

### Proposition

- Utiliser `tokio::time::interval`.
- Choisir `MissedTickBehavior::Skip` pour ne pas accumuler des cycles si un
  appel Docker, SQL ou webhook prend trop de temps.
- Executer immediatement un premier relevé Docker afin de construire la
  reference, sans produire d'evenements `Added` au demarrage.

## Priorite 4 - Optimiser le dispatcher de webhooks

Les regles, les alertes, les reservations Redis et les appels Discord sont
traites sequentiellement. Un webhook lent peut retarder toutes les alertes du
cycle.

### Proposition

- Construire d'abord la liste des alertes candidates.
- Reserver leurs cles de deduplication.
- Envoyer les webhooks avec une concurrence bornee, par exemple trois envois.
- Respecter les reponses `429` et leur delai de retry.
- Ajouter un compteur pour les alertes generees, dedupliquees, envoyees et en
  erreur.

Une concurrence illimitee n'est pas souhaitable a cause du rate limit Discord.

## Priorite 5 - Optimiser le monitor Docker

Chaque relevé realise actuellement plusieurs operations evitables :

- clone complet de la map `current` vers `previous` ;
- une insertion SQL attendue pour chaque changement ;
- ecriture du `RwLock`, puis relecture immediate pour serialiser son contenu ;
- nouvelle connexion Redis pour publier le snapshot.

### Proposition

1. Remplacer les maps par mouvement avec `std::mem::replace` ou restructurer
   la comparaison pour eviter le clone complet.
2. Construire le nouvel etat localement, le serialiser, puis le deplacer dans
   le `RwLock`.
3. Ajouter une methode `record_batch` au port `ServerEventRepository` et
   inserer les changements avec `QueryBuilder` ou `UNNEST`.
4. Reutiliser la connexion Redis multiplexee.

Le traitement par lot doit conserver une limite raisonnable afin de ne pas
produire une requete SQL demesuree apres une recreation massive de conteneurs.

## Priorite 6 - Corriger et accelerer les endpoints Docker

### Overview

`version_info` et `disk_usage` sont deux appels independants au Docker Agent,
mais ils sont attendus l'un apres l'autre. Ils peuvent etre executes avec
`tokio::try_join!` pour ramener la latence de l'overview vers celle de l'appel
le plus lent.

### Prune systeme

Le calcul `total_space_reclaimed_bytes` additionne les conteneurs, images et
volumes, mais oublie `networks.space_reclaimed_bytes`.

Les etapes de prune sont destructives et partiellement appliquees si une etape
ulterieure echoue. Elles doivent rester sequentielles, car elles ont des
dependances fonctionnelles, mais la reponse devrait exposer le resultat de
chaque etape, y compris en cas d'echec partiel.

### Fichier concerne

- `ops-api/src/handlers/docker.rs`

## Priorite 7 - Fiabiliser l'audit Docker

`audit_docker` enregistre actuellement l'action avant l'appel au Docker Agent.
Une operation refusee ou en erreur apparait donc comme une operation executee,
sans champ indiquant son resultat. L'insertion est detachee avec
`tokio::spawn` et peut aussi etre perdue lors d'un arret du processus.

### Proposition

- Enregistrer une tentative avec un identifiant d'operation.
- Completer l'evenement avec `success`, le statut et une erreur publique apres
  l'appel Docker ; ou enregistrer distinctement `requested` et `completed`.
- Ne pas laisser une tache detachee non suivie porter la seule trace durable.
- Mutualiser le wrapper d'execution/audit des actions Docker afin d'eviter la
  repetition dans chaque handler.

## Priorite 8 - Rendre le cleanup explicite et coherent

La purge des tables secondaires fonctionne en best-effort : une erreur SQL ou
HTTP est transformee en `0` ligne supprimee. L'operateur ne peut pas distinguer
une table vide d'une purge echouee. Les suppressions locales sont aussi
effectuees sans transaction, donc une erreur intermediaire laisse un resultat
partiel.

### Proposition

- Valider toutes les options avant la premiere suppression.
- Executer les suppressions PostgreSQL locales dans une transaction lorsque
  les permissions et les vues le permettent.
- Traiter la purge distante d'`auth-api` separement, car elle ne peut pas
  appartenir a la transaction PostgreSQL locale.
- Retourner un statut par cible : `deleted`, `skipped` ou `failed`.
- Ne journaliser comme reussie que la partie reellement appliquee.

### Fichiers concernes

- `ops-api/src/adapters/security_audit_repository.rs`
- `ops-api/src/adapters/auth_logins.rs`
- `ops-api/src/handlers/security.rs`

## Priorite 9 - Separer liveness et readiness

`/health` retourne toujours `{ "status": "ok" }`. Il ne permet pas de savoir
si PostgreSQL, Redis ou le Docker Agent sont utilisables.

### Proposition

- Garder une route de liveness locale qui ne provoque pas de redemarrages lors
  d'une panne d'infrastructure.
- Ajouter une readiness qui controle PostgreSQL et Redis en parallele.
- Exposer l'etat du Docker Agent comme dependance optionnelle ou degradee selon
  le profil de deploiement.
- Faire pointer le healthcheck Compose vers la route correspondant au
  comportement souhaite.

## Priorite 10 - Decoupler `ops-worker` de `ops-api`

`ops-worker` depend de la crate complete `ops-api` uniquement pour reutiliser
`HttpDockerHost` et `PgServerEventRepository`. Cela couple le Worker au
transport Axum et augmente sa surface de compilation.

### Proposition

Creer une petite crate `ops-adapters` contenant les adaptateurs partages :

- client du Docker Agent ;
- repository PostgreSQL des evenements serveur.

`ops-api` et `ops-worker` dependraient alors de `ops-core` et
`ops-adapters`, sans dependance Worker vers API.

## Priorite 11 - Decouper les god files

Deux handlers concentrent trop de responsabilites :

- `ops-api/src/handlers/security.rs` : 888 lignes ;
- `ops-api/src/handlers/docker.rs` : 610 lignes.

### Decoupage propose pour `security`

- `security/logs.rs` : top IP, auth failures et trafic ;
- `security/bans.rs` : bans manuels, ban et unban ;
- `security/probes.rs` : SSH, ports, disque, connexions, Trivy et integrite ;
- `security/audit.rs` : audit, logins et cleanup ;
- `security/tls.rs` : certificat et erreurs TLS ;
- `security/geoip.rs` : resolution GeoIP.

### Decoupage propose pour `docker`

- `docker/overview.rs` ;
- `docker/containers.rs` ;
- `docker/images.rs` ;
- `docker/volumes.rs` ;
- `docker/networks.rs` ;
- `docker/prune.rs` ;
- `docker/audit.rs`.

Les DTO specifiques peuvent rester pres de leur handler. Les DTO reutilises
par plusieurs modules peuvent etre places dans `docker/dto.rs`.

## Priorite 12 - Verifier les index des logs

Les lectures Ops filtrent principalement sur :

- `category = 'api'` et `timestamp` ;
- `details->>'status_code'` ;
- `details->>'client_ip'` ;
- `server_events.action` avec recherche par prefixe ;
- `server_events.severity` avec tri temporel.

Les index actuels couvrent surtout les colonnes individuellement. Avant toute
migration, mesurer les requetes reelles avec `EXPLAIN (ANALYZE, BUFFERS)`.
Selon les resultats, evaluer :

- un index `(category, timestamp DESC)` sur `logs` ;
- un index partiel des erreurs HTTP API ;
- un index d'expression sur le statut JSONB ;
- des index composites `(severity, timestamp DESC)` et
  `(action text_pattern_ops, timestamp DESC)` sur `server_events`.

Les migrations doivent tenir compte du partitionnement de `logs` et creer ou
attacher les index sur les partitions concernees.

## Documentation a remettre en coherence

Plusieurs commentaires decrivent encore une architecture precedente :

- `container_monitor.rs` explique que le job vit dans l'API alors qu'il est
  desormais dans `ops-worker` et publie son etat dans Redis ;
- `handlers/docker.rs` parle encore d'un socket Docker monte dans l'API ;
- `ops-api/src/lib.rs` et `compose.core.yml` parlent d'un role PostgreSQL Ops
  restreint, alors que la migration 028 abandonne explicitement ce role et que
  les deux services utilisent `sentinel_app`.

Cette documentation doit etre corrigee en meme temps que les modules touches.

## Couverture de tests a ajouter

- Requete de comptage des 401/403 stockes dans JSONB.
- Absence de reemission d'un ancien changement apres expiration du cooldown.
- Degradation Redis sans faux service offline.
- Calcul complet de `prune/system`, reseaux inclus.
- Audit d'une operation Docker reussie et echouee.
- Cleanup partiellement echoue.
- Routes de liveness et readiness.
- Tests de route Axum pour les principaux handlers Ops.

## Ordre d'implementation recommande

1. Corriger le comptage des echecs d'authentification.
2. Corriger la consommation des changements Docker.
3. Sortir `sysinfo` du runtime asynchrone.
4. Reutiliser et pipeliner les connexions Redis.
5. Stabiliser les boucles et borner la concurrence des webhooks.
6. Optimiser le monitor Docker.
7. Corriger l'overview, le total du prune et l'audit Docker.
8. Fiabiliser le cleanup et ajouter la readiness.
9. Extraire `ops-adapters`.
10. Decouper les handlers `security` et `docker`.
11. Mesurer les requetes puis ajouter uniquement les index justifies.

## Validation attendue apres chaque etape

```powershell
cargo fmt -p ops-core -p ops-api -p ops-worker -- --check
cargo test -p ops-core -p ops-api -p ops-worker
cargo clippy -p ops-core -p ops-api -p ops-worker --all-targets -- -D warnings
```

Pour les changements SQL, ajouter une nouvelle migration sans modifier les
migrations deja appliquees.
