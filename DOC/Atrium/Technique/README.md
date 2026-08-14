# Documentation technique Atrium

Cette documentation décrit les contrats techniques réellement exposés par Atrium. Elle complète [la documentation fonctionnelle](../README.md).

## Documents

1. [Architecture et flux](01-architecture.md)
2. [API HTTP d'administration](02-api-http.md)
3. [Contrats gRPC et bot Discord](03-grpc-et-bot.md)
4. [Configuration et variables](04-configuration.md)
5. [RAG, mémoire et rétention](05-rag-memoire.md)
6. [Erreurs, sécurité et exploitation](06-erreurs-securite.md)

## Source de vérité

Les routes sont déclarées dans `platform-api/src` et `platform-api/src/atrium/admin.rs`. Les messages gRPC sont définis dans `platform-proto`. Les règles métier de réponse sont dans `platform-core/src/atrium`.



