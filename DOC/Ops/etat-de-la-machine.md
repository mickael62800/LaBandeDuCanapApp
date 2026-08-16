# État de la machine

Cette fonctionnalité donne une vue de santé de la machine qui héberge les services.

## Comment ça marche

L'agent `docker-agent` s'exécute directement sur le serveur physique (ou la VM) qui héberge la plateforme. Il collecte périodiquement les métriques matérielles (utilisation CPU, RAM, espace disque, I/O réseau) et l'état des services clés (bases de données, conteneurs Docker en cours d'exécution). Ces données sont transmises à `platform-api` qui les consolide et les expose au dashboard web. Le rafraîchissement se fait en temps quasi-réel via polling ou WebSockets.

## Les actions des utilisateurs

- **Administrateurs système :** consulter cette page pour vérifier que le serveur n'est pas surchargé, repérer un manque d'espace disque imminent, ou confirmer que les conteneurs cruciaux (PostgreSQL, Redis, bots) tournent correctement.
- **Membres / Modérateurs :** accès strictement interdit.

## Les options

- **Actualisation :** un bouton ou un interrupteur permet de passer du mode de rafraîchissement manuel au mode d'actualisation automatique (ex: toutes les 5 secondes).
- **Vues détaillées :** affichage des métriques sous forme de jauges (pourcentages de charge) et listes de processus/conteneurs.

## Les conditions

- **Habilitation :** seuls les utilisateurs ayant le rôle technique "Superadmin" ou "Ops" peuvent accéder à ce module.
- **Indépendance :** si le `docker-agent` s'arrête de fonctionner ou perd sa connexion, l'état de la machine ne peut plus être remonté (l'interface affichera une erreur de communication).
- **Conséquences métier :** une surcharge disque ou RAM (ex: 99% d'utilisation) repérée ici explique généralement des lenteurs ou des pannes sur les bots (Sentinel/Nexus) et les modèles IA.

## Résultat attendu

La page doit présenter un état lisible et récent de l'infrastructure sous-jacente. Une anomalie (rouge) doit être traitée comme un signal de supervision (besoin d'intervention Ops), pas comme une preuve automatique de panne métier, bien qu'elle en soit souvent la cause.

