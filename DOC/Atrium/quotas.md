# Suivi de la consommation et des quotas

Cette fonctionnalité permet de suivre l'utilisation d'Atrium et les limites qui s'appliquent aux demandes.

## Comment ça marche

Le suivi des quotas protège l'infrastructure et la facturation contre les abus d'utilisation de l'IA (génération de texte/LLM). Chaque fois qu'un utilisateur envoie un prompt à Atrium, `platform-api` comptabilise la requête et les tokens estimés/réels dans PostgreSQL (et potentiellement Redis pour le rate-limiting). Le dashboard web récupère ces données pour fournir un monitoring en temps réel de la consommation. Si un quota (serveur ou utilisateur) est dépassé, l'API refuse de servir de nouvelles requêtes d'inférence jusqu'à la réinitialisation (souvent quotidienne).

## Les actions des utilisateurs

- **Administrateurs :** consulter le dashboard pour auditer l'utilisation d'Atrium, voir si le serveur approche de son plafond quotidien, identifier les utilisateurs les plus actifs (ou ceux qui spammeraient l'IA).
- **Membres :** ne voient pas cette page, mais subissent les limites. Si un membre spamme Atrium, il recevra un message d'erreur de la part du bot lui indiquant qu'il doit ralentir.

## Les options

- **Consommation globale vs locale :** affichage de la consommation totale du projet par rapport à la part spécifique du serveur sélectionné.
- **Limites configurables (par le propriétaire) :** bien que souvent affichées en lecture seule pour les administrateurs standards, les quotas maximums (requêtes/jour/serveur et requêtes/jour/utilisateur) sont paramétrés au niveau de l'infrastructure globale.
- **Délai (Rate Limit) :** paramétrage du délai minimal entre deux questions posées par une même personne (ex: 5 secondes) pour éviter les requêtes en rafale.

## Les conditions

- **Réinitialisation :** les compteurs de quotas quotidiens (daily) sont réinitialisés à zéro automatiquement chaque jour à minuit (UTC) par le `platform-scheduler` ou lors d'une vérification à la volée.
- **Sécurité anti-DDoS :** ces limites sont cruciales pour éviter qu'un membre malveillant ne génère des coûts importants d'API LLM externes.
- **Rôle requis :** seul un administrateur du serveur (ou le superadmin de la plateforme) peut consulter les métriques de consommation.

## Résultat attendu

La page doit permettre de distinguer clairement la consommation du serveur de la consommation globale, et d'identifier rapidement par des jauges ou des alertes visuelles si une limite est sur le point d'être franchie, bloquant temporairement Atrium.

