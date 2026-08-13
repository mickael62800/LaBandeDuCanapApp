# 5. Alertes et worker

## Fonctionnement

Le worker charge les règles, collecte les métriques puis évalue chaque condition. Les métriques peuvent inclure l'état des services, les échecs d'authentification, l'expiration TLS et les changements de conteneurs.

## Types de règles

- seuil CPU, mémoire ou disque ;
- service hors ligne ;
- échecs d'authentification supérieurs au seuil ;
- certificat TLS proche de l'expiration ;
- conteneur démarré, arrêté ou supprimé.

## Déduplication

Une même alerte n'est pas renvoyée indéfiniment pour la même clé. Redis conserve l'état nécessaire à cette déduplication. Si Redis est indisponible, le système doit signaler que la déduplication est dégradée.

## Notification

Les alertes sont envoyées via le canal configuré, notamment un webhook Discord. Le message doit préciser la règle, la ressource, la valeur observée et le seuil.
