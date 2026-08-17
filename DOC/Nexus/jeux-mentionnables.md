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

## La synchronisation avec Discord

Le dashboard et Discord décrivent le même état, sans qu'aucun mécanisme ne garantisse qu'ils restent d'accord. Un rôle supprimé à la main dans Discord, ou un jeu retiré pendant que le bot est hors ligne, laisse les deux côtés en désaccord — et les abonnements échouent alors en silence.

La section « Synchronisation avec Discord » de la page compare les deux mondes et affiche chaque écart : un jeu dont le rôle n'existe plus, un jeu sans rôle, un rôle de jeu que plus aucun jeu ne réclame, un panneau dont le message a disparu de son salon.

**Rien n'est réparé automatiquement.** Pour chaque écart, l'administrateur choisit le côté qui fait foi :

- **Discord fait foi** — le dashboard s'aligne sur ce qui existe vraiment : la liaison morte est effacée, le panneau disparu est oublié.
- **Le dashboard fait foi** — Discord est remis en conformité : le rôle est recréé, le panneau redéployé, le rôle orphelin supprimé.

Chaque résolution est confirmée par une fenêtre qui annonce ce qui va réellement se passer. Une réparation côté Discord passe par le bot : elle est *demandée*, et n'est constatée qu'à la vérification suivante.

Deux garde-fous méritent d'être connus :

- **Tant que le bot n'a pas rendu compte du serveur, l'état est affiché comme inconnu**, jamais comme correct. Ne rien savoir et tout aller bien ne se ressemblent que sur un écran mal conçu.
- **Un salon devenu illisible ne vaut pas panneau disparu.** Le doute profite à l'existant : mieux vaut manquer un écart que republier un panneau vivant ou proposer de supprimer un rôle de modération.

La vérification tourne aussi toute seule (`GAME_MENTION_SYNC_INTERVAL_SECS`, 6 h par défaut), et le bot signale immédiatement à l'API tout rôle supprimé dans Discord — c'est ce qui rattrape le cas le plus courant, le ménage dans les rôles du serveur.

## Résultat attendu

Après l'ajout, le jeu apparaît dans la liste web. Après le déploiement, les membres peuvent voir et utiliser le panneau interactif dans le salon choisi pour récupérer instantanément le rôle désiré. La suppression retire le jeu des propositions futures sur le panneau.

