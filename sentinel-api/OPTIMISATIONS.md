# Audit d'optimisation Sentinel

Date de l'audit : 11 aout 2026

## Perimetre

Cet audit couvre :

- `sentinel-core` ;
- `sentinel-api` ;
- `sentinel-worker` ;
- les migrations PostgreSQL et les usages Redis associes.

Aucun changement fonctionnel n'a ete applique pendant l'audit.

## Etat des validations

- `sentinel-api` : 669 tests passent et 3 sont ignores.
- `sentinel-worker` : 8 tests passent.
- `sentinel-core` : 1378 tests passent et 8 echouent.
- Clippy passe sur les cibles de production (`lib` et `bin`).
- Clippy avec `--all-targets -D warnings` echoue sur deux symboles morts dans
  les tests gRPC de moderation.

La qualite gate globale n'est donc pas verte au moment de l'audit.

## Priorite 0 - Terminer la migration de la moderation vers `audit_logs`

La source de verite des actions de moderation est en cours de migration de
`moderation_actions` vers `audit_logs` avec des `event_type = 'mod_*'`.
L'ecriture de production passe deja uniquement par `audit_logs` :

- `ManageModerationService::log_action` appelle `audit_logs_uc.create` ;
- `PgModerationRepository::save` est volontairement devenu un no-op.

Cependant, plusieurs chemins lisent encore `moderation_actions`, une table qui
n'est plus alimentee :

- comptage recent des actions d'un moderateur ;
- classement des moderateurs ;
- export `moderation_actions` ;
- file de revue manuelle avec jointure vers `moderation_actions` ;
- remise a zero des donnees d'un membre ;
- contraintes de preuve et de revue liees aux anciens identifiants.

### Impacts

- Le quota ou compteur d'actions recentes peut retourner zero a tort.
- Le classement des moderateurs peut etre vide ou incomplet.
- Un export de moderation peut omettre toutes les nouvelles actions.
- L'ajout en file de revue peut echouer sur une contrainte de cle etrangere.
- Une remise a zero peut annoncer la suppression d'actions sans retirer les
  lignes `mod_*` d'`audit_logs`.

### Proposition

1. Inventorier toutes les lectures, jointures, exports et suppressions de
   `moderation_actions`.
2. Les migrer vers `audit_logs` avec un mapping centralise `event_type/details`.
3. Migrer les relations `review_queue` et `moderation_evidence` vers
   l'identifiant reel choisi pour l'action.
4. Adapter la remise a zero membre sans supprimer les traces d'audit que la
   politique de conservation impose de garder.
5. Supprimer le port `ModerationRepository::save` devenu trompeur, ou lui
   rendre une semantique coherente avec les tests.
6. Supprimer `moderation_actions` seulement apres verification de tous les
   consommateurs et migration des donnees historiques.

### Fichiers concernes

- `sentinel-core/src/application/moderation/manage_moderation_service.rs`
- `sentinel-api/src/adapters/outbound/postgres/moderation/moderation_repository.rs`
- `sentinel-api/src/adapters/outbound/postgres/moderation/review_repository.rs`
- `sentinel-api/src/adapters/outbound/postgres/audit/modstats_repository.rs`
- `sentinel-api/src/adapters/outbound/postgres/system/export_repository.rs`
- `sentinel-core/src/domain/entities/community/guild_member_reset.rs`
- `sentinel-api/migrations/001_init.sql`

### Tests actuellement en echec

Les huit tests de `ManageModerationService` qui utilisent un repository
in-memory attendent encore que `log_action` appelle `repo.save`. Ils confirment
que le contrat du port et son implementation de production ont diverge.

Il faut corriger l'architecture et les tests ensemble, pas simplement modifier
les assertions pour rendre la CI verte.

## Priorite 0 - Corriger la retention de `logs`

Le job de cleanup execute :

```sql
DELETE FROM logs WHERE created_at < NOW() - make_interval(days => $1)
```

La table `logs` possede une colonne `timestamp`, pas `created_at`. La requete
echoue a chaque passage et la retention des logs n'est jamais appliquee.

### Proposition

- Utiliser `timestamp` comme le fait deja `PgLogRepository`.
- Ajouter un test d'integration SQL avec une ligne ancienne et une ligne
  recente.
- Ajouter une metrique `cleanup_rows_total{table=...}` et une metrique d'erreur
  par table.
- Conserver le rapport d'erreurs partielles actuel, mais rendre cette erreur
  visible dans la supervision du Worker.

### Fichier concerne

- `sentinel-worker/src/domains/cleanup/cleanup_old_data.rs`

## Priorite 0 - Reparer les retries des exports

Le Worker ne claim que les jobs avec `status = 'pending'`. Lors d'une erreur,
il place pourtant le job en `failed` tant que `max_retries` n'est pas atteint.
Aucun chemin ne remet ensuite un job `failed` en `pending`.

Le premier echec transitoire bloque donc definitivement l'export et
`max_retries` ne peut jamais jouer son role.

### Proposition

- Remettre le job en `pending` avec un `next_attempt_at` et un backoff ; ou
- inclure explicitement les jobs `failed` devenus eligibles dans le claim.
- Reserver `dead` aux echecs definitifs.
- Ajouter des tests couvrant succes, echec transitoire, backoff et epuisement
  de `max_retries`.

### Fichier concerne

- `sentinel-worker/src/domains/export/drain_export_jobs.rs`

## Priorite 0 - Fiabiliser la synchronisation Discord Audit

La synchronisation Discord Audit annonce une deduplication par
`details.discord_entry_id`, mais aucune contrainte unique ni requete
`ON CONFLICT` ne l'applique.

Les entrees sont inserees une par une. Si une ancienne insertion echoue puis
qu'une entree plus recente reussit, le curseur final peut avancer au-dela de
l'entree echouee. Celle-ci ne sera alors jamais rechargee. A l'inverse, une
remise a zero du curseur peut creer des doublons.

### Proposition

1. Ajouter une colonne dediee `discord_entry_id` ou un index d'expression
   unique adapte aux lignes `discord_audit:*`.
2. Inserer le lot et mettre a jour le curseur dans une transaction.
3. Ne faire avancer le curseur que si toutes les entrees pertinentes jusqu'a
   ce curseur sont durablement inserees ou deja presentes.
4. Inserer les lignes par lot avec `QueryBuilder` ou `UNNEST`.
5. Deriver `created_at` du snowflake Discord plutot que d'utiliser `NOW()` afin
   de conserver l'heure reelle de l'action.

### Fichier concerne

- `sentinel-worker/src/domains/discord_audit_sync/sync_discord_audit_logs.rs`

## Priorite 1 - Rendre le scheduler reellement supervisable

`sentinel-worker` orchestre plus de trente jobs dans un seul processus. Cette
fusion economise des runtimes, pools et conteneurs, mais augmente l'importance
du scheduler central.

Le scheduler actuel presente plusieurs limites :

- premiere execution retardee d'un intervalle complet ;
- cadence basee sur `sleep`, donc derive avec la duree du traitement ;
- aucun `catch_unwind` malgre le commentaire annonçant la capture des panics ;
- les handles de taches ne sont pas conserves ;
- le `main` envoie le shutdown, ferme immediatement le pool et termine sans
  attendre les jobs ;
- les boucles specialisees, comme les annonces et le monitoring, ne partagent
  pas toutes le meme mecanisme d'arret.

### Proposition

- Faire retourner les `JoinHandle` ou utiliser un `JoinSet` supervise.
- Utiliser `tokio::time::interval` avec `MissedTickBehavior::Skip`.
- Ecouter simultanement le tick et un `CancellationToken`/watch avec
  `tokio::select!`.
- Capturer et compter les panics, puis relancer le job selon une politique
  explicite.
- Lors du shutdown, arreter les nouveaux claims, attendre les jobs en cours
  avec un timeout, puis fermer PostgreSQL et Redis.
- Exposer pour chaque job : dernier debut, dernier succes, duree, erreurs
  consecutives et statut vivant.

### Fichiers concernes

- `platform-common-worker/src/lib.rs`
- `sentinel-worker/src/main.rs`
- `sentinel-worker/src/scheduler.rs`

## Priorite 2 - Creer un contexte partage pour le Worker

Plusieurs jobs reconstruisent leur propre client HTTP a chaque tick et ouvrent
une nouvelle connexion Redis a chaque execution ou resultat. Le scheduler ne
transmet aujourd'hui que le pool PostgreSQL, ce qui pousse chaque domaine a
recomposer ses dependances.

### Proposition

Introduire un `WorkerContext` cloneable contenant :

- le `PgPool` ;
- une connexion Redis multiplexee partagee ;
- un client HTTP commun avec timeouts de connexion et de reponse ;
- la configuration resolue ;
- le signal d'arret et les metriques de jobs.

Les clients necessitant un timeout specifique peuvent deriver d'un petit
ensemble de clients partages plutot que d'etre reconstruits a chaque cycle.

## Priorite 3 - Eliminer les N+1 Redis du monitoring

Le monitoring execute `SMEMBERS bots:known`, puis un `EXISTS` attendu
sequentiellement pour chaque service. Il ouvre une nouvelle connexion Redis a
chaque cycle et utilise un `reqwest::Client::new()` sans timeout explicite.

Le endpoint `/api/system/info` reproduit le meme N+1, ouvre ensuite une seconde
connexion Redis pour `PING`, puis effectue la collecte `sysinfo` dans le chemin
de la requete.

### Proposition

- Pipeliner les `EXISTS` ou modeliser les heartbeats dans un sorted set.
- Reutiliser la connexion multiplexee.
- Reutiliser un client HTTP avec timeout.
- Borner la concurrence des notifications de changement d'etat.
- Deplacer la collecte `sysinfo` dans `spawn_blocking` et mettre en cache un
  snapshot tres court si l'ecran la demande frequemment.

### Fichiers concernes

- `sentinel-worker/src/domains/monitoring/check_services.rs`
- `sentinel-api/src/adapters/inbound/http/handlers/system/info.rs`

## Priorite 4 - Fiabiliser les files Redis fire-and-forget

`JobClient::enqueue` et `EventBroadcaster::broadcast` lancent une tache detachee
pour chaque operation. Chaque tache ouvre sa propre connexion Redis. Le
handler peut annoncer un succes avant que le job soit reellement enfile, les
evenements peuvent etre reordonnes, et les taches peuvent etre perdues a
l'arret.

### Proposition

- Faire retourner un `Result` a l'enqueue lorsqu'il conditionne la reponse
  HTTP.
- Utiliser une connexion multiplexee persistante.
- Pour les evenements best-effort, utiliser un canal borne et une seule tache
  writer supervisee.
- Definir une politique claire lorsque le canal est plein : attente courte,
  rejet explicite ou compteur de perte.
- Envisager un outbox PostgreSQL pour les evenements qui ne doivent jamais
  etre perdus.

### Fichiers concernes

- `sentinel-api/src/adapters/outbound/job_client.rs`
- `sentinel-api/src/adapters/outbound/ws/broadcaster.rs`

## Priorite 5 - Rendre la publication des annonces atomique

`fetch_due_and_prepare` cree un run et avance `next_run_at` avant que le Worker
publie l'evenement dans Redis. Si `XADD` echoue, l'annonce a deja ete consideree
comme traitee et ne sera pas reprise au tick suivant. Le run reste `pending`.

Les insertions de run et mises a jour d'annonces sont aussi effectuees une par
une sans transaction globale ni claim SQL protege contre plusieurs instances.

### Proposition

- Utiliser un outbox transactionnel : claim de l'annonce, creation du run et
  creation de l'evenement outbox dans la meme transaction.
- Publier l'outbox dans Redis avec retry et idempotence.
- Mettre a jour le resultat du run apres l'accuse du bot.
- A minima, remettre l'annonce eligible et marquer le run en erreur lorsque
  `XADD` echoue.

### Fichiers concernes

- `sentinel-core/src/application/community/manage_announcements_service.rs`
- `sentinel-api/src/adapters/outbound/postgres/community/announcement_repository.rs`
- `sentinel-worker/src/domains/announcements/publish_due.rs`

## Priorite 6 - Optimiser les caches analytics multi-guild

Pour chaque guild, `warm_analytics` execute :

- une lecture du flag `enabled` ;
- cinq agregations PostgreSQL sequentielles ;
- une ecriture Redis.

`warm_dashboard` ajoute quatre autres agregations sequentielles par guild. Le
cout grandit donc lineairement avec le nombre de guilds et la latence de chaque
requete s'additionne.

### Proposition

- Charger les flags d'activation de toutes les guilds en une requete.
- Executer les agregations independantes avec `try_join!` si le pool le permet.
- Evaluer des requetes groupees pour toutes les guilds, puis partitionner les
  resultats en memoire.
- Borner la concurrence entre guilds afin de ne pas saturer PgBouncer.
- Pipeliner les ecritures Redis.
- Mesurer les requetes avec `EXPLAIN (ANALYZE, BUFFERS)` avant d'ajouter des
  index ou des vues materialisees.

### Fichiers concernes

- `sentinel-worker/src/domains/cache/warm_analytics.rs`
- `sentinel-worker/src/domains/cache/warm_dashboard.rs`
- `sentinel-worker/src/domains/cache/warm_voice_stats.rs`

## Priorite 7 - Mutualiser les checks d'activation des modules

Le Worker appelle `is_worker_globally_enabled` avant chaque tick et plusieurs
jobs appellent ensuite `is_worker_enabled` pour chaque ligne ou chaque guild.
Ces helpers executent une requete PostgreSQL et masquent les erreurs SQL en une
valeur par defaut.

### Proposition

- Charger une photographie de tous les flags necessaires au debut du tick.
- La partager entre les jobs ou la mettre en cache avec un TTL court.
- Conserver un mode fail-closed pour les actions destructives.
- Distinguer dans les logs/metriques `disabled` et `configuration
  indisponible`.

La configuration des intervalles est actuellement chargee seulement au
demarrage. Si elle est presentee comme dynamique dans l'administration, il
faut soit la recharger, soit afficher clairement qu'un redemarrage est requis.

## Priorite 8 - Decouper et simplifier l'analyse Automod

`AnalyzeMessageService` depasse 1100 lignes et concentre :

- chargement/cache des regles ;
- parsing de nombreuses configurations ;
- analyse DeepSeek ;
- inference ONNX ;
- fusion des scores ;
- tension de salon ;
- routage des sanctions ;
- persistance de l'infraction ;
- evaluation flood, pieces jointes et majuscules.

DeepSeek et ONNX sont actuellement executes l'un apres l'autre lorsqu'ils sont
tous deux actifs. Leur latence s'additionne et leur fusion est enfouie dans une
fonction fortement mutable.

### Proposition

- Extraire un snapshot type de configuration Automod.
- Extraire les analyseurs DeepSeek et ONNX en composants independants.
- Produire des signaux immuables, puis les fusionner dans un agregateur teste.
- Executer les analyseurs independants en parallele sous une limite globale.
- Separer tension de salon, routage et persistance.
- Reutiliser le meme snapshot de configuration pour `analyze`, flood,
  attachments et caps lorsqu'ils appartiennent au meme traitement bot.

### Fichier principal concerne

- `sentinel-core/src/application/ai/analyze_message_service.rs`

## Priorite 9 - Separer liveness et readiness

Le endpoint `/health` verifie PostgreSQL puis Redis sequentiellement. Le check
Redis ecrit une cle `health:ping` au lieu d'utiliser `PING`. Comme cette route
est utilisee comme healthcheck Docker, une panne de dependance peut rendre le
conteneur API unhealthy alors que son runtime fonctionne encore.

### Proposition

- `/health/live` : verifier uniquement que le serveur repond.
- `/health/ready` : tester PostgreSQL, Redis et les dependances requises en
  parallele avec un timeout court.
- Utiliser `PING` pour Redis.
- Faire pointer Compose vers la route correspondant a la politique de restart
  desiree.
- Rendre les statuts gRPC dependants d'un etat reel plutot que les marquer
  definitivement `SERVING` au demarrage.

## Priorite 10 - Achever l'extraction Ops

Sentinel conserve encore un `OpsState`, des metriques host, `sysinfo`, le
registre des services et des DTOs issus d'`ops-core`. Une partie est utile au
dashboard Sentinel, mais les responsabilites machine ont deja leur API dediee.

### Proposition

- Deplacer les metriques host et l'inventaire complet des services vers
  `ops-api`.
- Laisser Sentinel exposer uniquement les indicateurs metier Discord dont il
  est proprietaire.
- Si le dashboard Sentinel a besoin d'un resume Ops, le consommer via un client
  HTTP type plutot que partager les ports d'infrastructure.
- Retirer ensuite `sysinfo` et les reliquats Docker/TLS de `sentinel-api`.

Cette etape simplifiera `OpsState`, `AppState`, les routes systeme et l'image
Docker de Sentinel.

## Priorite 11 - Terminer le decoupage de `AppState`

Les sous-etats par domaine existent deja, mais `AppState` conserve encore de
nombreux champs historiques en doublon. Environ 80 occurrences de
`State<AppState>` subsistent dans les handlers HTTP, reparties dans une
quinzaine de fichiers.

`bootstrap/app_state.rs` fait plus de 800 lignes et construit tous les
repositories et use cases dans une seule fonction.

### Proposition

- Migrer les derniers handlers vers `AiState`, `ModerationState`,
  `CommunityState`, etc.
- Extraire une factory de composition par domaine.
- Conserver dans la racine seulement les dependances veritablement
  transverses.
- Retirer `pg_pool` de l'etat HTTP une fois les tests d'integration equipes de
  leurs propres fixtures.
- Ajouter un test d'architecture interdisant le SQL direct dans les handlers.

## Priorite 12 - Decouper les god files restants

Fichiers de production a traiter en premier :

- `sentinel-core/src/application/ai/analyze_message_service.rs` : 1125 lignes ;
- `sentinel-api/src/adapters/outbound/discord_api.rs` : 993 lignes ;
- `sentinel-api/src/adapters/outbound/postgres/moderation/automod_review_repository.rs` : 936 lignes ;
- `sentinel-api/src/bootstrap/app_state.rs` : 812 lignes ;
- `sentinel-api/src/adapters/inbound/http/handlers/moderation/automod/reviews.rs` : 797 lignes ;
- `sentinel-api/src/adapters/inbound/http/handlers/community/voice_channels.rs` : 783 lignes ;
- `sentinel-api/src/adapters/inbound/http/handlers/moderation/actions.rs` : 764 lignes ;
- `sentinel-worker/src/config.rs` : 699 lignes ;
- `sentinel-worker/src/scheduler.rs` : 628 lignes.

### Decoupages recommandes

- `discord_api` par ressource Discord : guilds, members, roles, channels,
  messages et moderation.
- `automod_review_repository` par agregat SQL : reviews, votes, incidents,
  sanctions et retention.
- `voice_channels` par capacite : lifecycle, permissions, bans, invitations et
  themes.
- `moderation/actions` par actions, historique, preuves, revues et statistiques.
- `WorkerConfig` en sous-configurations par domaine.
- `scheduler` en fonctions `register_<domain>_jobs`.

Le decoupage doit rester fonctionnel : deplacer les tests avec leur module et
eviter un simple morcellement en fichiers qui continuent tous de dependre du
meme god object.

## Priorite 13 - Verifier les index de la nouvelle source de verite

La migration vers `audit_logs` ajoute des acces frequents sur :

- `guild_id + event_type + created_at` ;
- prefixe `event_type LIKE 'mod_%'` ;
- `details->>'action_id'` ;
- `target_id` et `actor_id` ;
- `details->>'discord_entry_id'` pour la synchronisation Discord.

Les index historiques couvrent plusieurs colonnes, mais pas necessairement les
expressions JSONB ni tous les plans de la nouvelle source de verite.

### Proposition

- Capturer les requetes lentes via `pg_stat_statements`.
- Utiliser `EXPLAIN (ANALYZE, BUFFERS)` sur chaque chemin migre.
- Evaluer des index partiels limites a `event_type LIKE 'mod_%'`.
- Ajouter un index unique pour les identifiants Discord importes.
- Tenir compte du partitionnement mensuel d'`audit_logs` et attacher les index
  aux partitions.

## Nettoyage secondaire

### Dependances et commentaires obsoletes

Le manifeste et plusieurs routes mentionnent encore l'administration Docker,
le socket Unix, TLS et des fonctionnalites deja deplacees. `tar`,
`x509-parser` et certaines dependances Unix doivent etre verifiees avec un
outil de dependances inutilisees puis retirees si elles ne sont plus compilees
par aucun chemin.

`sysinfo` est encore utilise par `/api/system/info` et ne pourra etre retire
qu'apres l'extraction Ops.

### Cache Redis au demarrage

`connect_redis` ouvre deux connexions successives et supprime
`bot:definitions` a chaque demarrage de l'API. Avec plusieurs replicas ou des
redemarrages rapproches, cela peut provoquer des invalidations et
rechargements inutiles. Preferer une version de schema dans la cle de cache ou
une invalidation conditionnee a une migration effectivement appliquee.

### Invalidation par pattern

`RedisCache::invalidate_pattern` utilise correctement `SCAN`, mais supprime
ensuite les cles une par une. Pipeliner les `DEL` de chaque page reduira les
allers-retours.

## Couverture de tests a ajouter

- Integration de tous les chemins migres de `moderation_actions` vers
  `audit_logs`.
- Retention de la table partitionnee `logs` avec la colonne `timestamp`.
- Cycle complet de retry d'un export.
- Sync Discord Audit avec erreur intermediaire, redemarrage et doublon.
- Shutdown du Worker pendant un job long.
- Detection et supervision d'un job qui panique.
- Publication d'annonce lorsque Redis est temporairement indisponible.
- Enqueue Redis refuse : le handler ne doit pas annoncer un faux succes.
- Liveness/readiness HTTP et statut de sante gRPC degrade.

## Ordre d'implementation recommande

1. Corriger la retention `logs` et le retry des exports.
2. Finaliser la migration `moderation_actions` vers `audit_logs` et remettre
   les tests/Clippy au vert.
3. Rendre la synchronisation Discord Audit transactionnelle et idempotente.
4. Superviser le scheduler et implementer un shutdown reellement gracieux.
5. Introduire `WorkerContext` et mutualiser HTTP/Redis/config.
6. Fiabiliser les annonces et les files Redis fire-and-forget.
7. Optimiser monitoring et caches analytics.
8. Decouper `AnalyzeMessageService` et definir la fusion DeepSeek/ONNX.
9. Separer liveness/readiness et achever l'extraction Ops.
10. Terminer `AppState`, les god files et les index mesures.

## Validation attendue apres chaque etape

```powershell
cargo fmt -p sentinel-core -p sentinel-api -p sentinel-worker -- --check
cargo test -p sentinel-core -p sentinel-api -p sentinel-worker
cargo clippy -p sentinel-core -p sentinel-api -p sentinel-worker --all-targets -- -D warnings
```

Pour les changements SQL, ajouter une nouvelle migration sans modifier les
migrations deja appliquees. Tester les migrations sur une copie representative
de la base, en particulier pour les tables partitionnees `logs` et
`audit_logs`.
