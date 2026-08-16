# Serveurs de jeu

Cette fonctionnalité permet de créer et de gérer des serveurs de jeux pour la communauté.

## Comment ça marche

Ce module agit comme un panneau de contrôle pour orchestrer des serveurs de jeux vidéo hébergés via des conteneurs Docker (ex: Palworld, Minecraft). Lorsqu'un administrateur demande la création ou le démarrage d'un serveur depuis le dashboard, `platform-api` relaie la commande à `docker-agent` (le composant bas-niveau qui exécute les commandes Docker sur la machine hôte). L'état du serveur (En ligne, Arrêté, Erreur) est mis à jour dynamiquement. Un serveur de jeu NEXUS peut ensuite remonter des événements vers l'API (comme des succès/hauts-faits de joueurs).

## Les actions des utilisateurs

- **Administrateurs :** choisir un jeu dans le catalogue, configurer et créer le serveur, le démarrer/arrêter/redémarrer, consulter l'adresse IP et le port, surveiller la RAM/CPU via les statistiques, lire les logs en direct (RCON/Console), envoyer des commandes système au serveur de jeu.
- **Membres (Joueurs) :** consulter l'état du serveur de jeu depuis Discord si l'administrateur a publié l'information, et utiliser l'IP fournie pour s'y connecter en jeu.

## Les options

- **Cycle de vie :** boutons "Créer", "Démarrer", "Arrêter", "Redémarrer", "Supprimer".
- **Paramètres serveur :** nom, description, mots de passe, règles de jeu spécifiques (rates), selon le jeu choisi.
- **Monitoring :** accès aux journaux d'activité (logs standards et d'erreurs) et aux graphiques de performance.
- **RCON / Commandes :** champ pour injecter des commandes serveur directement (ex: `/broadcast`, `/save`).

## Les conditions

- **Infrastructure :** nécessite que le service `docker-agent` soit fonctionnel sur la machine hébergeant les jeux, et qu'il puisse communiquer avec `platform-api`.
- **Ressources système :** la création d'un serveur de jeu consomme des ressources réelles (RAM, CPU, Disque) sur l'hôte. Une surveillance est nécessaire pour éviter la surcharge.
- **Sécurité :** les mots de passe serveur (RCON/Admin) sont masqués ou protégés et ne doivent jamais être exposés dans les logs Discord.

## Résultat attendu

Après une action (comme "Démarrer"), l'interface doit indiquer clairement le nouvel état du conteneur Docker. Une création réussie produit un serveur visible dans la liste des serveurs de la communauté, prêt à accueillir des joueurs à l'adresse indiquée.

