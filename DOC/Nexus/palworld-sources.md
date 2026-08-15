# Palworld — sources de données pour les hauts faits

Document d'analyse. Il répond à une seule question : **comment attribuer
automatiquement les hauts faits Palworld qui restent en validation manuelle ?**

À lire avec [`haut-faits.md`](haut-faits.md), qui décrit le système lui-même.

## État actuel

Un seul canal est branché : **RCON `ShowPlayers`**, relevé toutes les 120 s par
le job `palworld-presence`. Il renvoie `name,playeruid,steamid` — donc le
**SteamID64**, ce qui relie un joueur connecté à un membre Discord via
`game_player_links`.

Cela couvre **2 hauts faits sur 57** :

| Code | Comment |
|---|---|
| `first_launch_palworld` | première présence constatée |
| `palworld_massive_session` | `criteria.players` joueurs identifiés simultanés |

Les 55 autres sont en `verification = 'manual'`. Ce document explique ce qu'il
faudrait pour en automatiser une partie, et ce qui restera hors de portée.

## Les quatre sources possibles

### 1. RCON (branché)

Commandes utiles : `ShowPlayers`, `Info`, `Save`, `Broadcast`.

**Ce que ça donne** : qui est connecté, avec son identité Steam. Rien sur ce
que le joueur *fait*.

**Plafond** : la présence, et ce qui s'en déduit (durée, simultanéité).

### 2. REST API officielle (non branchée)

Palworld expose une API HTTP depuis la v0.3, à activer côté serveur
(`RESTAPIEnabled` / `RESTAPIPort`, port 8212 par défaut ; dans l'image
`thijsvanloef`, via les variables `REST_API_ENABLED` et `REST_API_PORT`).
Authentification HTTP Basic, utilisateur `admin`, mot de passe =
`ADMIN_PASSWORD` — le même que celui déjà utilisé pour RCON.

**Ce que ça donne** (`GET /v1/api/players`), par joueur connecté :

```
name, accountName, playerId, userId, ip, ping,
location_x, location_y, level, building_count
```

Plus `GET /v1/api/metrics` (fps serveur, nombre de joueurs, uptime) et
`GET /v1/api/settings`.

> ⚠️ La liste exacte des champs varie selon la version du serveur. À vérifier
> sur l'instance réelle avant de coder quoi que ce soit dessus.

**Intérêt** : c'est officiel, stable, JSON — et surtout ça donne le **niveau**
du joueur et sa **position**, que RCON n'expose pas.

**Limite** : c'est un instantané des joueurs *connectés*. Aucun historique,
aucun événement, rien sur les Pals ni sur les technologies.

### 3. Lecture des sauvegardes (non branchée)

Les sauvegardes (`Level.sav`, `Players/<uid>.sav`) sont au format GVAS
compressé. Des outils communautaires les convertissent en JSON
(`palworld-save-tools` est le plus utilisé).

**Ce que ça donne** : le contenu réel de la partie — espèces capturées
(Paldeck), technologies débloquées, Pals possédés avec leurs passifs, camps de
base, guildes, niveau et expérience.

**C'est la source la plus riche**, et de loin.

**Ce qu'elle coûte** :

- `Level.sav` peut peser plusieurs centaines de Mo : le décodage est lourd en
  CPU et en mémoire, à ne pas faire à chaque minute ;
- le format n'est **pas documenté** par l'éditeur : il change à chaque grosse
  mise à jour du jeu, et un outil tiers casse avec ;
- il faut un accès en lecture au volume Docker du serveur, ce que seul
  `docker-agent` peut faire — ajouter un chemin de lecture de fichiers y est
  une décision d'architecture à peser (règle 9 du `CLAUDE.md`).

### 4. Mods serveur

Palworld n'a **pas d'API de mods serveur officielle**. Ce qui existe repose sur
de l'injection (UE4SS et dérivés), casse à chaque patch, et ferait dépendre les
hauts faits d'un binaire non officiel dans le conteneur de jeu. **Écarté.**

## Ce que chaque source couvre réellement

Classement des 57 hauts faits Palworld par source nécessaire.

### A. Déjà automatique — présence RCON (2)

`first_launch_palworld`, `palworld_massive_session`

### B. Atteignable par la REST API (~5)

| Code | Donnée utilisée | Fiabilité |
|---|---|---|
| `palworld_max_level` | `level` | ✅ exacte |
| `palworld_survivalist` | durée de présence cumulée | ⚠️ « sans mort » non vérifiable |
| `palworld_world_explorer` | `location_x/y` échantillonnées | ⚠️ approximation par zones visitées |
| `palworld_all_fast_travel` | positions | ⚠️ approximation |
| `palworld_night_explorer` | positions + heure | ⚠️ l'heure **du jeu** n'est pas exposée |

Seul `palworld_max_level` est mesuré exactement. Les autres reposent sur un
échantillonnage de positions : c'est une déduction, pas une preuve.

### C. Atteignable par la lecture des sauvegardes (~12)

| Code | Donnée utilisée |
|---|---|
| `palworld_full_paldeck` | espèces capturées |
| `palworld_technology_complete` | technologies débloquées |
| `palworld_max_level` | niveau (redondant avec la REST API) |
| `palworld_perfect_breed` | passifs / statistiques d'un Pal |
| `palworld_passive_master` | combinaison de passifs |
| `palworld_rare_collection` | Pals rares possédés |
| `palworld_one_species_team` | composition de l'équipe |
| `palworld_full_team_bred` | origine des Pals de l'équipe |
| `palworld_pal_workforce` | Pals assignés à une base |
| `palworld_three_bases` | camps de base |
| `palworld_guild_legacy` | guilde et membres |
| `palworld_breed_chain` | lignée — ⚠️ seulement si la généalogie est stockée |

> Ces correspondances sont **plausibles, pas vérifiées** : elles supposent que
> les champs existent dans la sauvegarde de la version installée. La première
> étape serait un décodage exploratoire, pas du code d'attribution.

### D. Dérivés — calculables sans aucune source externe (~5)

Ceux-là ne dépendent que des hauts faits **déjà obtenus**. Ils sont donc
automatisables tout de suite, sans toucher au serveur de jeu :

| Code | Règle |
|---|---|
| `palworld_completionist` | toutes les catégories Palworld débloquées |
| `palworld_legendary_trainer` | au moins un haut fait de progression + élevage + exploration + combat |
| `palworld_community_legend` | N hauts faits légendaires |
| `discord_community_legend` | plusieurs hauts faits avancés de catégories différentes |
| `collector` | plusieurs hauts faits rares |

**C'est le meilleur rapport valeur / risque du lot** : la donnée est déjà en
base (`user_achievements`), il n'y a aucune dépendance externe, et rien ne peut
casser à la prochaine mise à jour du jeu.

### E. Non observables — validation humaine (~33)

Aucune source ne peut établir ces faits. Tous les hauts faits de combat en font
partie :

- `palworld_boss_no_down`, `palworld_boss_no_death`, `palworld_boss_under_time`,
  `palworld_boss_under_level`, `palworld_boss_single_element`,
  `palworld_boss_single_pal`, `palworld_coop_boss`, `palworld_coop_no_down` ;
- tout ce qui exige de savoir que le joueur **n'est pas mort** ou **n'a pas
  utilisé le voyage rapide** : `palworld_no_death_run`, `palworld_no_fast_travel`,
  `palworld_map_without_death`, `palworld_immortal_run` ;
- les événements de raid et de production dans la durée : `palworld_raid_proof`,
  `palworld_rebuild`, `palworld_mass_production`, `palworld_logistics_master`,
  `palworld_base_specialist`, `palworld_automated_base` ;
- les hauts faits sociaux : `palworld_newcomer_mentor`, `palworld_server_event`,
  `palworld_shared_base`, `palworld_rescue_team`, `palworld_server_supplier`.

Le jeu ne journalise pas ces faits, et aucune sauvegarde ne les contient : une
sauvegarde décrit un **état**, pas la manière dont on y est arrivé.

**La seule voie honnête est la validation humaine.** Ce qui peut être amélioré,
ce n'est pas la preuve — c'est le *parcours* : aujourd'hui, un administrateur
doit attribuer chaque haut fait à la main sans que le joueur puisse rien
demander.

## Récapitulatif

| Catégorie | Nombre | Dépendance | Risque |
|---|---|---|---|
| A. Présence RCON | 2 | déjà en place | — |
| B. REST API | ~5 (1 exact) | activer l'API serveur | faible |
| C. Sauvegardes | ~12 | outil tiers + accès volume | **élevé** |
| D. Dérivés | ~5 | aucune | **nul** |
| E. Validation humaine | ~33 | — | — |

## Ordre recommandé

1. **Les dérivés (D)** — aucun risque, aucune dépendance externe, effet
   immédiat. À faire en premier quel que soit le reste.
2. **Le parcours de validation (E)** — c'est ce qui débloque le plus gros
   volume (33 hauts faits) sans prétendre prouver l'improuvable. Concrètement :
   une commande `/haut-faits demander` qui dépose une demande avec preuve dans
   un salon staff, et deux boutons *Valider* / *Refuser* qui appellent la route
   d'attribution manuelle existante — laquelle trace déjà `granted_by`.
3. **La REST API (B)** — apporte `palworld_max_level` de façon exacte et pose
   les bases (niveau, position) pour d'éventuelles approximations. Il faut
   d'abord activer `REST_API_ENABLED` sur le template et vérifier les champs
   réellement renvoyés par la version installée.
4. **Les sauvegardes (C)** — le plus gros gain automatique, mais le plus
   fragile. À n'entreprendre qu'après un décodage exploratoire confirmant que
   les champs attendus existent, et en acceptant que ça casse à chaque grosse
   mise à jour du jeu.

## Ce qu'il ne faut pas faire

- **Déduire un fait de combat d'un signal qui ne l'établit pas** (par exemple
  attribuer « boss vaincu » parce que le niveau a augmenté). Le document
  `haut-faits.md` l'interdit explicitement, et une attribution fausse dévalue
  tous les hauts faits.
- **Basculer un haut fait en `auto` sans source qui le prouve.** Le champ
  `verification` est le garde-fou : tant qu'il vaut `manual`, un événement de
  jeu ne peut pas l'attribuer, même bien formé.
- **Donner au conteneur de jeu un accès à la base ou à Discord.** La collecte
  reste un adaptateur appelé par `platform-api`, jamais l'inverse.
