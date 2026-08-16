# Configuration

La configuration permet d'activer ou de désactiver les modules NEXUS pour chaque serveur Discord et d'adapter leurs réglages.

## Comment ça marche

Ce domaine centralise l'activation et le réglage de l'ensemble des modules d'animation et de jeux de l'écosystème NEXUS sur un serveur Discord. Les paramètres modifiés dans l'interface web sont envoyés via `platform-api` et sauvegardés en base de données. Chaque composant NEXUS (comme la Roue du Destin, l'Économie, les Serveurs de Jeu) lit cette configuration pour adapter son comportement en temps réel, bloquer certaines commandes ou s'activer/se désactiver complètement sur le serveur cible.

## Les actions des utilisateurs

- **Administrateurs :** consulter le catalogue des modules NEXUS disponibles, activer ou désactiver d'un clic les animations pour la communauté, modifier les variables et limites spécifiques de chaque jeu (prix, délais, annonces).
- **Membres :** aucune action directe sur cette page. Les membres subissent l'activation ou la désactivation des fonctionnalités configurées par les administrateurs.

## Les options

- **État global :** un interrupteur principal (`enabled`) par module pour activer ou couper l'animation sur le serveur.
- **Réglages spécifiques :** variables de jeu ajustables (ex: le prix du ticket pour la Roue du Destin, le délai de relance d'un serveur de jeu, le salon de notification pour les Hauts-Faits).
- **Permissions :** définition éventuelle des rôles Discord autorisés à utiliser certaines commandes restreintes du module.

## Les conditions

- **Permissions d'accès :** la modification de ces réglages est strictement réservée aux administrateurs de la communauté.
- **Portée :** une configuration est locale et cloisonnée ; modifier un paramètre NEXUS sur la guilde A n'affecte en rien la guilde B.
- **Disponibilité :** si un module est désactivé (OFF), toutes ses commandes Discord associées répondront par un message d'erreur ou seront ignorées par le `nexus-bot`.

## Résultat attendu

Après sauvegarde, les réglages affichés correspondent au serveur Discord sélectionné et les modules actifs (et eux seuls) sont utilisables dans cette communauté, avec les règles exactes qui ont été paramétrées.

