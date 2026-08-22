# Vie de la communauté

Ce domaine regroupe les outils qui servent à accueillir les membres, publier des informations et faire vivre le serveur.

## Comment ça marche

Ce domaine agit comme le centre d'animation du serveur. Il connecte les modules du bot Sentinel (comme `welcome-bot`, `roles-bot`, `levels-bot`) à la base de données PostgreSQL via `platform-api`. Lorsqu'un événement survient sur Discord (arrivée d'un membre, activité vocale/écrite, création de ticket), le bot envoie la requête à l'API. L'API valide les règles métier et met à jour l'état. `platform-scheduler` intervient en arrière-plan pour traiter les événements asynchrones (publication planifiée, expiration de rôles temporaires). 

## Les actions des utilisateurs

- **Administrateurs / Animateurs :** configurer le message de bienvenue, planifier des annonces, créer et publier des embeds, gérer les panneaux de rôles, modérer les confessions, configurer les paliers de niveaux et les récompenses.
- **Modérateurs :** traiter les tickets d'aide, répondre aux confessions, valider ou refuser les idées proposées.
- **Membres :** rejoindre le serveur (déclenche l'accueil), choisir des rôles via les panneaux, soumettre des idées, envoyer des confessions, ouvrir des tickets de support, gagner de l'expérience (XP) en participant.

## Les options

- **Bienvenue :** activation du module, salon d'accueil, message personnalisé, carte de bienvenue générée dynamiquement, attribution d'un rôle automatique.
- **Annonces :** date/heure de publication, salon cible, contenu du message, mentions autorisées.
- **Confessions :** anonymat strict, salon de réception privé pour les modérateurs, salon de publication public.
- **Niveaux :** activation du gain d'XP vocal/écrit, configuration des paliers, rôles de récompense associés.
- **Rôles temporaires :** sélection du rôle, durée d'attribution (ex: 7 jours, 1 mois).
- **Panneaux de rôles :** création de catégories, ajout de rôles sélectionnables (exclusifs ou multiples).

## Le délai pour accepter le règlement

Le module Accueil pose le règlement et son bouton, puis n'attendait rien : un membre pouvait rester indéfiniment sans avoir cliqué, sans relance et sans fin. Il occupait une place, ne voyait qu'un salon, et personne ne s'en apercevait.

Le seul mécanisme d'expulsion après délai vivait dans la **quarantaine**, côté Sécurité — mais celle-ci ne se déclenche que sur suspicion (raid, compte trop récent, alt d'un banni) et ne voit donc jamais un arrivant ordinaire. Ses réglages parlaient pourtant d'« accepter le règlement », ce qui laissait croire que le système existait. Il n'existait pas.

Il vit désormais ici, où il a sa place, et **concerne tous les arrivants** :

- **Délai pour accepter le règlement** — désactivé par défaut. Rien ne change tant qu'on ne l'active pas.
- **Délai laissé pour accepter** — trois jours par défaut, entre une heure et trente jours. Assez long pour couvrir un week-end sans connexion.
- **Relance avant expulsion** — un message privé part le nombre de secondes indiqué avant l'échéance (un jour par défaut). À zéro, aucune relance. Le message dit **quand** le délai expire, à l'heure locale de chacun.
- **Expulser à l'expiration** — désactivable. Le délai ne sert alors qu'à relancer.

L'échéance est **figée à l'arrivée** : rallonger le réglage ne raccourcit pas le sursis de ceux qui attendent déjà, et le raccourcir n'expulse pas d'un coup toute la file.

Elle est levée dès que le membre accepte — par le bouton simple comme par le formulaire de vérification d'âge — et dès qu'il quitte le serveur de lui-même.

Avant d'être retirée, la personne reçoit un message privé qui explique pourquoi, et précise que **ce n'est pas un bannissement** : elle peut revenir avec une nouvelle invitation. Des messages privés fermés n'empêchent pas l'expulsion, ils la rendent seulement muette.

### Ce que ce délai n'est pas

Ce n'est pas un dispositif anti-raid. Un raid se traite en secondes et se solde par une expulsion massive ; quelqu'un qui tarde à cliquer mérite des jours et une relance. Les deux systèmes sont séparés, avec chacun leur table, leur rôle Discord et leur message — et le sas de vérification des comptes suspects continue de vivre dans [securite.md](securite.md).

## Les conditions

- **Permissions :** la création d'annonces, d'embeds et de panneaux nécessite des droits administrateur ou des rôles configurés (ex: `@Animateur`).
- **Dépendances :** les fonctionnalités requièrent l'activation préalable du composant correspondant dans l'onglet Configuration (ex: `levels-bot`, `welcome-bot`).
- **Contexte :** toute publication ou attribution de rôle doit désigner un identifiant Discord (ID de salon ou ID de rôle) valide et existant sur le serveur.
- **Confidentialité :** pour les confessions, le nom de l'auteur original est masqué de la base de données lors de la publication publique pour garantir l'anonymat, conformément aux règles du serveur.

## Résultat attendu

Chaque contenu doit être publié au bon endroit et chaque action doit laisser un état compréhensible : ouvert, publié, programmé, attribué, expiré ou clôturé.

