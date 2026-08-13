# 5. Configuration et jeux mentionnables

## Configuration des modules

- `GET /api/bots/definitions` : schémas des modules.
- `GET/PUT /api/config/{guild_id}/{bot_name}` : lire ou modifier la configuration.

La configuration est par guilde. Une clé absente doit être interprétée comme désactivée selon le comportement fail-closed de NEXUS. Les valeurs vides peuvent représenter une suppression logique.

## Jeux mentionnables

- `GET/POST /api/games/{guild_id}` : lister ou créer.
- `DELETE /api/games/{guild_id}/{game_id}` : supprimer.
- `POST /api/games/{guild_id}/detect-mentions` : détecter des mentions.
- `GET /api/games/{guild_id}/panels` : lister les panneaux.
- `POST /api/games/{guild_id}/panel/deploy` : publier un panneau.
- `POST /api/games/{guild_id}/upload-emoji` : préparer un emoji.

Un jeu doit avoir un nom. Un panneau est lié à une guilde et à un salon Discord. La suppression retire le jeu des propositions futures.
