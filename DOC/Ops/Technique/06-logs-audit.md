# 6. Logs et audit

## Catégories

- logs API ;
- logs bots ;
- logs workers ;
- logs WebSocket ;
- événements d'audit de l'infrastructure ;
- événements de sécurité.

## Différence importante

Un log technique décrit le fonctionnement d'un service. Un audit décrit une action d'administration. Un événement Discord relève de Sentinel et ne doit pas être confondu avec l'audit de l'hôte.

## Diagnostic

Filtrer par plateforme, service, niveau et période. Comparer les logs avec les métriques et les changements de conteneurs. Un log manquant ne prouve pas qu'aucun événement n'a eu lieu.

## Confidentialité

Les logs peuvent contenir des identifiants, adresses IP ou données de requête. Limiter leur accès et éviter de les recopier dans une réponse non sécurisée.
