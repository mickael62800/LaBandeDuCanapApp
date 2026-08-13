# 2. API HTTP et authentification

## Bases

- API principale : port `NEXUS_API_PORT`, par défaut `3100`.
- Santé : `GET /health`.
- Métriques : `GET /metrics`.
- Routes privées : préfixe `/api/`.
- Routes publiques : préfixe `/api/public/`.

Les routes privées utilisent un Bearer basé sur `NEXUS_API_KEY`. La passerelle web ajoute le secret côté serveur ; il ne doit jamais être placé dans le SPA.

## Familles de routes

- `/api/games/{guild_id}/...` : catalogue et serveurs d'une guilde.
- `/api/games/servers/{server_id}/...` : détail, logs, statistiques, configuration et sessions.
- `/api/wallet/...` : portefeuilles et transferts.
- `/api/wheel/...` : tirage et cases de la roue.
- `/api/coussin/...` : profils, combats, inventaire et actions du jeu.
- `/api/grand-salon/...` : membres, motions, cercles, dossiers et Gazette.
- `/api/config/{guild_id}/{bot_name}` : configuration d'un module.
- `/api/bots/definitions` : définitions et schémas des modules.

## Règles de requête

Les identifiants de guilde, membre et serveur doivent correspondre au contexte autorisé. La guilde configurée peut être imposée par le verrou `single_guild`. Une requête acceptée ne signifie pas que l'opération métier a réussi : lire le corps de réponse.
