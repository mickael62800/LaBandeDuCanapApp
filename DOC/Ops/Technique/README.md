# Documentation technique Ops

Cette documentation décrit les contrats techniques d'Ops. Elle complète [la documentation fonctionnelle](../README.md).

## Documents

1. [Architecture et flux](01-architecture.md)
2. [API HTTP et authentification](02-api-http.md)
3. [Machine, santé et Docker](03-machine-et-docker.md)
4. [Sécurité de l'hôte](04-securite.md)
5. [Alertes et worker](05-alertes.md)
6. [Logs et audit](06-logs-audit.md)
7. [Configuration et erreurs](07-configuration-erreurs.md)

## Source de vérité

Les routes sont définies dans `platform-api/src`. Les règles métier sont dans `platform-core/src/ops`. Le dashboard utilise `web/src/api/opsHttp.ts` et les services Ops.



