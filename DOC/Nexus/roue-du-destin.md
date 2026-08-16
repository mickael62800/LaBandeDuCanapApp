# Roue du Destin

La Roue du Destin est une roue de récompenses utilisable dans les animations NEXUS.

## Comment ça marche

La Roue du Destin est un jeu de tirage au sort intégré au bot NEXUS. L'administrateur configure les segments de la roue via le dashboard web, et ces données sont sauvegardées par `platform-api`. Lorsqu'un membre lance la commande Discord associée (ex: `/roue`), le bot génère un résultat aléatoire (en tenant compte de probabilités équiprobables ou pondérées, si configuré). Le bot applique ensuite l'effet de la case (gain de points, de réputation, ou rien du tout) et répond au membre.

## Les actions des utilisateurs

- **Membres :** lancer la Roue sur Discord (en payant potentiellement le coût d'activation en jetons) pour espérer remporter des récompenses.
- **Administrateurs / Animateurs :** paramétrer visuellement les segments de la roue depuis le dashboard web (nommer les cases, assigner des récompenses), modifier l'ordre d'affichage ou remettre la roue à zéro (reset).

## Les options

- **Segments (Cases) :** définition du nom de la case (ex: "Jackpot", "Perdu", "Rejoue") et de l'identifiant technique de la récompense liée.
- **Ordre :** possibilité de glisser-déposer les cases pour modifier leur ordre visuel sur la roue.
- **Reset :** bouton de restauration pour revenir à la configuration par défaut fournie par le système.

## Les conditions

- **Validation :** les changements opérés sur la roue web ne s'appliquent sur Discord qu'une fois explicitement enregistrés.
- **Économie :** les récompenses accordées par la roue s'interfacent avec le module d'économie virtuelle. Il n'y a aucune implication d'argent réel.
- **Activation :** la roue ne peut être utilisée sur Discord que si le module est activé dans l'onglet Configuration globale de Nexus.

## Résultat attendu

La roue invoquée sur Discord doit utiliser exactement les cases enregistrées et leur ordre tel que défini sur le dashboard. Une suppression ou une modification d'une case prend effet immédiatement sur le prochain tirage après enregistrement.

