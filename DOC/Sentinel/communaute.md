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

## Les conditions

- **Permissions :** la création d'annonces, d'embeds et de panneaux nécessite des droits administrateur ou des rôles configurés (ex: `@Animateur`).
- **Dépendances :** les fonctionnalités requièrent l'activation préalable du composant correspondant dans l'onglet Configuration (ex: `levels-bot`, `welcome-bot`).
- **Contexte :** toute publication ou attribution de rôle doit désigner un identifiant Discord (ID de salon ou ID de rôle) valide et existant sur le serveur.
- **Confidentialité :** pour les confessions, le nom de l'auteur original est masqué de la base de données lors de la publication publique pour garantir l'anonymat, conformément aux règles du serveur.

## Résultat attendu

Chaque contenu doit être publié au bon endroit et chaque action doit laisser un état compréhensible : ouvert, publié, programmé, attribué, expiré ou clôturé.

