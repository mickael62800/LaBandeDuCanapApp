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

## L'onglet Commandes

La console libre suppose de connaître par cœur la syntaxe de chaque jeu : Palworld bannit avec `BanPlayer`, Minecraft avec `ban`, 7 Days to Die avec `ban add`. Retenir trois syntaxes, c'est se tromper au moment où l'on est pressé.

L'onglet **Commandes** propose donc les gestes du jeu sous forme de fiches : annoncer un message, expulser, bannir, sauvegarder, arrêter proprement. Chaque fiche décrit ce qu'elle fait, ses paramètres, et ce qu'elle casse le cas échéant.

En tête d'onglet, la **liste des joueurs connectés** est lue en direct sur le serveur de jeu. Chaque joueur y porte ses actions : expulser ou bannir se fait d'un clic, sans recopier un identifiant Steam à la main — une saisie manuelle d'identifiant est une faute qui attend son heure. Ailleurs dans la page, un champ « joueur » se choisit toujours dans cette même liste.

Les gestes irréversibles (bannissement, arrêt du serveur) demandent une confirmation qui annonce ce qui va se passer, et se distinguent visuellement des autres.

### Ce qui rend l'ensemble sûr

Le catalogue vit en base, sur le modèle de jeu (`game_templates.command_schema`), au même titre que le schéma de configuration. Ajouter une commande, ou couvrir un nouveau jeu, se fait par migration sans toucher au front.

**Le navigateur n'envoie jamais de commande.** Il envoie une *clé* et des paramètres ; le serveur retrouve le gabarit — qui ne quitte jamais l'API — valide chaque valeur et compose la commande lui-même. Sans cette règle, un bouton « bannir » serait une console RCON ouverte à quiconque sait forger une requête.

La validation refuse notamment tout caractère de contrôle : un retour à la ligne dans un message d'annonce ferait lire **deux** commandes au serveur de jeu là où l'administrateur en a demandé une. Une clé de commande absente du catalogue est refusée, jamais interprétée.

Ce lot couvre **Palworld**. Les autres jeux gardent leur console libre jusqu'à ce que leur catalogue soit écrit : mieux vaut un jeu dont chaque commande a été vérifiée que sept jeux approximatifs.

## Les conditions

- **Infrastructure :** nécessite que le service `docker-agent` soit fonctionnel sur la machine hébergeant les jeux, et qu'il puisse communiquer avec `platform-api`.
- **Ressources système :** la création d'un serveur de jeu consomme des ressources réelles (RAM, CPU, Disque) sur l'hôte. Une surveillance est nécessaire pour éviter la surcharge.
- **Sécurité :** les mots de passe serveur (RCON/Admin) sont masqués ou protégés et ne doivent jamais être exposés dans les logs Discord.

## Résultat attendu

Après une action (comme "Démarrer"), l'interface doit indiquer clairement le nouvel état du conteneur Docker. Une création réussie produit un serveur visible dans la liste des serveurs de la communauté, prêt à accueillir des joueurs à l'adresse indiquée.

