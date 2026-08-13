# 6. Configuration, structure et sauvegardes

## Configuration

- `GET /api/bots/definitions` : définitions des bots.
- `/api/bots/config/...` ou les routes de composants : configuration par guilde.
- `/api/rules` : règles de scoring.

Une clé absente doit être interprétée comme désactivée pour les modules concernés. Une configuration d'une guilde ne doit pas être appliquée à une autre.

## Structure Discord

Le constructeur manipule catégories, salons et rôles. Vérifier les identifiants et les collisions avant d'appliquer une structure.

## Sauvegardes

- liste et détail de snapshots ;
- capture asynchrone ;
- renommage et suppression ;
- restauration avec option de nettoyage.

Une restauration peut modifier fortement la guilde. Elle doit être confirmée, auditée et vérifiée après exécution.
