# États et transitions

## Serveur de jeu NEXUS

`création → préparation → en ligne → arrêté`.

Un état `erreur` peut apparaître à chaque étape. Une action incompatible avec l'état actuel doit être refusée.

## Ticket Sentinel

`ouvert → en cours → résolu → clôturé`.

Un ticket peut être escaladé lorsque le délai de traitement est dépassé.

## Motion du Grand Salon

`proposée → en vote → acceptée ou refusée → clôturée`.

Une motion fermée ne doit plus accepter de vote.

## Document Atrium

`actif` ou `inactif`. Seul un document actif peut servir de source à une réponse.

## Alerte Ops

`non déclenchée → déclenchée → notifiée → dédupliquée ou résolue`.

Une alerte répétée ne doit pas produire un flot illimité de notifications.

## Règle générale

Ne pas déduire qu'une action a réussi uniquement parce qu'elle a été demandée. Relire l'état après l'action.

