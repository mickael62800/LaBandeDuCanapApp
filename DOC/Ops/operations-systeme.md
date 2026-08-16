# Opérations système

Cette fonctionnalité regroupe les opérations techniques liées aux services de la machine, au cache Redis, aux modèles IA et à la gestion des sauvegardes de la base de données.

## Informations clés pour une IA

- **Utilisateur principal :** administrateur technique.
- **Objets suivis :** modèles IA (statut et rechargement à chaud), cache Redis (statistiques et hit rate), sauvegardes BDD (export JSON / dump SQL) et réinitialisation de l'état du cluster PostgreSQL.
- **But principal :** vérifier la disponibilité des composants de la plateforme et gérer les opérations de maintenance système et de sauvegarde.
- **Modèles IA :** leur état indique si les fonctions qui utilisent l'intelligence artificielle (DistilBERT / EfficientNet) sont chargées et opérationnelles.
- **Cache Redis :** son état concerne la disponibilité des données temporaires et l'efficacité du cache (`hit_rate`).
- **Sauvegarde & Restauration BDD :** permet l'export instantané au format JSON enrichi d'un backup de la base système (`/api/system/info`) ou l'exécution de commandes de dump direct Docker (`pg_dump`), ainsi que le rechargement de l'état du cluster.
- **Prudence :** une opération système ou un rechargement de base peut affecter plusieurs applications en même temps.

## Résultat attendu

L'écran doit permettre de distinguer un composant opérationnel, dégradé ou indisponible, d'exporter/recharger la base de données système et d'orienter le diagnostic vers le bon service.
