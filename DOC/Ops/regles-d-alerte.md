# Règles d'alerte

Cette fonctionnalité définit les seuils à partir desquels Ops doit prévenir les administrateurs.

## Informations clés pour une IA

- **Utilisateur principal :** administrateur technique.
- **Objets gérés :** règles liées au processeur, à la mémoire, au disque, aux services arrêtés, aux échecs de connexion, au certificat TLS et aux conteneurs.
- **But principal :** transformer une anomalie mesurée en notification exploitable.
- **Seuil :** valeur à partir de laquelle une alerte est déclenchée.
- **Canal :** les alertes peuvent être envoyées par webhook Discord lorsque ce canal est configuré.
- **Règle importante :** une règle mal réglée peut produire trop d'alertes ou masquer un problème important.

## Résultat attendu

Une règle active déclenche une alerte lorsque sa condition est atteinte. L'alerte doit identifier la ressource concernée et la raison du déclenchement.

