# 6. Workers, jobs et événements

## Jobs serveurs de jeu

Les jobs internes peuvent contrôler la santé, arrêter les serveurs inactifs, réconcilier l'état de la base avec le runtime, nettoyer les images, révéler des IP selon le planning, envoyer un ping quotidien et démarrer automatiquement certains serveurs.

## Jeux mentionnables : consolidation

Le job `mention-sync` demande à chaque guilde possédant des jeux son inventaire Discord. Il ne répare rien : il publie `games_sync_requested`, le bot dépose sa photographie (rôles, messages de panneau vivants, salons illisibles) sur `PUT /api/games/{guild_id}/sync/inventory`, et le rapport de divergence se recalcule à la lecture.

La comparaison est un calcul pur du domaine (`domain::entities::casino::game_sync`) : le bot ne voit que Discord, l'API ne voit que la base, aucun des deux ne peut constater seul un désaccord. La direction de réparation — Discord ou le dashboard fait foi — est toujours choisie par un humain (`application::game_sync_service`).

En complément du passage périodique, le bot signale immédiatement toute disparition de rôle (`guild_role_delete` → `DELETE /api/games/{guild_id}/sync/roles/{role_id}`), ce qui coupe la liaison morte avant qu'elle ne fasse échouer des abonnements. Le jeu, lui, n'est jamais supprimé automatiquement.

## Routes internes

Elles utilisent le préfixe `/api/games/internal/jobs/` et ne sont pas destinées au dashboard public. Elles doivent être appelées par le worker autorisé.

## Événements

NEXUS utilise le flux Redis `nexus:events`, séparé de Sentinel et Atrium. Un événement NEXUS ne doit pas être publié sur le flux d'une autre plateforme.

## Reprise

Les jobs doivent être idempotents autant que possible : une relance après un timeout ne doit pas créer deux serveurs, deux récompenses ou deux sessions.
