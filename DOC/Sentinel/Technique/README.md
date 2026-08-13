# Documentation technique Sentinel

Cette documentation décrit les contrats techniques de Sentinel. Elle complète [la documentation fonctionnelle](../Complet/README.md).

## Documents

1. [Architecture et flux](01-architecture.md)
2. [API, authentification et droits](02-api-et-acces.md)
3. [Modération et infractions](03-moderation.md)
4. [AutoMod et revues](04-automod.md)
5. [Communauté et tickets](05-communaute.md)
6. [Configuration, structure et sauvegardes](06-configuration.md)
7. [Dataset IA, logs et workers](07-donnees-et-workers.md)
8. [Erreurs et sécurité](08-erreurs-securite.md)

## Source de vérité

Les routes sont dans `sentinel-api/src/adapters/inbound/http`. Les règles métier sont dans `platform-core/src/sentinel`. Le dashboard utilise `web/src/api/http.ts` et les services de `web/src/services`.
