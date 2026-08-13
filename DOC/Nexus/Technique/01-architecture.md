# 1. Architecture et flux

## Composants

- `nexus-bot` : commandes Discord et appels HTTP vers NEXUS.
- `nexus-api` : API, persistance PostgreSQL et orchestration métier.
- `platform-core::nexus` : entités et règles des jeux, wallets et serveurs.
- `platform-scheduler` : déclenche les tâches périodiques via `nexus-api`.
- runtime de jeux : exécution des serveurs de jeu, en mode Docker ou noop selon la configuration.
- web : tableau de bord via la passerelle `/nexus-api/`.

## Flux d'une action

1. Le dashboard ou le bot identifie la guilde et l'utilisateur.
2. La requête passe par l'API NEXUS et son contrôle d'accès.
3. Le domaine vérifie les droits, l'état et les limites.
4. PostgreSQL persiste le résultat.
5. Pour un serveur de jeu, le runtime applique ensuite l'opération au conteneur.
6. La réponse confirme le nouvel état ou décrit l'erreur.

Le bot n'accède jamais directement à la base de données.
