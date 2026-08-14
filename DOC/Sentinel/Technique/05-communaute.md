# 5. Communauté et tickets

## Familles de données

- annonces, embeds et messages ;
- actualités, sondages, événements et recherche de joueurs ;
- confessions et signalements associés ;
- tickets, messages, assignation et clôture ;
- idées et états de traitement ;
- salons vocaux, panneaux de rôles, niveaux et rôles temporaires.

## Règles techniques

Chaque objet est rattaché à une guilde. Les publications doivent identifier le salon cible. Les contenus anonymes doivent conserver la confidentialité de l'auteur. Un ticket ou une idée doit avoir un état explicite et un historique des changements.

## Tâches différées

Le worker peut vérifier le SLA des tickets, fermer les tickets inactifs, expirer des rôles temporaires et traiter les rappels. Une relance ne doit pas créer deux actions identiques.

