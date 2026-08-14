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

