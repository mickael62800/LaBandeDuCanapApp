# Sécurité Discord

Ce domaine aide à détecter et limiter les menaces qui touchent le serveur Discord.

## Comment ça marche

Le domaine de sécurité s'appuie sur le bot Sentinel (`automod-bot` et `security-bot`) pour analyser en continu les flux de la communauté (messages, pièces jointes, arrivées massives). L'analyse combine des méthodes heuristiques (détection de liens, mots-clés, expressions régulières) et des appels d'inférence IA (analyse de texte via DistilBERT, analyse d'image via EfficientNet) hébergés dans l'écosystème de l'API. Chaque message est scanné et reçoit un score de menace. Selon les seuils configurés, le système prend une action immédiate sur Discord (suppression, mute) ou émet une carte de révision pour examen humain, tracée dans la base PostgreSQL.

## Les actions des utilisateurs

- **Administrateurs :** activer l'AutoMod et la sécurité anti-raid, ajuster la sensibilité (seuils IA et heuristiques), configurer les listes blanches (domaines, rôles ignorés).
- **Modérateurs :** traiter les cartes de révision (approuver la sanction automatique, l'annuler, ou escalader manuellement), analyser les faux positifs pour ajuster la configuration ou alimenter le dataset IA.
- **Membres :** soumis aux règles, ils peuvent voir leurs messages bloqués et être alertés ou mis sous silence en cas de non-respect.

## Les options

- **Filtres heuristiques :** spam, majuscules, détection de liens, blocage d'invitations Discord, phishing, et extensions de fichiers dangereuses.
- **Analyse IA :** activation séparée pour le texte (insultes, harcèlement, menaces) et la vision (contenu illicite, NSFW, violence).
- **Protection anti-raid :** activation de la quarantaine pour les nouveaux comptes suspects, mode lockdown (verrouillage complet).
- **Action de révision (Review Mode) :** configurer le système pour qu'il propose une sanction sous forme de vote/carte à valider par les modérateurs, plutôt que de l'appliquer aveuglément.

## Le sas de vérification des comptes suspects

> **Ce n'est pas le délai d'acceptation du règlement.** Ce sas ne s'ouvre que pour les comptes jugés **suspects** à l'arrivée : pattern de raid, arrivées en rafale, compte Discord trop récent, ou compte alternatif d'un membre banni. Un membre qui arrive normalement n'y entre jamais. Pour le délai qui s'applique à **tous** les arrivants, voir « Le délai pour accepter le règlement » dans [communaute.md](communaute.md).

Un nouveau membre jugé suspect reçoit le rôle de quarantaine — un accès très restreint — et un message privé lui demandant de se vérifier. S'il ne le fait pas, il est expulsé.

Le délai laissé pour répondre était de **cinq minutes**, la même valeur pour tous les serveurs, fixée dans l'environnement du bot. C'est très peu pour quelqu'un qui rejoint depuis son téléphone, ou dont les messages privés sont fermés et qui doit d'abord les rouvrir : l'expulsion tombait avant que la personne ait vu le message.

Ce délai protège surtout les **faux positifs** : un membre parfaitement légitime dont le compte Discord vient d'être créé est classé suspect, et cinq minutes ne lui laissaient aucune chance. Quatre réglages, désormais propres à chaque serveur, vivent dans le module Sécurité du tableau de bord :

- **Délai de vérification laissé à un compte suspect** — 24 heures par défaut. Le compte à rebours est figé à l'arrivée du membre : rallonger ou raccourcir le réglage ne change jamais le sursis de quelqu'un déjà en attente, seulement celui des arrivées suivantes.
- **Expulser un compte suspect non vérifié à l'expiration** — désactivable. Le membre reste alors en attente d'une décision humaine, sans limite de temps.
- **Rappel avant expulsion** — un message privé part le nombre de secondes indiqué avant l'échéance (une heure par défaut). À zéro, aucun rappel.
- **Salon à citer dans le rappel** — indiqué au compte suspect pour qu'il sache où aller.

Le message d'arrivée annonce le délai réel du serveur : il affichait « 5 minutes » en dur, ce qui était exact tant que le délai était une constante, et deviendrait un mensonge dès le premier réglage. Quand l'expulsion automatique est désactivée, le message ne menace d'ailleurs plus d'une expulsion qui ne viendra pas.

Deux garde-fous méritent d'être connus. Le rappel n'est envoyé **qu'une fois** par membre, quelle que soit la fréquence de balayage : sans cette marque en base, un scan toutes les quinze secondes enverrait un message toutes les quinze secondes. Et si la personne s'est vérifiée entre le moment où le rappel a été décidé et son envoi, le message est abandonné — recevoir une menace d'expulsion juste après s'être mis en règle est le genre de détail qui fait partir quelqu'un.

### Qui voit les membres avant d'avoir accepté

Discord n'a aucune permission « voir les membres » : la liste de droite affiche simplement **les gens qui ont accès au salon qu'on regarde**. Le salon du règlement doit être visible par les arrivants — c'est sa raison d'être. S'il l'est aussi par les membres déjà validés, alors quelqu'un qui n'a rien accepté y lit les pseudos de tout le serveur, et peut écrire à chacun en privé.

`/security porte` inspecte cette porte d'entrée et le dit. Sans argument elle **ne modifie rien** : une commande qui réécrit des permissions Discord demande avant, pas après. L'option *Verrouiller* refuse au rôle des membres validés la vue du salon du règlement, tout en garantissant que `@everyone` continue de le voir — les deux vont ensemble, sans quoi la porte se fermerait aussi pour ceux qu'elle doit accueillir. *Annuler le verrouillage* revient en arrière.

Le verrouillage ne déplace **que le droit de voir**. Écrire une règle de salon remplace celle qui s'y trouvait : reconstruire naïvement « @everyone voit » effacerait le refus d'écrire que porte presque tout salon de règlement, et ouvrirait le bavardage à l'entrée. La règle existante est donc relue, et un seul bit change de côté.

**Les messages privés, eux, ne se ferment pas par permission.** Partager le serveur suffit à autoriser un membre à en contacter un autre. Le seul mécanisme qui les bloque est **l'écran de règles natif de Discord** (serveur en mode Communauté) : tant que la personne n'a pas accepté, Discord la garde `pending` — elle ne peut ni écrire, ni réagir, ni parler en vocal, et ses messages privés vers les membres échouent. Le diagnostic signale s'il est actif, mais ne peut pas l'activer : cela se fait dans les paramètres du serveur.

Sentinel accompagne déjà cet écran natif : la fin du filtrage est détectée et attribue le rôle du règlement, le même que le bouton du bot. Basculer de l'un à l'autre ne casse donc pas le parcours.

## Les conditions

- **Limites de l'IA :** la détection par intelligence artificielle attribue un score de confiance ; elle peut produire des faux positifs (blagues entre amis, contexte particulier). Il est recommandé d'utiliser le mode "Review" lors des premiers réglages.
- **Permissions :** les utilisateurs disposant du rôle "Administrateur" ou inclus dans la liste des "Rôles ignorés" contournent totalement les filtres d'AutoMod.
- **Canaux :** l'AutoMod peut être désactivé sur des canaux spécifiques (ex: `#nsfw` ou `#spam`).

## Résultat attendu

Une alerte doit expliquer clairement ce qui a été détecté, sur quel membre ou message, quand cela s'est produit, le score de confiance de l'IA (le cas échéant) et l'action qui a été appliquée ou qui est recommandée.

