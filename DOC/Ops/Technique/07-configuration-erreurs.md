# 7. Configuration et erreurs

## Variables principales

- `OPS_API_BIND_ADDR` : adresse de l'API.
- `OPS_API_TOKEN` : token obligatoire, avec longueur minimale.
- `OPS_DATABASE_URL` : base Ops obligatoire.
- `OPS_METRICS_TOKEN` : protection facultative des métriques.
- `DOCKER_AGENT_URL` : adresse de l'agent Docker.
- `DOCKER_AGENT_TOKEN` : token de l'agent Docker.
- `OPS_API_RATE_LIMIT_PER_SEC` : limite de débit de l'API.

## Erreurs à distinguer

- accès refusé : token ou droits incorrects ;
- ressource inconnue : conteneur, règle ou événement absent ;
- dépendance indisponible : base, Redis, agent Docker ou sonde ;
- action refusée : état incompatible ou opération dangereuse ;
- limite atteinte : débit ou taille de requête.

## Règle pour une IA

Ne pas confondre `/health` et `/ready` : une API peut répondre tout en n'étant pas prête pour toutes ses dépendances. Ne jamais redémarrer ou supprimer un composant sans identifier ses dépendants et conserver les logs utiles.

