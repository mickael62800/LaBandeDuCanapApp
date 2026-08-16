# Sécurité Discord

Ce domaine aide à détecter et limiter les menaces qui touchent le serveur Discord.

## Comment ça marche

Le domaine de sécurité s'appuie sur le bot Sentinel (`automod-bot` et `security-bot`) pour analyser en continu les flux de la communauté (messages, pièces jointes, arrivées massives). L'analyse combine des méthodes heuristiques (détection de liens, mots-clés, expressions régulières) et des appels d'inférence IA (analyse de texte via DistilBERT, analyse d'image via EfficientNet) hébergés dans l'écosystème de l'API. Chaque message est scanné et reçoit un score de menace. Selon les seuils configurés, le système prend une action immédiate sur Discord (suppression, mute) ou émet une carte de révision pour examen humain, tracée dans la base PostgreSQL.

## Les actions des utilisateurs

- **Administrateurs :** activer l'AutoMod et la sécurité anti-raid, ajuster la sensibilité (seuils IA et heuristiques), configurer les listes blanches (domaines, rôles ignorés).
- **Modérateurs :** traiter les cartes de révision (approuver la sanction automatique, l'annuler, ou escalader manuellement), analyser les faux positifs pour ajuster la configuration ou alimenter le dataset IA.
- **Membres :** soumis aux règles, ils peuvent voir leurs messages bloqués et être alertés ou mis sous silence en cas de non-respect.

## Les options

- **Filtres heuristiques :** spam, majuscules, détection de liens, blocage d'invitations Discord, phishing, et extensions de fichiers dangereuses.
- **Analyse IA :** activation séparée pour le texte (insultes, harcèlement, menaces) et la vision (contenu illicite, NSFW, violence).
- **Protection anti-raid :** activation de la quarantaine pour les nouveaux comptes suspects, mode lockdown (verrouillage complet).
- **Action de révision (Review Mode) :** configurer le système pour qu'il propose une sanction sous forme de vote/carte à valider par les modérateurs, plutôt que de l'appliquer aveuglément.

## Les conditions

- **Limites de l'IA :** la détection par intelligence artificielle attribue un score de confiance ; elle peut produire des faux positifs (blagues entre amis, contexte particulier). Il est recommandé d'utiliser le mode "Review" lors des premiers réglages.
- **Permissions :** les utilisateurs disposant du rôle "Administrateur" ou inclus dans la liste des "Rôles ignorés" contournent totalement les filtres d'AutoMod.
- **Canaux :** l'AutoMod peut être désactivé sur des canaux spécifiques (ex: `#nsfw` ou `#spam`).

## Résultat attendu

Une alerte doit expliquer clairement ce qui a été détecté, sur quel membre ou message, quand cela s'est produit, le score de confiance de l'IA (le cas échéant) et l'action qui a été appliquée ou qui est recommandée.

