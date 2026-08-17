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
- **Activation explicite :** sans réglage `enabled` posé pour le serveur, le module est éteint — pour Discord comme pour le dashboard. Une clé absente ne vaut jamais autorisation.

## Les défis sans réponse

Lancer un défi pose immédiatement le délai d'attente de l'attaquant. Tant que l'adversaire ne répond ni oui ni non, l'attaquant reste donc puni d'une bagarre qui n'a jamais eu lieu.

Un défi laissé sans réponse est fermé automatiquement au bout de 24 heures, et le délai d'attente de l'attaquant est levé — il n'avait rien à se reprocher. Rien n'est prélevé à personne : un défi en attente n'a débité aucune mise, et les paris ne s'ouvrent qu'une fois le défi accepté.

La fermeture est faite par un passage régulier (`COUSSIN_EXPIRE_COMBATS_INTERVAL_SECS`, 15 min par défaut).

## La fouille sous les coussins (`/chiper`)

La fouille se jouait sur un pourcentage fixe : 30 % de réussite, et la cible n'y pouvait rien. Perdre sept fois sur dix sans avoir eu son mot à dire n'est pas un jeu, c'est une taxe.

Elle reprend désormais le fonctionnement de l'ancien Coup de Coude :

1. `/chiper` **ouvre** la fouille — aucun coin ne bouge encore.
2. Un bouton **« Serrer les coussins »** apparaît, adressé à la cible seule.
3. Si elle réagit à temps, elle garde **toute sa défense** et la fouille se résout aussitôt.
4. Si elle laisse passer la fenêtre, son absence vaut réponse : elle encaisse un **malus de vigilance** et le voleur passe beaucoup plus facilement.

Le résultat vient de **deux jets de dé opposés**, pas d'un pourcentage :

- **Voleur** : d20 + 4 s'il est de la classe Piégeur.
- **Cible** : d20 + (sa DEF ÷ 10), moins le malus si elle n'a pas réagi.

En cas d'égalité, la cible l'emporte : un vol doit se mériter strictement. Le message affiche les deux totaux — un joueur qui perd doit pouvoir voir pourquoi.

Deux réglages décident de l'équilibre, dans la configuration Nexus, section « Fouille » :

- **Temps pour réagir** (60 s par défaut). Trop court, personne ne voit la notification et la défense ne sert à rien.
- **Malus de vigilance** (8 par défaut). À 0, réagir ne change plus rien et le bouton devient décoratif.

Se blinder garde son intérêt même absent : une DEF élevée résiste encore une fois le malus déduit. À l'inverse, une cible sans défense ne se met jamais à *aider* le voleur — le bonus s'arrête à zéro.

Les anciennes clés `steal_success_pct` et `steal_success_pct_piegeur` ont disparu : plus personne ne les lit, et un curseur sans effet fait croire le problème réglé.

## Pourquoi une acceptation peut être refusée

Accepter un défi exige que **les deux** joueurs aient de quoi payer la mise : personne n'entre dans un duel qu'il ne pourrait pas régler. Si l'un des deux n'a pas assez de coins — ou pas encore de porte-monnaie du tout — l'acceptation est refusée, et le message dit lequel des deux bloque. La réponse n'est visible que de la personne qui a cliqué : de l'extérieur, le défi semble simplement ne pas réagir.

## Résultat attendu

Une recherche ou un filtrage sur le dashboard web doit afficher le joueur correspondant et son état exact. Le classement permet de comparer les joueurs et de savoir instantanément qui domine le jeu, en toute transparence.

