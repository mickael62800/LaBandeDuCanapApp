# 1. Architecture et flux

## Composants

- `platform-api` : API HTTP, authentification, persistance et appels Discord.
- `platform-core::sentinel` : règles métier de modération, communauté et système.
- `sentinel-bot` : commandes et événements Discord.
- `sentinel-worker` : tâches périodiques, expirations, rappels et nettoyage.
- PostgreSQL : données métier, historiques, tickets et configuration.
- Redis : événements, cache, files et diffusion temps réel.
- web : dashboard d'administration.

## Flux d'une action

1. Le dashboard ou le bot identifie la guilde et l'utilisateur.
2. L'API authentifie l'appel et vérifie les droits.
3. Le domaine valide la règle métier.
4. La base enregistre le résultat.
5. L'adaptateur Discord applique l'action si nécessaire.
6. L'audit conserve la trace.

Le bot ne doit pas accéder directement à PostgreSQL. Les plateformes NEXUS et Atrium utilisent leurs APIs propres.


