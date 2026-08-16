# Opérations système

Cette fonctionnalité regroupe les opérations techniques liées aux services de la machine, au cache Redis, aux modèles IA et à la gestion des sauvegardes de la base de données.

## Comment ça marche

Cette page regroupe des actions de maintenance lourde et de supervision des composants sous-jacents qui font tourner l'infrastructure. Elle s'interface avec `platform-api` via des endpoints dédiés (`/api/system/info`, `/api/cache/stats`, `/api/models/*`) pour interroger en direct le statut du cache Redis, des modèles de Machine Learning chargés en mémoire (DistilBERT/EfficientNet pour la sécurité), et l'état du cluster PostgreSQL. Ces endpoints permettent aussi de déclencher des opérations de sauvegarde asynchrones.

## Les actions des utilisateurs

- **Administrateurs système :** 
  - Consulter l'état et l'efficacité (hit rate) du cache Redis.
  - Vérifier que les modèles IA sont correctement chargés en mémoire et, si besoin, forcer leur rechargement à chaud.
  - Exporter/télécharger un backup JSON de l'état système ou lancer un dump SQL complet.
  - (Action critique) Déclencher le rechargement de l'état de la base de données.

## Les options

- **Supervision Modèles IA :** vue sur le statut de chaque modèle (chargé, erreur, non initialisé) et bouton de rechargement.
- **Supervision Cache :** affichage des clés actives, du taux de réussite (hit rate) et de la mémoire utilisée.
- **Sauvegarde BDD :** boutons pour lancer une extraction (`export JSON`) via `/api/system/info` ou un backup direct via `docker-agent`.

## Les conditions

- **Impact critique :** un rechargement à chaud des modèles IA ou de la configuration de la base de données peut provoquer des micro-coupures de service.
- **Sécurité des données :** l'export d'une base de données produit un fichier très sensible contenant l'intégralité des données de la plateforme. Son accès est protégé par un Token d'API strict (`OPS_API_TOKEN`).
- **Permissions :** accessible uniquement au rôle "Superadmin" ou "Ops".

## Résultat attendu

L'écran doit permettre d'identifier immédiatement si un composant d'infrastructure (Modèle IA, Redis, Postgres) est dégradé. Les actions de maintenance (rechargement, sauvegarde) doivent retourner un succès ou un log d'erreur clair pour guider le dépannage technique.
