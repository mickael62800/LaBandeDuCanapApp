# Documentation technique NEXUS

Cette documentation décrit les contrats techniques de NEXUS. Elle complète [la documentation fonctionnelle](../README.md).

## Documents

1. [Architecture et flux](01-architecture.md)
2. [API HTTP et authentification](02-api-http.md)
3. [Serveurs de jeu et runtime](03-serveurs-de-jeu.md)
4. [Economie, roue et Coussin](04-jeux-et-economie.md)
5. [Configuration et jeux mentionnables](05-configuration.md)
6. [Workers, jobs et événements](06-workers.md)
7. [Erreurs, limites et sécurité](07-erreurs-securite.md)

## Source de vérité

Les routes sont définies dans `platform-api/src/mod.rs`. Les règles métier sont dans `platform-core/src/nexus`. Le client web utilise `web/src/api/nexusHttp.ts` et les services `web/src/services/nexus*.ts`.



