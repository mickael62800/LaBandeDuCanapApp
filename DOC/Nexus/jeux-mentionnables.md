# Jeux mentionnables

Cette fonctionnalité permet de proposer des jeux directement dans Discord lorsqu'un membre veut lancer une activité.

## Comment ça marche

Ce module permet de simplifier la recherche de joueurs sur le serveur en publiant un panneau interactif (un message Discord avec des boutons). L'administrateur définit les jeux qui l'intéressent sur le dashboard web. Ces données sont stockées en base via `platform-api`. Lorsqu'il déploie le panneau, le bot envoie un message avec un menu déroulant ou des boutons. Quand un membre clique, le bot lui attribue temporairement (ou définitivement) le rôle Discord correspondant au jeu, ce qui permet aux autres de le mentionner (ex: `@Joueurs Valorant, on lance ?`).

## Les actions des utilisateurs

- **Administrateurs / Animateurs :** ajouter de nouveaux jeux dans la liste du dashboard, regrouper les jeux par catégorie, supprimer un ancien jeu, et déclencher l'envoi/mise à jour du message du panneau de rôles sur un canal Discord spécifique.
- **Membres :** utiliser le panneau interactif sur Discord en cliquant sur les boutons ou menus déroulants pour s'abonner ou se désabonner des notifications d'un jeu.

## Les options

- **Définition du jeu :** nom du jeu (obligatoire) et catégorie (facultative) pour organiser visuellement le panneau.
- **Déploiement :** sélection du salon Discord cible où le bot postera ou mettra à jour le message interactif.

## Les conditions

- **Permissions :** seules les personnes ayant des droits d'administration peuvent ajouter des jeux ou déclencher le déploiement du panneau.
- **Création de rôles :** le bot doit avoir la permission Discord de créer et d'attribuer des rôles pour que le système fonctionne correctement en arrière-plan.
- **Persistance :** si un jeu est supprimé du dashboard, le rôle Discord associé n'est pas forcément détruit, mais il disparaîtra du panneau interactif lors du prochain déploiement.

## Résultat attendu

Après l'ajout, le jeu apparaît dans la liste web. Après le déploiement, les membres peuvent voir et utiliser le panneau interactif dans le salon choisi pour récupérer instantanément le rôle désiré. La suppression retire le jeu des propositions futures sur le panneau.

