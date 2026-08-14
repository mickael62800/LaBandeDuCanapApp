# 7. Dataset IA, logs et workers

## Dataset IA

Le dataset contient des messages destinés à l'entraînement ou à l'évaluation. Les routes permettent de lister avec filtres et de supprimer des éléments sélectionnés. Ces données ne sont pas automatiquement des règles de modération.

## Logs

Sentinel est la plateforme qui expose la réception de logs worker via `POST /api/logs`. Les logs sont catégorisés par service et niveau. Les workers doivent utiliser `SENTINEL_API_KEY` et ne doivent pas envoyer leurs logs vers une autre plateforme.

## Tâches périodiques

Le worker traite notamment les expirations de sanctions, rappels, votes AutoMod, nettoyages, SLA tickets, rôles temporaires et progression vocale. Les tâches doivent être idempotentes et tracer leurs erreurs.

