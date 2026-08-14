# 3. Serveurs de jeu et runtime

## Routes principales

- `GET /api/games/{guild_id}/servers` : liste des serveurs.
- `GET /api/games/{guild_id}/templates` : modèles autorisés.
- `GET /api/games/servers/{server_id}` : détail.
- `POST /api/games/servers/{server_id}/start` : démarrer.
- `POST /api/games/servers/{server_id}/stop` : arrêter.
- `POST /api/games/servers/{server_id}/restart` : redémarrer.
- `GET /api/games/servers/{server_id}/logs` : logs.
- `GET /api/games/servers/{server_id}/stats` : statistiques.
- `PUT /api/games/servers/{server_id}/config` : configuration.
- `GET /api/games/servers/{server_id}/sessions` : sessions et joueurs.
- `POST /api/games/servers/{server_id}/command` : commande console.

## États et limites

Le serveur possède un état de cycle de vie. Les transitions incompatibles doivent être refusées. La création vérifie le nombre maximal de serveurs, la mémoire et les paramètres du modèle.

## Runtime

`NEXUS_GAME_RUNTIME=docker` utilise le runtime Docker délégué. `noop` laisse les opérations de conteneur sans exécution réelle ; le listing et la configuration peuvent rester disponibles. Toujours vérifier le runtime avant de promettre qu'un serveur est en ligne.

## Données sensibles

L'adresse IP et les accès peuvent être révélés par des routes dédiées et planifiées. Ne pas les exposer avant confirmation de la règle de révélation.

Trois chemins de révélation coexistent :

- **Bouton « Révéler l'adresse IP » du panneau d'inscription Discord** (`POST /reveal-ip/request`, propriétaire du serveur) : démarre le conteneur s'il est à l'arrêt, annonce l'ouverture dans le panneau, puis **programme** la révélation à `now + reveal_delay_minutes` (config game-portal, défaut 10 min). Le worker `reveal-ip` publie l'adresse dans le **salon privé des inscrits** à l'échéance, une fois le serveur `running`. Échoue en fermeture si l'hôte public n'est pas configuré.
- **« Révéler maintenant » (admin web, `POST /reveal-ip`)** : révélation immédiate forcée, exige un serveur déjà `running`.
- **Programmation (`/schedule`, `/reveal-schedule`)** : ouverture différée, le worker démarre le conteneur ~5 min avant l'heure.

