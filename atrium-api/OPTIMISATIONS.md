# Audit d'optimisation Atrium

Date de l'audit : 11 aout 2026

## Etat initial

- Les 34 tests des crates `atrium-core`, `atrium-api` et `atrium-worker` passent.
- Clippy passe sur toutes les cibles sans avertissement.
- Aucun changement fonctionnel n'a ete applique pendant cet audit.

## Priorite 1 - Partager le pool PostgreSQL

Atrium construit plusieurs pools SQLx independants vers la meme base :

- service RAG ;
- garde de budget ;
- controle d'activation ;
- memoire conversationnelle ;
- configuration HTTP ;
- configuration gRPC.

Chaque pool gere ses propres connexions et ses propres limites. Cela augmente
inutilement le nombre potentiel de connexions PostgreSQL et complique le
reglage de leur cycle de vie.

### Proposition

1. Construire un seul `PgPool` au demarrage de `atrium-api`.
2. Executer les migrations avec ce pool.
3. Injecter des clones du pool dans `RagService`, `BudgetGuard`,
   `BotControlStore`, `ConversationMemory`, la surface HTTP et la surface
   gRPC.
4. Configurer explicitement `max_connections`, le timeout d'acquisition et le
   timeout de connexion.

### Fichiers concernes

- `atrium-api/src/main.rs`
- `atrium-api/src/lib.rs`
- `atrium-api/src/grpc.rs`
- `atrium-api/src/rag.rs`
- `atrium-api/src/budget.rs`
- `atrium-api/src/control.rs`
- `atrium-api/src/memory.rs`

## Priorite 2 - Charger la configuration une seule fois par requete

Une reponse d'accueil peut lire plusieurs fois les memes lignes de
`bot_guild_config` :

1. verification de la cle `enabled` ;
2. lecture des limites du budget ;
3. lecture de `welcome_context`.

Les chemins HTTP, gRPC normal et gRPC streaming reproduisent cette sequence.

### Proposition

- Charger une photographie de la configuration au debut de la requete.
- En deduire l'activation, les quotas et le contexte administrateur.
- Transmettre les limites deja resolues a `BudgetGuard`.
- Conserver une lecture par requete afin d'eviter un cache perime.

## Priorite 3 - Sortir la retention du chemin critique du budget

`BudgetGuard::check_and_record` execute actuellement deux suppressions de
retention a chaque appel IA :

- anciennes lignes de `atrium_ai_usage_users` ;
- anciennes lignes de `atrium_ai_usage_global`.

Ces suppressions sont faites dans la transaction qui verrouille les compteurs.
Elles augmentent sa duree et sont repetees alors que la retention ne doit etre
effectuee qu'une fois par jour.

### Proposition

- Ajouter un endpoint interne de retention.
- Le declencher quotidiennement depuis `atrium-worker`.
- Garder `check_and_record` limite au controle et a l'incrementation atomique
  des quotas.

### Point de coherence a corriger

`global_daily_limit` est presente comme un plafond par serveur, mais la table
`atrium_ai_usage_global` possede une seule ligne par date, toutes guilds
confondues. Cette ligne verrouille et serialise egalement les appels de toutes
les guilds.

Deux options sont possibles :

- ajouter `guild_id` a la cle du compteur pour obtenir un plafond par serveur ;
- conserver un plafond plateforme, mais le renommer et ne plus le configurer
  par guild.

La premiere option correspond au schema et au texte actuellement affiches
dans l'administration.

## Priorite 4 - Optimiser et securiser l'indexation RAG

L'indexation des connaissances embarquees traite actuellement 19 documents de
maniere sequentielle. Pour chaque document modifie, elle effectue :

1. une lecture de son etat ;
2. un appel au service d'embeddings ;
3. un upsert du document ;
4. une suppression des anciens fragments ;
5. une insertion SQL par fragment.

Le remplacement n'est pas transactionnel. Une interruption apres la
suppression peut donc laisser un document sans fragments. De plus,
l'indexation bloque le demarrage complet de l'API si Ollama est indisponible.

### Proposition

- Charger les hashes des documents en une seule requete.
- Ignorer immediatement les documents inchanges.
- Remplacer les fragments dans une transaction par document.
- Inserer les fragments par lot avec `QueryBuilder` ou `UNNEST`.
- Deplacer l'indexation dans `atrium-worker`, ou la lancer en arriere-plan avec
  un etat de sante degrade plutot que de bloquer l'API.

## Priorite 5 - Mutualiser l'orchestration HTTP et gRPC

Les trois chemins suivants repetent presque toute la logique d'accueil :

- endpoint HTTP ;
- RPC `GenerateReply` ;
- RPC `StreamReply`.

La sequence dupliquee comprend le controle d'activation, le budget,
l'historique, le RAG, le resume recent, le contexte administrateur, l'appel IA
et la sauvegarde de la memoire.

### Proposition

Creer un orchestrateur partage qui retourne une reponse metier independante du
transport. HTTP et gRPC ne conserveraient que :

- la conversion de leur requete vers le modele commun ;
- la conversion des erreurs ;
- la serialisation de la reponse.

Une fois cette mutualisation faite, les lectures independantes de l'historique,
du resume et du contexte RAG pourront etre executees en parallele avec une
concurrence bornee.

## Priorite 6 - Ajouter des timeouts aux appels externes

Les clients DeepSeek et embeddings sont construits avec
`reqwest::Client::new()` sans timeout explicite. Une connexion ou une reponse
bloquee peut donc immobiliser une requete, et l'indexation peut immobiliser le
demarrage de l'API.

### Proposition

- timeout de connexion court ;
- timeout total adapte a DeepSeek et aux embeddings ;
- taille maximale du corps de reponse ;
- logs distinguant connexion, timeout, statut HTTP et payload invalide.

## Priorite 7 - Rendre les mises a jour de configuration atomiques

L'endpoint d'administration valide puis ecrit chaque cle avec une requete
separee. Une erreur SQL intermediaire peut laisser une partie des valeurs
appliquee et l'autre non.

### Proposition

1. Valider l'ensemble du payload avant toute ecriture.
2. Ouvrir une transaction.
3. Faire un upsert groupe des valeurs.
4. Committer seulement si toutes les cles ont ete ecrites.

Le volume maximal est faible, mais le gain principal est la coherence de la
configuration.

## Optimisations secondaires

### Memoire conversationnelle

- Regrouper les deux insertions d'un echange dans une seule requete.
- Evaluer une retention moins frequente que la suppression apres chaque
  message.
- Conserver l'index `(guild_id, member_id, id DESC)`, adapte aux lectures
  actuelles.

### Streaming gRPC

Le RPC de streaming attend actuellement la reponse DeepSeek complete, puis la
decoupe en mots. Il n'ameliore donc pas le temps jusqu'au premier token.

- Soit brancher le vrai streaming du fournisseur IA.
- Soit conserver une reponse unaire et supprimer le faux streaming.

### Worker multi-guild

`atrium-worker` genere le resume d'une seule guild configuree par variable
d'environnement. Si Atrium devient reellement multi-guild, le Worker devra
lister les guilds actives et les traiter avec une concurrence bornee.

## Ordre d'implementation recommande

1. Pool PostgreSQL partage.
2. Configuration chargee une fois et orchestration partagee.
3. Retention du budget dans le Worker et compteur par guild.
4. Timeouts HTTP externes.
5. Indexation RAG transactionnelle et groupee.
6. Upsert atomique de la configuration.
7. Optimisations secondaires de memoire et de streaming.

## Validation attendue apres chaque etape

```powershell
cargo fmt -p atrium-core -p atrium-api -p atrium-worker -- --check
cargo test -p atrium-core -p atrium-api -p atrium-worker
cargo clippy -p atrium-core -p atrium-api -p atrium-worker --all-targets -- -D warnings
```

Pour les changements SQL, ajouter une nouvelle migration sans modifier les
migrations deja appliquees.
