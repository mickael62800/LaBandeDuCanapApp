# Statistiques et modération

Ce domaine permet de surveiller l'activité du serveur et de traiter les comportements qui ne respectent pas ses règles.

## Comment ça marche

Ce domaine centralise la surveillance et la discipline sur le serveur Discord. L'API `platform-api` collecte en temps réel les événements d'activité via le bot et les stocke en base de données pour générer des statistiques. Lorsqu'un modérateur déclenche une sanction (via commande Discord comme `/ban` ou via le dashboard web), l'API vérifie les permissions, applique l'action sur Discord via l'adaptateur, puis enregistre l'infraction dans l'historique (PostgreSQL). `platform-scheduler` gère ensuite les expirations (ex: levée d'un mute temporaire après X jours).

## Les actions des utilisateurs

- **Administrateurs :** configurer le barème des règles et de gravité, définir les permissions des rôles de modération, consulter les statistiques globales du serveur.
- **Modérateurs :** consulter la fiche d'un membre (profil, historique des pseudos, infractions passées), appliquer une sanction (avertissement, mute, kick, ban), lever une sanction existante.
- **Membres :** peuvent (selon la configuration) recevoir un message privé expliquant leur sanction et faire appel de la décision via un ticket ou une commande dédiée.

## Les options

- **Sanctions :** avertissement (warn) silencieux ou public, réduction au silence (mute) avec durée définie, expulsion (kick), bannissement (ban) temporaire ou définitif.
- **Barème et règles :** définition du poids (points de strike) de chaque type d'infraction, automatisation de l'escalade (ex: 3 warns = 1 mute automatique).
- **Historique :** conservation de l'historique complet, avec ou sans option de péremption des points (ex: un strike expire après 6 mois).

## Les conditions

- **Permissions :** l'exécution d'une sanction nécessite des droits explicites (permission Discord native ou configuration de rôle de modérateur). 
- **Hiérarchie Discord :** le bot (et par extension le modérateur via le dashboard) ne peut sanctionner qu'un membre ayant un rôle inférieur au sien dans la hiérarchie des rôles Discord. Les propriétaires de serveur sont immunisés.
- **Traçabilité :** chaque action manuelle doit être justifiée. Une raison est requise ou fortement recommandée pour la traçabilité.
- **Objectivité :** les statistiques fournissent un contexte, mais ne remplacent pas le discernement humain.

## Résultat attendu

Une action doit indiquer clairement le membre concerné, la raison, la durée éventuelle et le résultat obtenu. Les décisions importantes doivent rester consultables dans l'historique et appliquées fidèlement sur le serveur Discord.

