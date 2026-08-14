# 1. Architecture et flux

## Composants

- `platform-api` : API de supervision, sécurité, Docker, logs et alertes.
- `platform-core::ops` : sondes, contrats de conteneurs, sécurité hôte et règles métier indépendantes.
- `ops-agent` : collecte hôte, surveillance Docker et monitoring des services.
- `platform-scheduler` : déclenche l'évaluation des alertes dans `platform-api`.
- `docker-agent` : accès contrôlé aux opérations Docker.
- PostgreSQL : règles d'alerte, journaux et audit.
- Redis : état de supervision et changements de conteneurs.

## Flux de supervision

1. Les sondes et l'agent Docker collectent des mesures.
2. Ops API expose l'état au dashboard.
3. Ops worker compare les mesures aux règles d'alerte.
4. Une condition atteinte produit une notification, avec déduplication.
5. Les actions d'administration sont enregistrées dans l'audit d'infrastructure.

Ops peut fonctionner même si une sonde secondaire est indisponible ; il faut alors signaler la donnée comme indisponible plutôt que la considérer normale.



