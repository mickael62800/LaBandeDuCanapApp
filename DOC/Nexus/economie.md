# Economie

L'économie permet de suivre les ressources virtuelles utilisées par les membres dans les jeux et les animations NEXUS.

## Comment ça marche

Le module d'économie gère une devise virtuelle (souvent appelée "pièces" ou "crédits") utilisée pour récompenser l'activité et l'engagement sur le serveur. Lorsqu'un membre gagne un jeu (comme la Roue du Destin), achète un rôle, ou reçoit une récompense périodique, le bot interagit avec `platform-api` pour créditer ou débiter le compte de l'utilisateur. Chaque transaction est enregistrée de manière transactionnelle dans PostgreSQL, ce qui permet de tracer l'origine de chaque mouvement (l'historique) et de construire un classement communautaire.

## Les actions des utilisateurs

- **Membres :** gagner de l'argent virtuel en jouant ou en participant, dépenser cet argent dans des boutiques de rôles virtuels ou pour lancer des mini-jeux, consulter leur solde et leur historique de transactions (via Discord).
- **Administrateurs / Animateurs :** consulter le dashboard web pour auditer les portefeuilles virtuels, surveiller la création de monnaie (inflation virtuelle), voir le classement global des plus riches, analyser l'historique d'un joueur suspect.

## Les options

- **Classement (Dashboard) :** possibilité de lister tous les membres triés par solde décroissant pour voir les plus riches.
- **Historique détaillé :** en sélectionnant un membre, un administrateur peut voir toutes ses transactions (date, montant, raison/origine).
- **Configuration (via onglet config) :** désignation de la devise, définition du montant distribué chaque jour ou lors d'une victoire.

## Les conditions

- **Ressources virtuelles :** cette économie est strictement virtuelle et n'a aucune valeur financière réelle. Aucun paiement par carte ou fiat n'est possible.
- **Permissions de consultation :** seul le membre peut voir son propre portefeuille sur Discord. Sur le web, seuls les administrateurs ont accès à la vue globale de tous les comptes.
- **Intégrité :** à des fins de sécurité, le dashboard web permet de *consulter* les historiques et soldes, mais un administrateur ne peut pas générer ou supprimer de la monnaie depuis l'interface (cela doit passer par les événements de jeu prévus).

## Résultat attendu

Une consultation sur l'interface doit retourner le classement instantané de la communauté ou l'historique complet d'un membre. Le solde affiché doit correspondre exactement à la somme mathématique de tous les mouvements tracés dans l'historique.

