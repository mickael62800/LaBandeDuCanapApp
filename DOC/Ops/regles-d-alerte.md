# Règles d'alerte

Cette fonctionnalité définit les seuils à partir desquels Ops doit prévenir les administrateurs.

## Comment ça marche

Ce module permet de surveiller la plateforme de manière proactive. `platform-api` (ou un service de monitoring dédié) évalue périodiquement les métriques remontées par le `docker-agent` (CPU, RAM, espace disque, conteneurs arrêtés) par rapport à des seuils définis en base de données. Si un seuil est franchi pendant une durée spécifiée (pour éviter les faux positifs liés aux pics temporaires), une alerte est générée et poussée sur un canal Discord défini (via un Webhook) pour réveiller les administrateurs.

## Les actions des utilisateurs

- **Administrateurs système :** configurer de nouvelles règles d'alerte, ajuster les seuils de sensibilité, définir le salon de destination des notifications (Webhook), consulter l'historique des alertes déclenchées.
- **Membres / Modérateurs :** accès strictement interdit.

## Les options

- **Critères d'alerte :** CPU > X%, RAM > Y%, Disque > Z%, Conteneur spécifique arrêté, Taux d'erreur API excessif, Certificat SSL bientôt expiré.
- **Durée (Sustain) :** temps pendant lequel la condition doit être vraie avant de déclencher l'alerte (ex: CPU > 90% pendant 5 minutes).
- **Canaux de diffusion :** URL de Webhook Discord pour recevoir une notification `ping`.

## Les conditions

- **Équilibre :** une règle mal réglée (trop sensible ou sans durée de sustain) peut produire du "spam d'alertes", rendant les administrateurs insensibles (alerte fatigue) et masquant les vrais problèmes.
- **Connectivité :** si le serveur perd sa connexion internet ou crashe complètement, il ne pourra pas envoyer l'alerte Discord. Une supervision externe (ex: UptimeRobot) est recommandée en complément.
- **Habilitation :** accessible uniquement au rôle technique "Superadmin" ou "Ops".

## Résultat attendu

Une règle active surveille silencieusement le système et déclenche une alerte (message Discord ou notification) uniquement lorsque sa condition est atteinte. L'alerte doit identifier clairement la ressource concernée, la raison du déclenchement, et l'heure exacte.

