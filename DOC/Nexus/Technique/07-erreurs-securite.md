# 7. Erreurs, limites et sécurité

## Erreurs à distinguer

- accès refusé : authentification ou autorisation absente ;
- identifiant invalide : guilde, serveur ou utilisateur incorrect ;
- état incompatible : serveur déjà démarré, arrêté ou en transition ;
- limite atteinte : mémoire, nombre de serveurs, cooldown ou solde ;
- runtime indisponible : l'API répond mais Docker ne peut pas appliquer l'action ;
- base indisponible : la persistance ne peut pas être confirmée.

## Limites HTTP

`NEXUS_MAX_BODY_SIZE` limite la taille des requêtes. Les lectures utilisent `NEXUS_RATE_LIMIT_PER_SEC` et les opérations lourdes, comme celles qui lancent un conteneur, utilisent `NEXUS_HEAVY_RATE_LIMIT_PER_SEC`.

## Sécurité

Les clés API et tokens Docker ne doivent jamais apparaître dans le frontend ou les logs. Les opérations de runtime doivent utiliser le token de surface de jeu dédié. Les IP, commandes console, sessions et wallets sont des données sensibles.

## Règle pour une IA

Ne jamais annoncer une action comme réussie sans lire la réponse métier. En cas d'erreur, conserver l'état précédent comme hypothèse et demander une nouvelle lecture avant de réessayer.
