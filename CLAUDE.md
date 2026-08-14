# Regles d'or de DiscordSentinel

Ce fichier est la charte de travail du depot. Le code existant et les tests sont
la source de verite. La documentation produit se trouve dans `DOC/`.

## 1. Architecture de reference

Le depot est un monorepo Rust. L'architecture actuelle est :

| Composant | Responsabilite |
|---|---|
| `platform-core` | Regles metier et cas d'usage des domaines `sentinel`, `nexus`, `atrium`, `ops` |
| `platform-api` | Service API unifie : HTTP, gRPC, persistance et adaptateurs externes |
| `platform-proto` | Contrats protobuf/gRPC partages |
| `platform-scheduler` | Appels HTTP periodiques, sans logique metier ni acces base |
| `platform-gateway` | Relais Redis Streams vers WebSocket |
| `sentinel-bot`, `nexus-bot`, `atrium-bot` | Connexions Discord et interfaces utilisateur des trois produits |
| `auth-core`, `auth-api` | OAuth Discord, sessions web et identite |
| `ops-agent` | Sondes de l'hote et supervision privilegiee |
| `docker-agent` | Seul composant autorise a parler au socket Docker |
| `web` | Dashboard Vue/TypeScript multi-univers |

Les anciens crates `sentinel-api`, `nexus-api`, `atrium-api`, `ops-api`, les
anciens workers et les anciens protos ne doivent pas etre recrees.

## 2. Dependances et couches

Respecter le sens suivant :

`domain -> application -> ports -> adapters`

- `platform-core` ne depend pas de SQLx, Axum, Serenity, Reqwest, Redis ou Docker.
- Les ports sont definis pres du domaine qui en a besoin.
- Les implementations des ports vivent dans `platform-api`, `ops-agent` ou
  `docker-agent` selon leur responsabilite.
- Un handler HTTP traduit une requete et appelle un cas d'usage. Il ne contient
  pas de regle metier et ne fait pas d'I/O sortante directement.
- Un bot appelle l'API ou gRPC. Il ne lit jamais PostgreSQL et ne contient pas
  de decision metier durable.
- Le scheduler appelle une route interne authentifiee. Il ne contient ni SQLx,
  ni Redis metier, ni Docker, ni SDK Discord.

## 3. Frontieres des domaines

- Sentinel concerne la moderation, la securite et la vie d'un serveur Discord.
- Nexus concerne les jeux, serveurs de jeu, wallet, economie et animations.
- Atrium concerne l'accueil, l'assistance IA, le RAG, la memoire et les quotas.
- Ops concerne la machine hote, les services, les logs techniques, TLS, les
  alertes et Docker.
- Auth concerne l'identite et les sessions ; aucun domaine ne doit reimplementer
  une resolution d'identite locale.

Ne pas deplacer une regle dans un domaine voisin pour eviter de creer un
nouveau port. Si une fonctionnalite touche plusieurs domaines, definir un
contrat explicite et conserver un proprietaire metier unique.

## 4. Regles de securite non negociables

1. Le navigateur ne recoit jamais de secret backend, token API, token Discord,
   token Docker ou credential de base.
2. L'acteur authentifie vient d'une frontiere serveur de confiance ; ne jamais
   faire confiance a un identifiant fourni par le navigateur.
3. `Denied` (403) et `Unavailable` (503) restent distincts.
4. Une permission Discord sensible est reverifiee dans le handler concerne.
5. Toute configuration inconnue ou permission incertaine echoue en fermeture.
6. Une cle de module absente signifie module desactive (fail closed).
7. Le ban Discord n'est jamais automatique : une finalisation humaine est
   obligatoire.
8. Les routes `/internal/` sont reservees au scheduler ou aux services
   autorises et exigent leur authentification dediee.
9. Le socket Docker est monte uniquement dans `docker-agent`. Ne jamais ajouter
   `bollard` ou un acces socket a `platform-api`, un bot ou le web.
10. Les tokens, preuves de moderation, transcripts, wallets, sessions et donnees
    IA ne doivent jamais apparaitre dans les logs.
11. TLS invalide, secret absent ou token trop court provoque un echec ferme.
12. Ne jamais utiliser un token de surface Docker hote pour les serveurs de jeu.

## 5. Donnees, bus et jobs

- Un bot ne parle jamais directement a PostgreSQL.
- Chaque domaine utilise sa base logique, ses migrations et son role applicatif.
- Toute migration doit etre ajoutee dans l'arborescence actuelle de
  `platform-api/migrations/`, apres lecture des migrations precedentes.
- Ne jamais modifier une migration deja appliquee pour corriger le schema.
- Les streams Redis sont cloisonnes par domaine (`sentinel:events`,
  `nexus:events`, etc.). Ne pas publier un evenement sur le stream d'un autre
  domaine.
- Un job doit etre idempotent et proteger les effets non repetables par un
  verrou ou une contrainte d'unicite.
- Une reponse de succes n'est annoncee qu'apres confirmation de l'API et de la
  persistance.
- Les appels scheduler portent `x-scheduler-job` et utilisent une URL interne.

## 6. Configuration

- Un reglage lie a une guilde appartient a `bot_definitions.config_schema` et
  `bot_guild_config`, pas a une variable d'environnement seule.
- Les variables d'environnement servent aux secrets, connexions, ports,
  defaults globaux et parametres d'exploitation.
- Toute nouvelle variable doit etre ajoutee a `.env.example`, a la
  documentation technique et au compose concerne.
- Ne jamais afficher la valeur d'une variable secrete dans une erreur ou un log.

## 7. Web

- Les quatre univers sont `sentinel`, `nexus`, `atrium` et `ops`.
- L'univers vient de `route.meta.universe`, jamais d'une deduction par URL.
- Respecter l'ordre atomic design : `atoms -> molecules -> organisms ->
  templates -> pages`.
- Les pages admin utilisent le shell admin commun ; les pages publiques utilisent
  le layout public ; les pages d'authentification utilisent le layout bare.
- Les clients HTTP ne doivent pas etre fusionnes si leur mode d'authentification
  differe.
- Le web ne contient aucune regle d'autorisation faisant foi : l'API tranche.

## 8. Methode de modification

Avant de coder :

1. Lire le module metier concerne, son port et l'adaptateur existant.
2. Rechercher les consommateurs et les tests avant de changer un contrat.
3. Verifier si la fonctionnalite existe deja dans `platform-core`.
4. Choisir le plus petit changement compatible avec les frontieres ci-dessus.

Apres modification :

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd web; npm run lint; npm run build
```

Les tests PostgreSQL necessitent une base de test dediee et une
`DATABASE_URL`. Ne jamais lancer une suite destructive contre une base de
developpement ou de production.

## 9. Restrictions d'intervention

- Ne pas demarrer, arreter ou recreer les services Docker sans demande explicite.
- Ne pas supprimer une migration, un volume, une table ou un fichier utilisateur
  sans cible precise et verification prealable.
- Ne pas ajouter de `#[allow(dead_code)]` sans justification locale ; supprimer
  le code mort ou le borner a `#[cfg(test)]` en priorite.
- Ne pas introduire de compatibilite avec une ancienne crate simplement parce
  qu'un document ou un commentaire la mentionne : verifier le code actuel.
- Toute modification d'architecture doit mettre a jour `README.md`, `DOC/` et
  les fichiers Docker/configuration impactes.

## 10. Regle finale

Une modification est correcte si elle respecte les frontieres du domaine,
conserve le fail-closed, ne contourne pas les ports, ne donne pas de privilege
supplementaire a un composant et laisse le depot compilable.
