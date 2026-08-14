# 4. Sécurité de l'hôte

## Données exposées

- adresses IP bannies et jails ;
- IP les plus actives ;
- échecs d'authentification ;
- tendance du trafic ;
- certificat TLS et erreurs TLS ;
- événements d'audit et derniers accès.

## Actions

La surface de sécurité peut débannir une IP, consulter les événements et lancer une purge ciblée des journaux selon les options choisies.

## Règles de conservation

La purge des logs API, logs d'audit et bans manuels est une opération destructive. Elle doit être explicitement demandée, bornée par une durée et journalisée.

Une IP bannie ne doit pas entraîner automatiquement la suppression de ses preuves : la rétention est gérée séparément.

