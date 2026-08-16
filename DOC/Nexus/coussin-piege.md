# Coussin Piégé

Le Coussin Piégé est un jeu de classement dans lequel les membres disposent d'un nombre de points de vie virtuels.

## Comment ça marche

Le Coussin Piégé est un module de mini-jeu intégré au bot NEXUS. Lorsqu'un joueur participe (via commande ou interaction Discord), `platform-api` calcule les conséquences (ex: perte ou gain de points de vie) en se basant sur une logique aléatoire ou des règles préétablies. Les points de vie actuels et le statut d'élimination sont mis à jour dans PostgreSQL. Le dashboard web interroge l'API pour récupérer en temps réel le classement et l'état de santé des participants de la communauté, sans permettre d'altérer tricher ou de tricher.

## Les actions des utilisateurs

- **Membres (Joueurs) :** s'inscrire au jeu via Discord, participer aux événements de piégeage, consulter leurs points de vie et vérifier le classement général.
- **Administrateurs / Animateurs :** configurer le module (points de vie de départ, récompenses pour le gagnant), consulter le dashboard web pour suivre l'évolution de la partie, voir qui est encore en vie ou éliminé, et relancer une nouvelle partie à la fin.

## Les options

- **Affichage :** l'interface web permet de lister les joueurs, de trier par classement, ou de rechercher un pseudo spécifique.
- **Paramétrage (via Configuration) :** l'administrateur peut régler le nombre de HP initiaux, les pénalités moyennes, et l'activation du module.

## Les conditions

- **Participation :** seuls les membres du serveur Discord où le jeu est actif (module `enabled`) peuvent être listés et interagir avec le Coussin.
- **Lecture seule (Web) :** le dashboard web est une vue en *lecture seule* des statistiques. Les administrateurs ne peuvent pas soigner manuellement un joueur ou baisser ses points de vie depuis l'interface web pour garantir l'intégrité du jeu.
- **Élimination :** un joueur dont les points de vie (HP) tombent à zéro ou moins est considéré comme éliminé et ne peut plus agir jusqu'à la prochaine partie.

## Résultat attendu

Une recherche ou un filtrage sur le dashboard web doit afficher le joueur correspondant et son état exact. Le classement permet de comparer les joueurs et de savoir instantanément qui domine le jeu, en toute transparence.

