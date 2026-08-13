# 6. Workers, jobs et événements

## Jobs serveurs de jeu

Les jobs internes peuvent contrôler la santé, arrêter les serveurs inactifs, réconcilier l'état de la base avec le runtime, nettoyer les images, révéler des IP selon le planning, envoyer un ping quotidien et démarrer automatiquement certains serveurs.

## Routes internes

Elles utilisent le préfixe `/api/games/internal/jobs/` et ne sont pas destinées au dashboard public. Elles doivent être appelées par le worker autorisé.

## Événements

NEXUS utilise le flux Redis `nexus:events`, séparé de Sentinel et Atrium. Un événement NEXUS ne doit pas être publié sur le flux d'une autre plateforme.

## Reprise

Les jobs doivent être idempotents autant que possible : une relance après un timeout ne doit pas créer deux serveurs, deux récompenses ou deux sessions.
