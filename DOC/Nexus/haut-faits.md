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

## État de l'implémentation

Première tranche livrée, centrée sur Palworld.

**En place**

- Migration `platform-api/migrations/nexus/031_achievements.sql` : tables
  `achievements`, `game_player_links`, `user_achievements`, catalogue Palworld
  (56 définitions) et module de config `nexus-achievements` (salon d'annonce,
  interrupteur d'annonce, mention, profils publics).
- Domaine `platform-core::nexus` : entités, ports et
  `achievements_service` (règles d'attribution, idempotence, filtrage des
  hauts faits secrets).
- API : `GET/PATCH /api/achievements/definitions`, progression d'un membre,
  liaison d'identité (`PUT/GET/DELETE .../links/...`), attribution manuelle
  (`POST .../grant`) et relais d'événement de jeu (`POST .../game-events`).
  Publication de `achievement.unlocked` après persistance confirmée.
- Bot : consumer durable de `achievement.unlocked` (annonce dans le salon
  configuré, salon vérifié comme appartenant à la guilde) et commande
  `/haut-faits` (`moi`, `membre`, `compte`, `lier`, `delier`), réponses
  éphémères.
- Dashboard : page **Hauts faits** (`/nexus/haut-faits`) pour choisir l'image
  de chaque haut fait, parmi les visuels livrés dans
  `web/public/Achievement/<jeu>/` ou une URL libre, et activer/désactiver une
  définition.

**Liaison des joueurs**

Le membre déclare lui-même son identité — c'est la vérification exigée par ce
document : elle vient de son propre compte Discord, pas d'un nom lu dans un log.

Deux points d'entrée, pour la même opération :

- **Boutons du panneau d'inscription** — « ID Steam » / « ID Xbox » ouvrent une
  **modale** de saisie. C'est le chemin principal : le joueur lie son compte au
  moment où il pense à la session, sans quitter le salon. Le jeu est déduit du
  serveur porteur du panneau, il n'a donc pas à le choisir.
- **`/haut-faits lier`** — même opération en commande, avec choix explicite du
  jeu et de la plateforme.

Le format dépend de la **plateforme**, pas du jeu (Palworld se joue via Steam
et via le Microsoft Store) :

| Plateforme | Format accepté |
|---|---|
| `steam` | SteamID64 — 17 chiffres, préfixe `7656119` |
| `xbox` | XUID (16 chiffres) ou Gamertag (3 à 15 caractères) |

Une identité ne peut être revendiquée que par un seul membre par guilde, et un
membre n'a qu'une identité par jeu (les deux unicités sont portées par le
schéma).

⚠️ L'attribution **automatique** par l'adaptateur de présence n'est établie que
pour **Steam** : `ShowPlayers` renvoie un SteamID64. Un joueur Xbox peut lier
son compte et recevoir des hauts faits manuels, mais la correspondance
automatique ne sera effective que si le serveur rapporte ce même identifiant —
ce qui reste à valider sur un serveur avec crossplay actif.

**Adaptateur Palworld : la présence par RCON**

La source retenue est l'option 2 du document (RCON). `ShowPlayers` renvoie le
**SteamID64** de chaque joueur connecté : la présence est donc une observation
vérifiable, reliable à un membre Discord via `game_player_links`.

- `platform-core::…::game::presence` porte **tout le contrat RCON par jeu** :
  la commande, l'analyse de la réponse, et les variables d'environnement qui
  activent la console. Deux défauts réels y sont corrigés :
  - le health check interrogeait tous les jeux avec la commande Minecraft
    (`list`), donc rapportait « 0 joueur » sur un serveur Palworld peuplé — et
    ce compteur alimente l'extinction automatique ;
  - la plateforme injectait `ENABLE_RCON` / `RCON_PASSWORD` (conventions des
    images Minecraft `itzg`) à **toutes** les images. Palworld attend
    `RCON_ENABLED`, et n'a pas de mot de passe RCON distinct : c'est
    l'`ADMIN_PASSWORD` du serveur. RCON restait donc fermé côté Palworld, et la
    plateforme interrogeait un port où personne n'écoutait.

  Pour Palworld, le mot de passe RCON est désormais l'`ADMIN_PASSWORD`
  **effectif** — celui choisi dans l'interface reste donc autoritaire.
- Le job `palworld-presence` (scheduler → `POST
  /api/games/internal/jobs/palworld-presence`, 120 s par défaut) relève les
  joueurs et demande l'attribution. Les `source_event_id` sont stables par
  (guilde, joueur, haut fait) : rejouer le job ne crée aucun doublon.

Deux hauts faits sont donc en `auto` : `first_launch_palworld` et
`palworld_massive_session` (seuil `criteria.players`, lu depuis la définition
et jamais codé en dur).

**Non livré — et pourquoi**

Les hauts faits de gameplay (boss, Paldeck, élevage, bases, exploration) ne
sont **pas observables par RCON**. Les sources envisageables pour aller plus
loin, ce qu'elles couvrent réellement et ce qui restera hors de portée sont
analysés dans [`palworld-sources.md`](palworld-sources.md). Ils restent en `verification = 'manual'` :
seul un administrateur peut les attribuer, de façon tracée (`granted_by`). Les
rendre automatiques demandera une source qui les prouve (mod, plugin ou
lecture de sauvegarde validée), pas une déduction depuis un signal qui ne les
établit pas. La route `POST /game-events` est déjà là pour accueillir un tel
producteur.

## Catalogue initial des hauts faits Discord

### Arrivée et découverte

| Code | Nom | Critère |
|---|---|---|
| `welcome` | Bienvenue parmi nous | Rejoindre le serveur Discord |
| `first_steps` | Premier pas | Lire le règlement |
| `explorer` | Explorateur | Consulter plusieurs salons |
| `profile_completed` | Profil complété | Configurer son profil ou ses rôles |
| `community_intro` | Présenté à la communauté | Se présenter dans le salon prévu |

### Messages et participation

| Code | Nom | Critère |
|---|---|---|
| `first_message` | Premier message | Envoyer son premier message |
| `talkative_100` | Bavard | Envoyer 100 messages |
| `talkative_1000` | Pipelette | Envoyer 1 000 messages |
| `talkative_10000` | Infatigable | Envoyer 10 000 messages |
| `regular` | Régulier | Participer plusieurs jours consécutifs |
| `community_pillar` | Pilier de la communauté | Être actif pendant 30 jours |
| `old_timer` | Ancien de la maison | Être présent depuis un an |
| `engaged_conversation` | Conversation engagée | Participer à plusieurs discussions |
| `reaction_chain` | Réaction en chaîne | Réagir à plusieurs messages |
| `mood_maker` | Ambianceur | Recevoir un nombre défini de réactions positives |

### Vocal

| Code | Nom | Critère |
|---|---|---|
| `first_voice` | Premier vocal | Rejoindre un salon vocal |
| `voice_1h` | Bavard vocal | Passer une heure en vocal |
| `voice_10h` | Habitué du vocal | Passer 10 heures en vocal |
| `voice_marathon` | Marathon vocal | Participer plusieurs heures à une même session |
| `voice_regular` | Toujours présent | Rejoindre des vocaux plusieurs jours |
| `voice_host` | Animateur vocal | Participer à une soirée ou activité vocale |
| `familiar_voice` | Voix familière | Rejoindre régulièrement le même salon |

### Communauté

| Code | Nom | Critère |
|---|---|---|
| `team_spirit` | Esprit d'équipe | Participer à un événement communautaire |
| `organizer` | Organisateur | Créer ou organiser un événement |
| `community_guide` | Guide de la communauté | Aider un nouveau membre |
| `warm_welcome` | Bienvenue chaleureuse | Accueillir plusieurs nouveaux membres |
| `mediator` | Médiateur | Participer positivement à une discussion difficile |
| `helping_hand` | Coup de main | Répondre à plusieurs demandes d'aide |
| `faithful_participant` | Fidèle participant | Participer à plusieurs événements |
| `community_builder` | Membre fédérateur | Faire participer plusieurs membres |

### Système Nexus

| Code | Nom | Critère |
|---|---|---|
| `first_game` | Premier jeu | Lancer sa première session |
| `confirmed_player` | Joueur confirmé | Participer à plusieurs sessions |
| `multigaming` | Multigaming | Jouer à trois jeux différents |
| `game_explorer` | Découvreur | Jouer à cinq jeux différents |
| `portal_master` | Maître du portail | Utiliser les fonctions principales du Game Portal |
| `first_victory` | Première victoire | Gagner une partie ou une activité |
| `winning_streak` | Série victorieuse | Remporter plusieurs parties |
| `team_player` | Joueur d'équipe | Participer à une session avec plusieurs membres |
| `session_host` | Hôte de session | Créer une session de jeu |
| `session_gatherer` | Rassembleur | Inviter plusieurs membres dans une session |

### Économie et jeux Discord

| Code | Nom | Critère |
|---|---|---|
| `first_reward` | Premier gain | Recevoir ses premières récompenses |
| `first_purchase` | Premier achat | Acheter un élément dans la boutique |
| `saver` | Épargnant | Atteindre un montant défini dans le wallet |
| `spender` | Dépensier | Effectuer plusieurs achats |
| `lucky` | Chanceux | Gagner à la Roue du Destin |
| `wheel_regular` | Roue en mouvement | Utiliser la roue plusieurs fois |
| `careful_player` | Joueur prudent | Utiliser une protection ou une assurance |
| `lucky_strike` | Coup de chance | Obtenir une récompense rare |
| `bad_idea` | Mauvaise idée | Déclencher le Coussin Piégé |
| `cushion_survivor` | Survivant du coussin | Réussir une série d'activités du Coussin Piégé |

### Modération et sécurité positive

Ces hauts faits récompensent uniquement des actions utiles et validées. Une
infraction, un spam, une sanction reçue ou un contournement de sécurité ne doit
jamais être récompensé.

| Code | Nom | Critère |
|---|---|---|
| `server_secured` | Serveur sécurisé | Activer les fonctions essentielles de sécurité |
| `setup_completed` | Configuration terminée | Configurer les outils principaux de Sentinel |
| `moderator_beginner` | Modérateur débutant | Effectuer une action de modération validée |
| `trusted_reporter` | Veilleur | Signaler correctement un contenu problématique |
| `community_protector` | Protecteur | Participer à des actions de protection communautaire |
| `audit_reader` | Journaliste | Consulter les journaux d'audit autorisés |
| `organized_server` | Serveur organisé | Configurer salons, rôles ou règles recommandés |

### Hauts faits secrets

| Code | Nom | Critère |
|---|---|---|
| `found_it` | Trouvé ! | Découvrir une commande cachée |
| `midnight` | À minuit | Réaliser une action à une heure spéciale |
| `collector` | Collectionneur | Débloquer plusieurs hauts faits rares |
| `without_interruption` | Sans interruption | Maintenir une série d'activité |
| `community_legend` | Légende de la communauté | Débloquer un ensemble complet |
| `old_survivor` | Ancien survivant | Être actif depuis la création du serveur |
| `try_everything` | Tout essayer | Utiliser une fonctionnalité de chaque univers |

### Premiers hauts faits par jeu

Ces entrées peuvent être créées pour chaque jeu du portail, sous réserve que
la session et l'identité du joueur soient connues :

- `first_launch_7dtd` — Premier lancement — 7 Days to Die ;
- `first_launch_core_keeper` — Premier lancement — Core Keeper ;
- `first_launch_factorio` — Premier lancement — Factorio ;
- `first_launch_minecraft` — Premier lancement — Minecraft ;
- `first_launch_palworld` — Premier lancement — Palworld ;
- `first_launch_space_engineers` — Premier lancement — Space Engineers ;
- `first_launch_starbound` — Premier lancement — Starbound ;
- `first_launch_terraria` — Premier lancement — Terraria ;
- `first_launch_valheim` — Premier lancement — Valheim ;
- `first_launch_v_rising` — Premier lancement — V Rising ;
- `first_launch_zomboid` — Premier lancement — Project Zomboid.

Les hauts faits propres au gameplay de chaque jeu restent expérimentaux tant
qu'un adaptateur fiable — logs structurés, RCON, plugin ou mod — n'a pas été
validé.

## Hauts faits Palworld avancés

Ces hauts faits sont volontairement difficiles. Ils doivent être attribués
uniquement après réception d'événements vérifiables et ne doivent pas être
déduits d'un simple message Discord.

### Progression extrême

| Code | Nom | Critère proposé |
|---|---|---|
| `palworld_full_paldeck` | Paldeck presque complet | Capturer toutes les espèces prévues par la saison du serveur |
| `palworld_all_towers` | Maître des tours | Vaincre tous les boss de tour |
| `palworld_all_legendaries` | Chasseur de légendes | Vaincre tous les boss légendaires configurés |
| `palworld_all_alpha_bosses` | Dompteur d'Alphas | Vaincre tous les Alphas suivis par le serveur |
| `palworld_max_level` | Niveau maximum | Atteindre le niveau maximal du serveur |
| `palworld_technology_complete` | Technologie ultime | Débloquer toutes les technologies prévues |
| `palworld_endgame` | Fin de parcours | Atteindre simultanément les objectifs de progression de fin de jeu |

### Défis sans marge d'erreur

| Code | Nom | Critère proposé |
|---|---|---|
| `palworld_boss_no_down` | Invaincu | Vaincre un boss sans être mis K.O. |
| `palworld_boss_no_death` | Aucun sacrifice | Vaincre un boss sans perte de Pal dans l'équipe |
| `palworld_boss_under_time` | Course contre le temps | Vaincre un boss avant une durée limite |
| `palworld_boss_under_level` | Contre toute attente | Vaincre un boss avec un niveau inférieur au niveau recommandé |
| `palworld_boss_single_element` | Spécialiste élémentaire | Vaincre un boss avec une équipe d'un seul élément |
| `palworld_boss_single_pal` | Un seul compagnon | Vaincre un boss avec un seul Pal actif |
| `palworld_no_fast_travel` | Marcheur infatigable | Terminer une expédition ou un objectif sans téléportation |
| `palworld_no_death_run` | Sans seconde chance | Atteindre un objectif majeur sans mourir |

### Élevage et maîtrise des Pals

| Code | Nom | Critère proposé |
|---|---|---|
| `palworld_perfect_breed` | Élevage parfait | Obtenir un Pal avec les critères de reproduction définis |
| `palworld_passive_master` | Maître des passifs | Obtenir un Pal avec une combinaison de passifs rare |
| `palworld_breed_chain` | Lignée exceptionnelle | Réaliser une chaîne d'élevage de plusieurs générations |
| `palworld_one_species_team` | Équipe spécialisée | Vaincre un objectif avec une équipe d'une même espèce |
| `palworld_full_team_bred` | Équipe issue de l'élevage | Utiliser une équipe complète issue de reproductions |
| `palworld_pal_workforce` | Main-d'œuvre parfaite | Faire fonctionner une base avec des Pals ayant les aptitudes requises |
| `palworld_partner_loyalty` | Partenaire fidèle | Utiliser le même Pal sur une longue progression |
| `palworld_rare_collection` | Collection rare | Obtenir plusieurs variantes ou Pals rares suivis par le serveur |

### Base et production

| Code | Nom | Critère proposé |
|---|---|---|
| `palworld_automated_base` | Base autonome | Maintenir une production complète pendant une durée définie |
| `palworld_three_bases` | Triple implantation | Maintenir trois bases opérationnelles |
| `palworld_raid_proof` | Forteresse imprenable | Résister à plusieurs raids sans bâtiment critique détruit |
| `palworld_mass_production` | Production industrielle | Produire une quantité élevée d'objets ou de ressources |
| `palworld_logistics_master` | Maître logistique | Maintenir une base sans rupture de ressources critiques |
| `palworld_base_specialist` | Base spécialisée | Atteindre le rendement cible d'une base spécialisée |
| `palworld_rebuild` | Reconstruction héroïque | Restaurer une base après un raid ou un incident majeur |
| `palworld_server_supplier` | Fournisseur du serveur | Produire et partager des ressources avec plusieurs joueurs |

### Exploration longue durée

| Code | Nom | Critère proposé |
|---|---|---|
| `palworld_world_explorer` | Explorateur du monde | Découvrir toutes les zones suivies par le serveur |
| `palworld_dungeon_chain` | Maître des donjons | Terminer plusieurs donjons consécutivement |
| `palworld_all_fast_travel` | Réseau complet | Découvrir tous les points de voyage rapide |
| `palworld_extreme_expedition` | Expédition extrême | Revenir vivant d'une zone de très haut niveau |
| `palworld_map_without_death` | Cartographe prudent | Explorer une grande partie de la carte sans mourir |
| `palworld_night_explorer` | Enfant de la nuit | Accomplir une exploration nocturne complète |
| `palworld_sea_to_sky` | De la mer au ciel | Utiliser plusieurs types de montures pendant une même expédition |

### Coopération et serveur communautaire

| Code | Nom | Critère proposé |
|---|---|---|
| `palworld_coop_boss` | Boss en équipe | Vaincre un boss avec un groupe complet de joueurs |
| `palworld_coop_no_down` | Escouade invincible | Réussir un combat de groupe sans joueur mis K.O. |
| `palworld_shared_base` | Base communautaire | Participer à la construction d'une base partagée |
| `palworld_rescue_team` | Équipe de secours | Aider plusieurs joueurs à récupérer après une défaite |
| `palworld_newcomer_mentor` | Mentor de Palworld | Accompagner un nouveau joueur jusqu'à un objectif défini |
| `palworld_server_event` | Événement historique | Participer à un événement communautaire majeur |
| `palworld_massive_session` | Grande expédition | Participer à une session réunissant beaucoup de joueurs |
| `palworld_guild_legacy` | Héritage de guilde | Contribuer à plusieurs objectifs collectifs du serveur |

### Maîtrise totale

| Code | Nom | Critère proposé |
|---|---|---|
| `palworld_speedrunner` | Coureur de Palworld | Atteindre un objectif de progression dans un temps record |
| `palworld_survivalist` | Survie absolue | Atteindre une durée élevée sans mort |
| `palworld_completionist` | Complétionniste | Débloquer toutes les catégories de hauts faits Palworld |
| `palworld_legendary_trainer` | Dresseur légendaire | Réunir progression, élevage, exploration et combats avancés |
| `palworld_world_guardian` | Gardien du monde | Protéger plusieurs bases et participer aux défenses du serveur |
| `palworld_immortal_run` | Parcours immortel | Atteindre la fin de parcours sans aucune mort du joueur |
| `palworld_server_champion` | Champion du serveur | Être premier dans plusieurs classements Palworld |
| `palworld_community_legend` | Légende de Palworld | Accomplir un ensemble de hauts faits légendaires |

Les critères de durée, de niveau, de nombre de joueurs et de difficulté doivent
être configurables par serveur. Ils ne doivent jamais être codés en dur dans le
bot. Les hauts faits impossibles à vérifier automatiquement restent masqués ou
réservés à une validation d'administrateur clairement auditée.

## Hauts faits Discord avancés

Ces hauts faits demandent une implication longue ou une combinaison d'actions.
Ils doivent être calculés côté API à partir d'événements persistés, jamais à
partir du seul compteur affiché dans le bot.

### Engagement sur la durée

| Code | Nom | Critère proposé |
|---|---|---|
| `discord_streak_7` | Une semaine fidèle | Être actif 7 jours consécutifs |
| `discord_streak_30` | Trente jours parmi nous | Être actif 30 jours consécutifs |
| `discord_streak_100` | Cent jours de présence | Être actif 100 jours selon les règles du serveur |
| `discord_year_active` | Année complète | Être actif sur douze mois distincts |
| `discord_all_seasons` | Toutes les saisons | Participer à chaque saison ou période officielle |
| `discord_daily_regular` | Rituel quotidien | Atteindre plusieurs séries quotidiennes validées |
| `discord_returning_member` | Toujours de retour | Revenir activement après plusieurs périodes d'absence |
| `discord_long_term_pillar` | Pilier historique | Cumuler ancienneté et activité au-dessus des seuils définis |

### Défis de participation

| Code | Nom | Critère proposé |
|---|---|---|
| `discord_all_event_types` | Touche-à-tout | Participer à chaque type d'événement du serveur |
| `discord_event_marathon` | Marathon communautaire | Participer à plusieurs événements dans une période courte |
| `discord_event_winner` | Vainqueur officiel | Remporter un événement enregistré par le bot |
| `discord_podium_collection` | Collection de podiums | Obtenir plusieurs classements dans des événements différents |
| `discord_season_complete` | Saison terminée | Atteindre les objectifs d'une saison communautaire |
| `discord_perfect_attendance` | Présence parfaite | Participer à toutes les étapes d'un événement multi-jours |
| `discord_event_organizer` | Architecte d'événements | Organiser plusieurs événements validés |
| `discord_community_calendar` | Année animée | Contribuer à des événements répartis sur une année |

### Entraide et communauté

| Code | Nom | Critère proposé |
|---|---|---|
| `discord_mentor_5` | Mentor confirmé | Accompagner 5 nouveaux membres selon un parcours terminé |
| `discord_mentor_25` | Référent communautaire | Accompagner 25 nouveaux membres |
| `discord_help_streak` | Aide constante | Répondre utilement pendant plusieurs jours consécutifs |
| `discord_rescue_team` | Équipe de secours | Aider plusieurs membres lors d'incidents ou difficultés |
| `discord_bridge_builder` | Créateur de liens | Faire participer des membres qui n'avaient pas encore interagi |
| `discord_peacemaker` | Gardien de l'entente | Participer à des résolutions validées par la modération |
| `discord_knowledge_keeper` | Mémoire de la communauté | Fournir plusieurs réponses validées dans la base de connaissances |
| `discord_welcome_committee` | Comité d'accueil | Accueillir régulièrement de nouveaux membres |

### Vocal et activités collectives

| Code | Nom | Critère proposé |
|---|---|---|
| `discord_voice_streak` | Voix régulière | Participer à un vocal plusieurs jours consécutifs |
| `discord_voice_marathon` | Marathon vocal | Participer à une longue session sans abandonner l'activité |
| `discord_voice_host_10` | Animateur confirmé | Animer 10 activités vocales enregistrées |
| `discord_voice_all_rooms` | Tour des salons | Participer à plusieurs types de salons vocaux |
| `discord_group_activity` | Toujours en groupe | Participer à plusieurs activités avec un groupe récurrent |
| `discord_cross_game_party` | Équipe multijeux | Jouer avec le même groupe sur plusieurs jeux Nexus |
| `discord_night_owl` | Oiseau de nuit | Participer à plusieurs activités nocturnes autorisées |
| `discord_voice_community_pillar` | Pilier vocal | Cumuler durée, régularité et participation collective |

### Maîtrise du serveur

| Code | Nom | Critère proposé |
|---|---|---|
| `discord_full_discovery` | Visite complète | Découvrir toutes les fonctionnalités publiques du serveur |
| `discord_all_universes` | Voyageur des univers | Utiliser Sentinel, Nexus, Atrium et les fonctionnalités publiques |
| `discord_configuration_master` | Maître de la configuration | Configurer plusieurs fonctionnalités autorisées |
| `discord_security_partner` | Partenaire sécurité | Participer à plusieurs actions positives de sécurité |
| `discord_feedback_loop` | Amélioration continue | Fournir plusieurs retours validés et utiles |
| `discord_bug_hunter` | Chasseur de problèmes | Signaler plusieurs anomalies reproductibles |
| `discord_documentation_reader` | Lecteur attentif | Consulter ou valider plusieurs ressources documentaires |
| `discord_server_ambassador` | Ambassadeur du serveur | Cumuler accueil, aide, participation et ancienneté |

### Défis sociaux et secrets

| Code | Nom | Critère proposé |
|---|---|---|
| `discord_everyone_knows` | Visage connu | Interagir positivement avec un grand nombre de membres distincts |
| `discord_circle_expander` | Cercle élargi | Créer des interactions entre plusieurs groupes de membres |
| `discord_secret_route` | Chemin secret | Découvrir une combinaison cachée de fonctionnalités |
| `discord_perfect_week` | Semaine parfaite | Atteindre plusieurs objectifs communautaires dans la même semaine |
| `discord_perfect_month` | Mois parfait | Conserver activité, vocal, entraide et participation pendant un mois |
| `discord_no_spam` | Qualité avant quantité | Atteindre un objectif d'activité sans avertissement ni détection de spam |
| `discord_positive_record` | Historique exemplaire | Maintenir une période longue sans sanction et avec participation positive |
| `discord_community_legend` | Légende de la communauté | Débloquer plusieurs hauts faits avancés de catégories différentes |

### Défis collectifs

| Code | Nom | Critère proposé |
|---|---|---|
| `discord_collective_goal` | Objectif commun | Contribuer à un objectif atteint par toute la communauté |
| `discord_server_challenge` | Défi du serveur | Participer à un défi communautaire limité dans le temps |
| `discord_guild_record` | Record communautaire | Participer à l'établissement d'un record suivi par le serveur |
| `discord_massive_event` | Grande mobilisation | Participer à un événement réunissant un nombre élevé de membres |
| `discord_cross_universe` | Pont entre univers | Contribuer à une activité impliquant plusieurs univers du dashboard |
| `discord_community_builder` | Bâtisseur collectif | Contribuer durablement à la croissance ou à l'organisation du serveur |

Les critères sociaux doivent respecter la confidentialité : ne pas publier de
classement humiliant, ne pas exposer les messages privés et ne pas récompenser
le volume de messages au détriment de leur qualité. Les seuils doivent être
configurables et les actions de modération doivent pouvoir invalider un haut
fait obtenu par abus.
