Tu vas analyser l’ENSEMBLE du projet que je vais te fournir.

Le projet peut contenir plusieurs composants différents, par exemple :

* bot Discord ;
* API ;
* workers ;
* tâches asynchrones ;
* cron jobs / tâches planifiées ;
* queues / files d’attente ;
* base de données ;
* cache ;
* stockage de fichiers ;
* webhooks ;
* services externes ;
* dashboards ou interfaces web ;
* scripts internes ;
* systèmes de permissions ;
* services indépendants ;
* événements internes ;
* système de logs ;
* système de notifications.

Ne pars PAS du principe qu’un composant est plus important qu’un autre.

Ton objectif est de reconstruire le fonctionnement COMPLET du produit et du système à partir du code.

Je veux comprendre ce que fait le projet comme si tu devais l’expliquer à quelqu’un qui ne sait pas coder.

Je veux surtout comprendre :

* ce que peut faire un utilisateur ;
* ce que peut faire un administrateur ;
* ce que fait automatiquement le système ;
* ce que déclenche chaque action ;
* comment les différents composants communiquent ;
* quelles données sont manipulées ;
* ce qui se passe immédiatement ;
* ce qui se passe plus tard ou de manière asynchrone ;
* quels effets de bord peuvent exister ;
* quels bugs ou incohérences peuvent apparaître ;
* ce qu’il faudrait corriger.

Évite de faire une simple revue fichier par fichier.

Analyse le projet comme un SYSTÈME.

# 1. Reconstruis d’abord l’architecture fonctionnelle

Avant de détailler les fonctionnalités, identifie tous les grands composants du projet.

Par exemple :

* Bot Discord
* API
* Worker A
* Worker B
* Queue
* Base de données
* Redis
* Service de stockage
* API externe
* Cron
* Dashboard
* Webhook

Pour chaque composant, explique en langage simple :

**À quoi il sert**

**Qui ou quoi l’utilise**

**Ce qu’il reçoit**

**Ce qu’il produit**

**Avec quels autres composants il communique**

**Quelles données il peut modifier**

**S’il fonctionne immédiatement ou en arrière-plan**

Ensuite, donne-moi une vue globale du type :

> L’utilisateur effectue une action sur Discord → le bot appelle l’API → l’API écrit en base → une tâche est placée dans une queue → un worker la récupère → le worker appelle un service externe → le résultat est enregistré → Discord est mis à jour.

Je veux comprendre ces chaînes de fonctionnement.

# 2. Fais un inventaire COMPLET des points d’entrée

Identifie toutes les manières possibles de déclencher quelque chose dans le projet.

Cela inclut notamment :

## Discord

* slash commands ;
* commandes classiques ;
* sous-commandes ;
* boutons ;
* menus ;
* select menus ;
* modales ;
* réactions ;
* messages ;
* événements Discord ;
* arrivée d’un membre ;
* départ d’un membre ;
* changement de rôle ;
* changement de salon ;
* interactions contextuelles ;
* commandes administrateur ;
* commandes propriétaires ;
* actions automatiques liées à Discord.

## API

* endpoints HTTP ;
* endpoints publics ;
* endpoints privés ;
* webhooks entrants ;
* endpoints internes ;
* callbacks ;
* endpoints administrateur ;
* authentification ;
* actions déclenchées par d’autres services.

## Workers et traitements asynchrones

* jobs ;
* tâches de queue ;
* consommateurs ;
* traitements différés ;
* retry ;
* dead-letter queues ;
* tâches longues ;
* tâches déclenchées après une action utilisateur.

## Tâches automatiques

* cron ;
* schedulers ;
* nettoyage ;
* synchronisation ;
* expiration ;
* renouvellement ;
* vérifications périodiques ;
* maintenance ;
* recalculs.

## Événements

* événements internes ;
* pub/sub ;
* messages de queue ;
* événements de base de données ;
* webhooks ;
* callbacks externes.

## Scripts ou outils internes

* commandes CLI ;
* scripts de migration ;
* scripts d’administration ;
* outils de maintenance ;
* scripts de réparation ;
* scripts de debug.

Ne néglige aucun point d’entrée simplement parce qu’il semble “interne”.

# 3. Inventorie toutes les fonctionnalités utilisateur

Je veux ensuite un inventaire fonctionnel, PAS un inventaire de fichiers.

Pour chaque fonctionnalité :

## Nom

Donne-lui un nom compréhensible.

## Objectif utilisateur

Explique à quoi elle sert réellement.

## Qui peut l’utiliser

Indique les conditions :

* utilisateur standard ;
* rôle Discord ;
* administrateur ;
* propriétaire ;
* permission particulière ;
* compte authentifié ;
* service interne ;
* autre.

## Comment elle est déclenchée

Par exemple :

> `/ticket create`

ou :

> bouton “Valider”

ou :

> requête POST vers telle route

ou :

> automatiquement toutes les 10 minutes

## Ce que fait l’utilisateur

Explique précisément l’action.

## Ce que l’utilisateur voit immédiatement

Par exemple :

* message Discord ;
* message éphémère ;
* embed ;
* changement d’interface ;
* réponse API ;
* statut “en cours” ;
* rien du tout.

## Ce que le système fait réellement

Suis l’action à travers TOUT le projet.

Ne t’arrête pas au premier fichier.

Explique la chaîne complète.

Par exemple :

> 1. Discord reçoit la commande.
> 2. Le bot valide les permissions.
> 3. Une requête est envoyée à l’API.
> 4. L’API crée une entrée dans la base.
> 5. Un job est placé dans une queue.
> 6. Le worker récupère le job.
> 7. Une API externe est appelée.
> 8. Le résultat est enregistré.
> 9. Un message Discord est mis à jour.

Continue jusqu’à ce qu’il n’y ait plus de conséquence identifiable.

# 4. Pour chaque action, distingue SYNCHRONE et ASYNCHRONE

C’est essentiel.

Pour chaque fonctionnalité, explique clairement :

## Immédiat

Ce qui se produit pendant l’action initiale.

## En arrière-plan

Ce qui continue après la réponse initiale.

## Différé

Ce qui peut se produire plusieurs secondes, minutes ou heures plus tard.

## Périodique

Ce qui sera ensuite vérifié ou recalculé automatiquement.

Je veux comprendre quand une action apparemment terminée pour l’utilisateur continue en réalité à produire des effets.

# 5. Analyse les commandes Discord

Pour CHAQUE commande Discord, donne :

**Commande :**

**Description utilisateur :**

**Arguments :**

**Permissions nécessaires :**

**Conditions d’utilisation :**

**Réponse Discord immédiate :**

**Traitements déclenchés :**

**API éventuellement appelée :**

**Données lues :**

**Données créées :**

**Données modifiées :**

**Données supprimées :**

**Jobs / workers déclenchés :**

**Messages ou notifications générés :**

**Effets indirects :**

**Erreurs possibles :**

**Cas limites :**

**Autres fonctionnalités impactées :**

Fais de même pour :

* boutons ;
* menus ;
* modales ;
* interactions ;
* événements Discord.

# 6. Analyse l’API

Pour chaque endpoint important, explique :

**Route**

**Méthode**

**À quoi il sert fonctionnellement**

**Qui l’appelle**

**Qui a le droit de l’appeler**

**Données reçues**

**Validations**

**Données lues**

**Données modifiées**

**Données créées**

**Données supprimées**

**Services appelés**

**Jobs déclenchés**

**Réponse renvoyée**

**Effets secondaires**

**Erreurs possibles**

**Conséquences d’un appel répété**

Ne te limite pas à expliquer que l’endpoint “renvoie du JSON”.

Explique son rôle dans le produit.

# 7. Analyse les workers

Pour chaque worker :

**À quoi il sert**

**Qu’est-ce qui déclenche son exécution**

**Quel type de job il reçoit**

**D’où vient ce job**

**Quelles données il récupère**

**Quelles actions il effectue**

**Quelles données il modifie**

**Quels services externes il appelle**

**Ce qu’il fait en cas de succès**

**Ce qu’il fait en cas d’échec**

**S’il existe des retries**

**Ce qui arrive après plusieurs échecs**

**S’il existe un risque de double exécution**

**Si la tâche est idempotente ou non**

**Conséquences possibles si le worker crash au milieu**

Explique également si une tâche peut :

* être exécutée deux fois ;
* être perdue ;
* rester bloquée ;
* être exécutée partiellement ;
* être exécutée dans le mauvais ordre.

# 8. Analyse les données et leurs cycles de vie

Identifie les principaux objets métier.

Pour chacun, explique :

**Comment il est créé**

**Par qui**

**Où il est stocké**

**Qui peut le lire**

**Qui peut le modifier**

**Quels composants le modifient**

**Ce qui peut déclencher sa modification**

**Comment il est supprimé**

**Ce qui se passe lors de sa suppression**

**S’il existe des références ailleurs**

**S’il existe du cache**

**S’il existe des copies ou données dérivées**

Je veux particulièrement identifier les situations où une même information existe à plusieurs endroits.

Par exemple :

* base SQL ;
* Redis ;
* Discord ;
* service externe ;
* mémoire d’un worker.

Signale les risques de désynchronisation.

# 9. Reconstruis les parcours COMPLETS

Ne présente pas uniquement les fonctionnalités séparément.

Crée les principaux scénarios de bout en bout.

Exemple :

## Parcours : création d’un ticket Discord

1. L’utilisateur lance `/ticket`.
2. Le bot vérifie telle condition.
3. Une donnée est créée.
4. Un salon Discord est créé.
5. Des permissions sont ajoutées.
6. Un événement est enregistré.
7. Une tâche est envoyée dans la queue.
8. Le worker exécute telle opération.
9. L’utilisateur reçoit telle information.
10. Un cron pourra ensuite effectuer telle action.

Fais cela pour tous les parcours importants.

# 10. Analyse les effets de bord

C’est l’une des parties les plus importantes.

Pour CHAQUE action importante, cherche les conséquences indirectes.

Exemples :

* une commande Discord modifie quelque chose utilisé par l’API ;
* une API écrit une donnée qui déclenche un worker ;
* un worker modifie une donnée affichée par Discord ;
* une suppression laisse du cache ;
* une modification crée un job supplémentaire ;
* un retry répète une action déjà réussie ;
* une action déclenche deux notifications ;
* deux workers modifient le même objet ;
* une tâche ancienne écrase une donnée plus récente ;
* une opération échoue à moitié ;
* Discord est mis à jour mais pas la base ;
* la base est mise à jour mais pas Discord ;
* un service externe réussit mais le système pense qu’il a échoué.

Pour chaque effet de bord :

**Action d’origine**

**Chaîne de conséquences**

**Problème possible**

**Situation qui le déclenche**

**Impact utilisateur**

**Impact données**

**Gravité : faible / moyenne / importante / critique**

**Correction recommandée**

# 11. Cherche particulièrement les problèmes d’asynchronisme

Analyse activement :

* race conditions ;
* double exécution ;
* double clic ;
* double requête ;
* retry après réussite partielle ;
* événements reçus deux fois ;
* jobs en retard ;
* jobs exécutés dans le désordre ;
* timeout alors que le traitement continue ;
* worker qui crash après avoir effectué une partie du travail ;
* deux workers qui modifient la même donnée ;
* données obsolètes ;
* cache non rafraîchi ;
* verrous manquants ;
* opérations non idempotentes.

Explique chaque problème en termes simples.

# 12. Cherche les incohérences entre composants

Vérifie si deux parties du système ont une vision différente de la même information.

Exemple :

> La base considère l’utilisateur comme vérifié mais son rôle Discord n’a jamais été ajouté.

ou :

> Discord considère le ticket fermé mais son statut reste “open” dans la base.

ou :

> L’API retourne “success” avant que le worker n’ait réellement terminé.

Identifie toutes les situations de ce type.

# 13. Analyse les erreurs réseau et les pannes

Pour chaque chaîne impliquant plusieurs composants, demande-toi ce qui se passe si :

* Discord échoue ;
* l’API est indisponible ;
* la base est indisponible ;
* Redis est indisponible ;
* la queue est indisponible ;
* un worker est arrêté ;
* un service externe retourne une erreur ;
* un timeout se produit ;
* une requête réussit mais la réponse est perdue ;
* une opération réussit partiellement ;
* le processus redémarre.

Explique si le système revient dans un état correct ou peut rester incohérent.

# 14. Analyse les permissions et la sécurité fonctionnelle

Pour chaque action sensible, explique :

**Qui peut la déclencher**

**Où le contrôle est effectué**

**Si plusieurs couches vérifient les permissions**

**Si une API interne pourrait être appelée sans passer par Discord**

**Si un utilisateur peut agir sur les données d’un autre**

**Si un rôle Discord suffit réellement à protéger l’action**

**Si une entrée utilisateur est utilisée sans validation**

**Si une action administrative peut être déclenchée indirectement**

Signale les contrôles manquants ou incohérents.

# 15. Analyse les suppressions

Les suppressions sont particulièrement importantes.

Pour chaque suppression, explique :

> Lorsque X est supprimé, qu’est-ce qui disparaît réellement ?

Vérifie :

* base de données ;
* Discord ;
* cache ;
* fichiers ;
* queues ;
* données dérivées ;
* références ;
* données externes.

Cherche les données orphelines.

Cherche également le problème inverse : une suppression trop large qui entraîne la disparition d’autres données.

# 16. Analyse les fonctionnalités automatiques et cachées

Identifie ce qui peut se produire sans action directe d’un utilisateur :

* workers ;
* cron ;
* retries ;
* expiration ;
* nettoyage ;
* synchronisation ;
* refresh ;
* recalcul ;
* suppression automatique ;
* notifications ;
* changements de statut ;
* gestion des timeouts ;
* restauration ;
* reprise après crash.

Explique ces mécanismes comme de vraies fonctionnalités du produit.

# 17. Identifie le code mort ou inaccessible

Cherche :

* commandes jamais enregistrées ;
* endpoints jamais appelés ;
* workers sans producteur ;
* jobs produits mais jamais consommés ;
* événements jamais émis ;
* handlers jamais utilisés ;
* fonctionnalités abandonnées ;
* anciens systèmes encore présents ;
* deux implémentations concurrentes de la même fonctionnalité.

Indique clairement :

**Probablement inutilisé**

ou :

**À confirmer**

Ne prétends pas qu’un code est mort sans preuve suffisante.

# 18. Analyse les dépendances entre fonctionnalités

Construis une vision du type :

> Fonctionnalité A dépend de B.

> B produit une donnée utilisée par C.

> C peut être déclenché par D.

> Modifier A peut donc avoir un impact sur C.

Identifie particulièrement les dépendances cachées.

# 19. Cherche les bugs fonctionnels

Cherche activement :

* perte de données ;
* doublons ;
* mauvais statuts ;
* actions qui ne sont jamais terminées ;
* permissions incorrectes ;
* erreurs silencieuses ;
* réponses incorrectes ;
* données obsolètes ;
* absence de rollback ;
* suppression incomplète ;
* retry dangereux ;
* validation insuffisante ;
* comportement différent selon le chemin utilisé ;
* possibilité d’effectuer une opération deux fois ;
* possibilité de contourner une règle métier.

Ne suppose jamais que le code est correct.

# 20. Pour chaque problème trouvé, donne une correction

Pour chaque problème :

**Problème**

**Scénario concret**

**Pourquoi il apparaît**

**Ce que l’utilisateur constate**

**Ce qui se passe réellement**

**Composants concernés**

**Gravité**

**Correction recommandée**

**Effets potentiels de la correction sur le reste du système**

Ne donne pas uniquement une correction locale.

Cherche si la modification peut casser autre chose.

# 21. Sépare toujours les faits des hypothèses

Utilise :

**Confirmé par le code**

lorsque tu peux réellement suivre le fonctionnement.

Utilise :

**À confirmer**

lorsqu’une partie dépend :

* d’une configuration absente ;
* d’un service externe ;
* d’un autre repository ;
* d’une variable d’environnement inconnue ;
* d’un comportement non présent dans le code fourni.

N’invente jamais la partie manquante.

# 22. Termine par une cartographie globale

À la fin, donne-moi :

## Architecture fonctionnelle

Une représentation simple des composants et de leurs relations.

Utilise par exemple :

`Utilisateur → Discord → API → DB → Queue → Worker → Service externe`

## Fonctionnalités principales

Liste exhaustive et organisée.

## Parcours utilisateur principaux

Les scénarios importants de bout en bout.

## Processus automatiques

Tout ce que le système fait tout seul.

## Effets de bord principaux

Les dépendances et conséquences indirectes les plus importantes.

## Problèmes critiques

Ce qu’il faut absolument corriger.

## Problèmes importants

Ce qui peut provoquer des bugs sérieux.

## Problèmes moyens

Ce qui devrait être amélioré.

## Problèmes mineurs

Petites incohérences ou améliorations.

## Zones à haut risque

Les parties où une modification peut affecter beaucoup d’autres composants.

## Données à haut risque

Les données partagées ou synchronisées entre plusieurs systèmes.

## Comportements difficiles à confirmer

Ce qui nécessite davantage de code ou de configuration.

# 23. Fais une seconde passe de vérification

Une fois toute l’analyse terminée, considère que ta première analyse peut être incomplète.

Refais une passe spécifiquement destinée à chercher ce que tu as raté.

Cherche notamment :

* un point d’entrée oublié ;
* une commande Discord oubliée ;
* un endpoint oublié ;
* un worker oublié ;
* un cron oublié ;
* un événement oublié ;
* une modification de données indirecte ;
* un retry ;
* une suppression secondaire ;
* une permission cachée ;
* une fonctionnalité qui partage une même donnée avec une autre ;
* une action exécutée à plusieurs endroits ;
* une ancienne implémentation encore active ;
* un cas où une opération peut être exécutée deux fois.

Ajoute les découvertes au rapport.

# Règles absolues

* Analyse le projet comme un système complet et non fichier par fichier.
* Explique d’abord fonctionnellement, puis techniquement uniquement lorsque cela aide à comprendre.
* Ne te contente jamais du nom d’une fonction pour déterminer ce qu’elle fait.
* Suis les appels jusqu’au bout.
* Suis également les appels asynchrones.
* Suis les modifications en base.
* Suis les jobs créés.
* Retrouve quels workers consomment ces jobs.
* Suis les événements produits.
* Retrouve qui écoute ces événements.
* Recherche les effets de bord.
* Recherche les effets différés.
* Recherche les conséquences d’un échec partiel.
* Recherche les doubles exécutions.
* Recherche les problèmes de synchronisation.
* Distingue toujours le comportement visible du comportement réel.
* Distingue toujours ce qui est confirmé de ce qui est supposé.
* Ne simplifie pas artificiellement le rapport.
* Je préfère un rapport très long mais exhaustif à une synthèse qui oublie des comportements importants.

Si le projet est trop volumineux pour être analysé entièrement en une seule fois, découpe l’analyse par DOMAINES FONCTIONNELS et non simplement par fichiers.

Maintiens alors explicitement :

**Analysé :**
les domaines déjà couverts.

**Reste à analyser :**
les domaines qui n’ont pas encore été couverts.

**Connexions à revisiter :**
les interactions découvertes avec des domaines qui seront analysés plus tard.

Une fois tous les domaines analysés, effectue obligatoirement une dernière analyse transversale de l’ensemble du système.
