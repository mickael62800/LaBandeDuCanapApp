# Hauts faits Discord et jeux

## Objectif

NEXUS peut attribuer des hauts faits aux membres Discord pour leurs actions
dans la communauté et dans les serveurs de jeux gérés par le Game Portal.
Les hauts faits sont persistants, attribués une seule fois et publiables dans
un salon Discord configuré par serveur.

Le système doit rester indépendant du jeu. Palworld est le premier adaptateur
prévu, mais le même contrat doit pouvoir accueillir Zomboid, V Rising,
Minecraft, Factorio et les autres jeux du portail.

## Architecture

```text
Événement Discord ou jeu
        ↓
platform-api / adaptateur d'événement
        ↓
platform-core::nexus::achievements
        ↓
PostgreSQL + événement nexus:events
        ↓
nexus-bot
        ↓
Salon Discord configuré
```

Les responsabilités sont séparées ainsi :

- `platform-core` contient les règles d'attribution et les critères ;
- `platform-api` reçoit les événements, vérifie l'identité et persiste ;
- `docker-agent` reste limité aux opérations Docker autorisées ;
- `platform-scheduler` ne fait que déclencher les collectes périodiques ;
- `nexus-bot` affiche les hauts faits et expose les commandes Discord ;
- PostgreSQL est la source de vérité ;
- `nexus:events` transporte les notifications vers le bot.

Le bot ne lit jamais directement la base et le serveur de jeu ne doit jamais
recevoir de privilège Discord.

## Sources d'événements

### Discord

Les événements peuvent provenir de Sentinel ou de Nexus : message envoyé,
arrivée sur le serveur, présence vocale, niveau atteint, participation à un
événement, création d'une session de jeu ou victoire à un jeu Nexus.

L'événement doit contenir au minimum :

```json
{
  "event": "discord.activity",
  "data": {
    "guild_id": "...",
    "user_id": "...",
    "activity": "message_sent",
    "event_id": "...",
    "occurred_at": "..."
  }
}
```

### Jeux Dockerisés

Un serveur de jeu peut fournir ses événements par logs, RCON, plugin, mod ou
adaptateur spécifique. La méthode dépend du jeu et ne doit pas contaminer le
domaine métier.

L'adaptateur normalise les données dans un événement commun :

```json
{
  "event": "game.achievement_candidate",
  "data": {
    "game": "palworld",
    "server_id": "...",
    "game_player_id": "...",
    "achievement_code": "first_survival",
    "source_event_id": "...",
    "occurred_at": "..."
  }
}
```

Le texte brut des logs ne doit pas être considéré comme une preuve suffisante
si l'identité du joueur ou le contexte ne peuvent pas être vérifiés.

## Palworld

Le support Palworld nécessite de choisir une source d'événements compatible
avec le serveur utilisé. Les options sont, par ordre de préférence :

1. plugin ou mod serveur produisant un événement structuré ;
2. RCON si l'information nécessaire est exposée ;
3. analyse contrôlée des logs du conteneur ;
4. lecture de données de sauvegarde uniquement après validation du format.

`docker-agent` peut gérer le cycle de vie du conteneur, mais il ne doit pas
devenir le propriétaire de la logique des hauts faits. La collecte peut être
un adaptateur dédié appelé par `platform-api`, avec accès strictement limité
au serveur concerné.

Un haut fait Palworld ne peut être attribué que si les trois éléments suivants
sont connus : serveur de jeu, joueur du jeu et membre Discord associé.

## Liaison joueur / Discord

Une table de liaison est nécessaire :

```text
guild_id
discord_user_id
game
game_player_id
verified_at
```

La liaison doit être explicitement vérifiée par le membre. Un nom affiché dans
un log ne suffit pas : les homonymes, changements de pseudo et usurpations
doivent être pris en compte.

Sans liaison vérifiée, l'événement peut être conservé comme candidat en
attente, mais aucun haut fait ne doit être attribué ni publié.

## Modèle de données

Les tables prévues sont :

```text
achievements
- id
- game nullable
- code unique par jeu
- name
- description
- icon_url nullable
- criteria
- enabled

user_achievements
- id
- guild_id
- discord_user_id
- achievement_id
- game_player_id nullable
- source_event_id
- unlocked_at

game_player_links
- id
- guild_id
- discord_user_id
- game
- game_player_id
- verified_at
```

Une contrainte d'unicité doit empêcher un membre de recevoir deux fois le même
haut fait pour la même guilde et le même jeu. `source_event_id` doit également
être unique pour rendre la consommation Redis idempotente.

## Publication Discord

La configuration par guilde doit permettre de choisir :

- le salon de publication des hauts faits ;
- l'activation ou non des notifications ;
- la publication dans le salon général ou dans le salon de session ;
- le regroupement de plusieurs hauts faits rapprochés ;
- le rôle ou la mention éventuelle, désactivée par défaut.

Exemple de message :

```text
🏆 Haut fait débloqué

Membre : @joueur
Haut fait : Premier survivant
Jeu : Palworld
Serveur : Palworld communautaire
```

Le bot ne publie le message qu'après confirmation de la transaction métier.
Un échec Discord ne doit pas annuler le haut fait déjà enregistré ; l'événement
reste rejouable ou passe dans une file de notification à réessayer.

## Commandes prévues

- `/haut-faits` : afficher les hauts faits du membre courant ;
- `/haut-faits membre` : consulter ceux d'un autre membre si la configuration
  l'autorise ;
- `/haut-faits jeu` : filtrer par jeu ;
- `/haut-faits progression` : afficher les critères encore manquants ;
- commande d'administration pour définir ou désactiver le salon ;
- commande d'administration pour gérer les définitions activées.

Les commandes d'administration doivent vérifier la permission Discord côté API
et côté bot. Une configuration désactivée ne doit jamais être présentée comme
disponible.

## Événement Redis de publication

Après attribution, NEXUS publie un événement sur `nexus:events` :

```json
{
  "event": "achievement.unlocked",
  "data": {
    "guild_id": "...",
    "discord_user_id": "...",
    "achievement_id": "...",
    "achievement_code": "first_survival",
    "game": "palworld",
    "source_event_id": "..."
  }
}
```

La consommation doit utiliser un consumer group durable. Le bot accuse
réception uniquement après traitement ou classement explicite en échec.

## Sécurité et règles d'or

- Aucun bot ou conteneur de jeu n'accède directement à PostgreSQL.
- Aucun token Docker hôte n'est utilisé pour une opération de jeu.
- Une identité de jeu non vérifiée ne débloque rien.
- Un événement inconnu, incomplet ou ancien est rejeté ou conservé en attente.
- Les événements sont idempotents et rejouables.
- Les messages Discord ne contiennent pas d'identifiants techniques inutiles.
- Les logs ne contiennent ni token, ni mot de passe, ni contenu sensible du jeu.
- Les hauts faits sont propres à une guilde : aucune attribution inter-guildes.
- Le salon cible est validé comme appartenant à la guilde avant publication.

## Plan d'implémentation

1. Ajouter le modèle métier et les ports dans `platform-core/src/nexus`.
2. Ajouter la migration PostgreSQL dans `platform-api/migrations/nexus/`.
3. Ajouter les routes de consultation, liaison et administration dans
   `platform-api/src/nexus`.
4. Ajouter le publisher `achievement.unlocked` sur `nexus:events`.
5. Ajouter le consommateur et les embeds dans `nexus-bot`.
6. Implémenter d'abord les hauts faits Discord et les événements déjà produits
   par Nexus.
7. Ajouter l'adaptateur Palworld après validation de la source d'événements.
8. Ajouter les tests d'idempotence, de permissions, de liaison et de reprise.

La fonctionnalité ne doit pas être déclarée disponible pour Palworld tant que
la source d'événements et la méthode de liaison des joueurs n'ont pas été
validées sur le conteneur réel.
