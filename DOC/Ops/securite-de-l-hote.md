# Sécurité de l'hôte

Cette fonctionnalité surveille et protège la machine contre les accès ou comportements suspects.

## Comment ça marche

La sécurité de l'hôte surveille les accès non autorisés et les abus d'utilisation de l'infrastructure web (pas la sécurité de la communauté Discord, mais celle du serveur physique). Les requêtes suspectes (DDoS HTTP, force brute sur les formulaires, erreurs répétées d'OAuth) sont interceptées par le pare-feu applicatif ou l'API (Rate Limiting). Lorsqu'une IP dépasse un certain score de menace, elle est automatiquement bannie (Fail2Ban ou ban interne `platform-api`). Ces événements de sécurité sont stockés dans les `audit_logs` d'Ops.

## Les actions des utilisateurs

- **Administrateurs système :** surveiller les tentatives d'intrusion, consulter la liste des adresses IP actuellement bannies, débannir manuellement une IP légitime qui se serait trompée, ou bannir explicitement une IP malveillante.
- **Membres / Modérateurs :** accès strictement interdit.

## Les options

- **Liste des bans :** affichage de toutes les IP bloquées, la raison du blocage, et la date d'expiration.
- **Nettoyage :** possibilité de vider certains journaux de sécurité (audit logs) pour libérer de la place, sous réserve de droits élevés.
- **Événements de sécurité :** filtrage spécifique des logs pour isoler uniquement les problèmes de sécurité (tentatives de connexion refusées, abus d'API).

## Les conditions

- **Séparation des domaines :** il est crucial de ne pas confondre "Sécurité de l'hôte" (attaques web, DDoS, piratage du dashboard) et "Sécurité Sentinel" (insultes sur Discord, spam de messages). Ici, on protège le serveur.
- **Sensibilité :** débannir une IP est une action sensible. Cela permet potentiellement à un attaquant de reprendre son activité malveillante. Le débannissement doit toujours être enregistré dans l'historique d'audit.
- **Habilitation :** accessible uniquement au rôle technique "Superadmin".

## Résultat attendu

La page doit montrer un tableau de bord clair des menaces d'infrastructure et des protections actives. Les IP bloquées ne peuvent plus faire de requêtes vers le dashboard ou l'API, garantissant la stabilité du service.

