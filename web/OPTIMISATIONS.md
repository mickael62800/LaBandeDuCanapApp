# Audit d'optimisation Web

Date de l'audit : 11 aout 2026

## Perimetre

Cet audit couvre l'application Vue 3, ses clients HTTP, l'authentification,
le temps reel, le routage, les composants, les tests, le bundle Vite et
l'image Nginx qui sert le site.

Aucun changement fonctionnel n'a ete applique pendant l'audit.

## Etat des validations

- Les 60 tests Vitest passent dans 4 fichiers de tests.
- Le build TypeScript et Vite passe en 4,75 secondes.
- ESLint ne remonte aucune erreur, mais 38 avertissements de mise en forme
  dans `NexusGrandSalonPage.vue`.
- Le plus gros chunk est `vendor-charts` : 186,11 ko, soit 65,04 ko gzip.
- Le JavaScript d'entree pese 73,54 ko, soit 24,62 ko gzip.
- Le code splitting des pages est globalement sain et aucun chunk ne depasse
  le seuil Vite configure de 500 ko.

La base est donc compilable et fonctionnelle, mais les tests actuels couvrent
tres peu les chemins critiques : session, clients HTTP, route guards, temps
reel et principaux composants.

## Synthese des priorites

1. Rendre les erreurs HTTP typees et corriger la revocation d'acces 403.
2. Unifier le transport des quatre APIs et le renouvellement de session.
3. Reparer le contrat des evenements WebSocket et ajouter la reconnexion.
4. Annuler ou ignorer les requetes devenues obsoletes lors d'un changement de
   guilde, filtre ou page.
5. Proteger le demarrage et tous les appels reseau avec des timeouts.
6. Decouper les composants de 600 a 1 855 lignes.
7. Etendre les tests avant les refactorings structurels.

## Priorite 0 - Rendre les erreurs HTTP exploitables

Le client principal transforme les reponses non 2xx en simples `Error`. Le
message visible est extrait du corps JSON et ne contient generalement plus le
code HTTP.

`authStore.checkSession` tente pourtant de detecter une revocation d'acces avec
`msg.includes("403")`. Si le serveur repond avec un message humain sans le
nombre `403`, l'utilisateur conserve son identite en cache. Le route guard le
laisse alors entrer dans le back-office, tandis que les appels suivants
echouent.

### Proposition

- Creer une erreur HTTP commune avec au minimum `status`, `code`, `message`,
  `body` et eventuellement `requestId`.
- Tester `error.status === 403`, jamais le texte affiche a l'utilisateur.
- Centraliser la reaction aux statuts 401 et 403 : refresh, purge de session,
  redirection et message utilisateur.
- Conserver le message metier du serveur pour l'interface sans perdre les
  metadonnees techniques.
- Ajouter des tests pour 200, 401 avec refresh reussi, 401 avec refresh echoue,
  403 et reponse non JSON.

### Fichiers concernes

- `src/api/http.ts`
- `src/api/backendHttp.ts`
- `src/stores/authStore.ts`
- `src/utils/errMsg.ts`

## Priorite 0 - Unifier les clients des APIs

Le Web possede deux transports authentifies dont les comportements divergent :

- `api/http.ts` renouvelle la session apres un 401, rejoue la requete et
  redirige si la session est perdue ;
- `api/backendHttp.ts`, utilise pour Nexus, Ops et Atrium, remonte seulement
  une erreur 401.

Une session expiree peut donc continuer a fonctionner sur Sentinel apres un
refresh transparent tout en echouant sur les trois autres univers. Les deux
clients dupliquent egalement la construction des en-tetes, le parsing JSON et
la traduction des erreurs.

`httpGetWithTotal` reimplemente encore le transport principal uniquement parce
que `request` ne rend pas les en-tetes de reponse.

### Proposition

- Construire un noyau `request` unique acceptant une base URL, une politique
  d'authentification, un parseur de reponse et une politique de retry.
- Faire beneficier Sentinel, Nexus, Ops et Atrium du meme refresh deduplique.
- Retourner une structure interne `{ data, response }` afin d'exposer les
  en-tetes sans dupliquer tout le flux.
- N'appliquer les retries automatiques qu'aux operations idempotentes et
  respecter `Retry-After` lorsque le serveur le fournit.
- Maintenir de petits adaptateurs par backend uniquement pour leurs messages
  metier et leurs prefixes.

### Fichiers concernes

- `src/api/http.ts`
- `src/api/backendHttp.ts`
- `src/services/nexusService.ts`
- `src/services/opsService.ts`
- `src/services/atriumService.ts`

## Priorite 0 - Reparer le temps reel

Le service WebSocket republie chaque frame sous le nom `ws:<event>`. Par
exemple, une frame `bot_heartbeat` devient `ws:bot_heartbeat`.

`ConnectionBanner` et `useNotifications` ecoutent toutefois `ws:event` puis
cherchent un champ `event` dans le payload. Ce canal generique n'est jamais
emis par `realtimeService`. Les notifications construites par
`eventToNotification`, les toasts de logs et la detection de heartbeat ne
peuvent donc pas fonctionner avec ce contrat.

La connexion a aussi plusieurs fragilites :

- aucune reconnexion n'est implementee, bien qu'un commentaire indique qu'un
  retry serait gere ailleurs ;
- la promesse de `connect()` n'est ni rejetee ni resolue si `onerror` ou
  `onclose` arrive avant `onopen` ;
- `MainLayout` peut donc rester suspendu dans son callback `onMounted` ;
- un passage hors ligne ou une interruption du proxy coupe definitivement le
  flux jusqu'a une action externe ;
- `ConnectionBanner` ne relance le healthcheck periodique que lorsque l'etat
  vaut deja `ok`, il ne detecte donc pas seul le retour du serveur apres une
  panne ;
- les fonctions de desabonnement rendues aux composants restent aussi
  conservees dans la liste globale du store jusqu'au nettoyage complet.

### Proposition

1. Choisir un contrat unique : emettre a la fois un canal generique
   `ws:event` avec `{ event, data }` et le canal specialise `ws:<event>`, ou
   migrer tous les consommateurs vers les canaux specialises.
2. Rejeter `connect()` sur erreur/fermeture avant ouverture et lui appliquer
   un timeout.
3. Ajouter une reconnexion exponentielle avec jitter et un maximum borne.
4. Suspendre les tentatives hors ligne et, si utile, quand le document est
   masque ; reprendre sur les evenements `online` et `visibilitychange`.
5. Renouveler la session avant une reconnexion si le jeton est expire.
6. Faire du store l'unique proprietaire de la connexion et supprimer les
   desabonnements de sa liste des qu'ils sont executes.
7. Tester ouverture, erreur initiale, fermeture, reconnexion, changement de
   token et distribution des deux types d'evenements.

### Ne plus transporter un jeton durable dans l'URL

Le token Discord ou la cle API est place dans la query string WebSocket. Nginx
masque deja cette query dans son propre format de log, ce qui est un bon filet
de securite. L'URL peut toutefois encore apparaitre dans des diagnostics
navigateur, des traces de proxy amont ou d'autres outils d'observabilite.

La solution cible est une authentification par cookie HttpOnly same-origin ou
l'echange HTTP d'une session valide contre un ticket WebSocket a usage unique
et tres courte duree. Il faut migrer le gateway et le client ensemble.

### Fichiers concernes

- `src/services/realtimeService.ts`
- `src/stores/realtimeStore.ts`
- `src/composables/useNotifications.ts`
- `src/components/atoms/ConnectionBanner.vue`
- `src/components/templates/MainLayout.vue`
- `nginx.conf`

## Priorite 0 - Eviter les courses entre requetes

`useGuildFetch` lance un nouveau chargement lors d'un changement de guilde ou
d'une source observee, mais n'annule pas le precedent et ne verifie pas que sa
reponse est toujours la plus recente.

Une requete lente pour la guilde A peut donc se terminer apres celle de la
guilde B et remplacer les donnees visibles par celles de A. Le meme probleme
existe pour les recherches, filtres et navigations rapides. Plusieurs watchers
peuvent aussi declencher des appels identiques et le premier appel termine peut
mettre `loading` a `false` alors qu'un second est toujours actif.

### Proposition

- Fournir un `AbortSignal` aux fetchers et annuler la requete precedente.
- Ajouter un compteur de sequence `latest wins` comme seconde protection.
- Construire une cle de requete stable a partir de la guilde, des filtres et
  de la pagination afin de dedupliquer les appels identiques.
- Separer les etats `initialLoading`, `refreshing`, `error` et `stale`.
- Centraliser cache, duree de fraicheur et invalidation. Une primitive interne
  suffit ; TanStack Vue Query n'est utile que si l'equipe veut deleguer aussi
  le cache, les retries et la synchronisation de fenetre.
- Tester explicitement A lent -> B rapide et deux refreshs simultanes.

### Fichiers concernes

- `src/composables/useGuildFetch.ts`
- `src/composables/useFetch.ts`
- les pages qui observent guilde, filtres ou pagination

## Priorite 1 - Borner tous les appels reseau

Les clients HTTP principaux, le client public et le chargement de
`site-config.json` n'ont pas de timeout. Une connexion TCP suspendue peut donc
laisser un ecran en chargement sans limite.

Le montage Vue attend actuellement la fin de `loadSiteConfig()`. Une requete
qui ne termine jamais empeche toute l'application de s'afficher, y compris les
pages qui n'utilisent pas cette configuration.

### Proposition

- Composer un `AbortSignal.timeout` avec le signal fourni par l'appelant.
- Choisir des budgets distincts : court pour la configuration/health, normal
  pour les lectures, plus long et explicite pour les operations Docker.
- Monter l'application immediatement avec une configuration reactive par
  defaut, ou imposer un timeout court avant le montage.
- Normaliser les erreurs timeout, offline, annulation volontaire et serveur.
- Suspendre les pollings lorsque l'onglet est masque ou le navigateur offline.

### Fichiers concernes

- `src/main.ts`
- `src/siteConfig.ts`
- `src/api/http.ts`
- `src/api/backendHttp.ts`
- `src/services/publicHttp.ts`

## Priorite 1 - Assainir le stockage local

`getApiConfig`, `getDiscordUser` et `Store.get` appellent `JSON.parse` sans
recuperation. Une valeur partiellement ecrite, modifiee par une ancienne
version ou corrompue manuellement peut casser le bootstrap.

L'identite est en outre stockee sous deux formes :

- `ds.discord.user` via `config.ts` ;
- `ds.store:auth.json:discord_user` via le faux store asynchrone.

Cette double source de verite explique les branches de restauration et de
purge dupliquees dans `authStore`. L'abstraction `Store` conserve une semantique
de fichier/asynchrone alors qu'elle enveloppe directement `localStorage`.

Enfin, `getApiBaseUrl` met l'URL en cache sans invalidation ; une mise a jour de
configuration n'est pas necessairement visible avant rechargement.

### Proposition

- Utiliser un parseur defensif versionne qui supprime ou migre une valeur
  invalide.
- Conserver une seule source pour l'identite et une seule fonction de purge de
  session.
- Remplacer le store generique par des repositories de stockage synchrones et
  types, ou documenter la raison de conserver son API asynchrone.
- Invalider le cache d'URL dans `setApiConfig`, ou calculer cette URL sans cache.
- Garder le token en `sessionStorage` et la whitelist d'origines existante :
  ces deux protections sont pertinentes.

### Fichiers concernes

- `src/api/config.ts`
- `src/api/store.ts`
- `src/stores/authStore.ts`
- `src/services/authService.ts`
- `src/utils/api.ts`

## Priorite 2 - Decouper les god files

Les fichiers les plus volumineux sont :

| Fichier | Lignes |
| --- | ---: |
| `MemberHomePage.vue` | 1 855 |
| `GamesPage.vue` | 1 364 |
| `ModerationJournalTab.vue` | 824 |
| `CommunityLifePage.vue` | 781 |
| `ServerBuilderPage.vue` | 722 |
| `ComponentConfigForm.vue` | 697 |
| `DockerAdminSection.vue` | 684 |
| `WelcomeForm.vue` | 673 |
| `AnnouncementFormModal.vue` | 662 |
| `NexusServerDetailPage.vue` | 659 |
| `TopBar.vue` | 608 |
| `AtriumPage.vue` | 594 |

`src/types/index.ts` atteint egalement 593 lignes et melange les domaines.

### Decoupage recommande

- `MemberHomePage` : entete de profil, resume des statistiques, presence,
  activite, progression et sections publiques ; garder l'orchestration dans un
  composable de page.
- `GamesPage` : catalogue/filtres, carte de jeu, etat serveur, details/modal et
  appel a rejoindre ; isoler le chargement public.
- `ModerationJournalTab` : filtres, table, selection/actions groupees et modal
  de detail.
- `CommunityLifePage` : extraire chaque onglet ou fonctionnalite en panneau.
- `ServerBuilderPage` : arbre/canvas, inspecteur, outils et sauvegarde.
- `ComponentConfigForm` : rendre chaque famille de champ via un composant et
  isoler mapping, valeurs par defaut et validation.
- `DockerAdminSection` : overview, conteneurs, images, volumes, logs et actions,
  avec un composable dedie au polling.
- `WelcomeForm` et `AnnouncementFormModal` : formulaire, apercu, ciblage et
  planification.
- `NexusServerDetailPage` : resume, configuration, runtime, joueurs et actions.
- `TopBar` : selecteur de guilde, navigation d'univers et menu utilisateur.
- `types/index.ts` : separer `auth`, `guild`, `moderation`, `community`,
  `nexus`, `ops` et `atrium`.

Chaque extraction doit commencer par des tests de comportement. L'objectif
n'est pas seulement de diminuer le nombre de lignes, mais de donner a chaque
bloc une responsabilite, des props/emits types et un cycle de vie autonome.

## Priorite 3 - Elargir la couverture de tests

Les 60 tests actuels couvrent quatre zones isolees : bornage numerique,
bannieres, builder de serveur et planning hebdomadaire. Ils ne protegent pas
les principaux risques releves pendant l'audit.

### Socle recommande

- tests unitaires du transport HTTP et de l'erreur typee ;
- tests du refresh concurrent et du replay unique ;
- tests du store d'authentification et des route guards ;
- tests WebSocket avec serveur simule, reconnexion et distribution d'events ;
- tests du stockage corrompu et des migrations de schema ;
- tests des races de requetes et annulations ;
- tests composants des formulaires et modals critiques ;
- smoke tests E2E de connexion et de navigation dans Sentinel, Nexus, Ops,
  Atrium et les pages publiques ;
- test E2E d'expiration/revocation de session.

La CI devrait executer `npm test -- --run`, `npm run build` et ESLint avec
`--max-warnings=0`. Les 38 avertissements actuels doivent etre corriges avant
d'activer cette derniere regle.

## Priorite 4 - Ameliorer l'accessibilite

Plusieurs modals utilisent une simple `div` de backdrop sans contrat de
dialogue visible dans le template. Il faut verifier systematiquement :

- `role="dialog"` ou l'element natif `dialog` ;
- `aria-modal`, titre associe et description si necessaire ;
- focus initial, piege de focus et restitution du focus a la fermeture ;
- fermeture clavier avec `Escape` ;
- navigation complete au clavier pour menus et panneaux ;
- annonce des erreurs, toasts et changements de statut avec des live regions ;
- contrastes, libelles de champs et etats disabled/loading ;
- respect de `prefers-reduced-motion` pour les animations CSS comme pour la
  bibliotheque Motion.

Ajouter `eslint-plugin-vuejs-accessibility` et quelques tests axe sur les
layouts, formulaires et modals donnera un filet automatique. Un audit manuel
clavier et lecteur d'ecran reste necessaire.

## Priorite 5 - Optimiser le bundle sans casser le splitting existant

Les routes sont deja majoritairement chargees a la demande et Chart.js
enregistre uniquement les composants utilises. Le `dist/index.html` ne
precharge que Vue et Motion : le chunk Chart.js n'est pas impose au premier
affichage. Ce fonctionnement est a conserver.

Le build signale toutefois trois imports dynamiques sans effet :

- `moderationService.ts` est aussi importe statiquement ;
- `api/http.ts` est partage par de nombreux imports statiques ;
- `useGuildSelector.ts` est partage par de nombreux imports statiques.

Rollup ne peut pas creer de chunk dynamique pour un module deja present dans
le graphe statique.

### Proposition

- Remplacer ces faux imports dynamiques par des imports statiques clairs, ou
  deplacer reellement la frontiere de lazy loading au niveau d'une feature.
- Ajouter un budget de taille dans la CI pour l'entree et les chunks gzip.
- Mesurer avant d'ajouter d'autres `manualChunks` : le decoupage actuel est
  deja raisonnable.
- Evaluer le cout de Motion : son chunk de 13,67 ko gzip est precharge pour
  toute l'application alors que le commentaire le destine a la page publique.
  Si les animations sont peu nombreuses, du CSS ou une initialisation limitee
  au layout public peut eviter ce cout aux administrateurs.
- Conserver l'enregistrement selectif de Chart.js dans `utils/chartjs.ts`.
- Verifier les images publiques et utiliser AVIF/WebP, tailles explicites et
  lazy loading pour les contenus sous la ligne de flottaison.

## Priorite 6 - Centraliser les erreurs et l'observabilite frontend

De nombreux composants et composables appellent directement `console.error`
avec des formats differents. Les utilisateurs recoivent parfois un toast
generique qui perd le statut, le backend et l'identifiant de requete.

### Proposition

- Centraliser la normalisation des erreurs et leur affichage.
- Associer backend, route, methode, duree, statut et request ID aux erreurs
  techniques, sans journaliser les tokens ni les corps sensibles.
- Dedupliquer les toasts lorsque plusieurs composants echouent sur la meme
  panne.
- Instrumenter les Web Vitals, les erreurs non gerees, les refus de promesse et
  le taux de reconnexion WebSocket.
- Garder les messages metier courts dans l'UI et envoyer le contexte detaille a
  l'outil d'observabilite.

## Priorite 7 - Docker et Nginx

La configuration actuelle possede deja de bons choix :

- build multi-stage Node -> Nginx ;
- `npm ci`, contexte racine filtre et absence de `node_modules` dans l'image ;
- precompression gzip servie avec `gzip_static` ;
- cache d'un an immutable pour les assets hashes et absence de cache pour le
  document SPA ;
- healthcheck, TLS moderne, CSP, HSTS et headers de securite ;
- masquage de la query string WebSocket dans le log Nginx ;
- resolution DNS dynamique des services Docker ;
- secrets des APIs internes injectes par Nginx et jamais livres au SPA.

Les optimisations restantes sont secondaires par rapport aux P0 :

- pinner les images Node et Nginx a une version de patch, voire un digest,
  avec une procedure reguliere de mise a jour ;
- utiliser un cache BuildKit pour le cache npm si les builders CI sont
  persistants ;
- regrouper la generation des quatre snippets de secrets dans un entrypoint
  generique parametre pour reduire la duplication, tout en conservant des
  fichiers de sortie et permissions distincts ;
- evaluer Brotli uniquement apres mesure du temps de build, de la taille
  d'image et du gain de transfert ; gzip est deja correctement deploye ;
- tester la configuration avec `nginx -t` dans le build ou la CI ;
- ajouter des tests de proxy pour les prefixes, `auth_request`, les timeouts,
  les headers et l'absence de secrets dans les logs ;
- envisager une image Nginx non-root dans un chantier dedie, car les ports
  80/443, les certificats et les entrypoints exigent une migration coordonnee.

Il n'est pas utile de multiplier les Dockerfiles Web : les stages actuels sont
coherents et une seule image statique sert tous les univers. Separer les images
publique et admin ne devient interessant que si la mesure confirme un gain de
bundle ou si leurs cycles de deploiement divergent.

## Ordre d'implementation recommande

### Lot 1 - Fiabilite critique

- erreur HTTP typee et correction du 403 ;
- transport commun et refresh multi-API ;
- timeouts et annulations ;
- tests du transport et de l'authentification.

### Lot 2 - Temps reel

- contrat d'evenements unique ;
- reconnexion et promesse de connexion bornee ;
- correction de la banniere et des notifications ;
- tests WebSocket.

### Lot 3 - Donnees et stockage

- requetes `latest wins`, deduplication et cache ;
- stockage versionne avec une seule identite ;
- tests de courses et de migration.

### Lot 4 - Decoupage

- `MemberHomePage` et `GamesPage` ;
- journal de moderation et vie communautaire ;
- builder/configuration/Docker ;
- navigation, pages Nexus/Atrium et types par domaine.

### Lot 5 - Qualite et performance

- tests composants et E2E ;
- accessibilite ;
- suppression des imports dynamiques inefficaces ;
- budget de bundle et optimisations Nginx mesurees.

## Criteres de sortie

Le chantier peut etre considere termine lorsque :

- une session expiree ou revoquee produit le meme comportement sur les quatre
  APIs ;
- aucune reponse obsolete ne peut remplacer les donnees de la selection
  courante ;
- le WebSocket se reconnecte et les notifications recoivent effectivement les
  evenements ;
- aucun appel reseau critique ne peut bloquer indefiniment l'application ;
- les god files prioritaires sont couverts puis decoupes ;
- tests, build et lint avec zero avertissement passent en CI ;
- les budgets de bundle et les parcours E2E critiques sont surveilles.
