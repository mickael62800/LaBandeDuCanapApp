-- 018_memoire_16go_et_7dtd.sql
--
-- Trois choses.
--
-- 1. Plafond memoire porte a 16 Go pour tous les jeux.
-- 2. Reglages complets de 7 Days to Die.
-- 3. Correction d'une cle inventee dans la migration 012.


-- ─────────────────────────────────────────────────────────────────────
-- 1. Memoire : jusqu'a 16 Go par serveur
-- ─────────────────────────────────────────────────────────────────────
--
-- Les plafonds dataient d'avant les mods. Un modpack Minecraft consequent
-- ou 7 Days to Die avec des mods depassent largement 8 Go, et le plafond
-- refusait la creation sans rapport avec la machine reelle.
--
-- Le minimum ne bouge pas : c'est lui qui empeche de creer un serveur
-- condamne a ramer.
--
-- Rappel utile : le CONTENEUR recoit un quart de plus que cette valeur
-- (cf. `container_memory_mb`). Un serveur a 16 Go occupe donc 20 Go de
-- memoire machine.

UPDATE game_templates SET max_memory_mb = 16384 WHERE max_memory_mb < 16384;

-- Le quota de guilde plafonnait a 8 Go CUMULES : un seul serveur a 16 Go
-- aurait ete refuse malgre un plafond individuel releve. Les deux vont
-- ensemble.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'key' = 'max_memory_total_mb'
             THEN elem || '{"default": "32768"}'::jsonb
             ELSE elem END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE bot_name = 'game-portal'
  AND config_schema @> '[{"key": "max_memory_total_mb"}]'::jsonb;


-- ─────────────────────────────────────────────────────────────────────
-- 2. Cle inventee : ZombiesRun n'existe pas
-- ─────────────────────────────────────────────────────────────────────
--
-- J'ai ecrit `SERVERCONFIG_ZombiesRun` en 012. Le nom reel de l'option est
-- `ZombieMove` : le reglage etait donc affiche, modifiable, et sans le
-- moindre effet sur le jeu. Il est retire ici et remplace plus bas par les
-- quatre vrais reglages de deplacement des zombies — le jeu distingue le
-- jour, la nuit, les feraux et la lune de sang.

UPDATE game_templates
SET config_schema = (
    SELECT COALESCE(jsonb_agg(elem ORDER BY ord), '[]'::jsonb)
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
    WHERE elem ->> 'key' <> 'SERVERCONFIG_ZombiesRun'
)
WHERE slug = '7dtd';

-- Les serveurs deja crees peuvent porter cette cle dans leurs surcharges :
-- elle serait transmise au conteneur, qui l'ignore. Inoffensive, mais elle
-- reapparaitrait dans le formulaire sans rien commander.
DELETE FROM game_server_configs
WHERE config_key = 'SERVERCONFIG_ZombiesRun';


-- ─────────────────────────────────────────────────────────────────────
-- 3. 7 Days to Die : reglages complets
-- ─────────────────────────────────────────────────────────────────────
--
-- L'image `vinanrra/7dtd-server` transmet toute variable `SERVERCONFIG_<X>`
-- a la propriete `<X>` de serverconfig.xml. Les noms sont donc ceux du jeu,
-- a la casse pres — une faute de frappe donne un reglage silencieusement
-- inerte, comme ZombiesRun ci-dessus.

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "SERVERCONFIG_ServerName", "type": "text", "label": "Nom du serveur",
   "group": "Serveur", "default": "Sentinel", "required": false},

  {"key": "SERVERCONFIG_ServerDescription", "type": "text", "label": "Description",
   "group": "Serveur", "required": false},

  {"key": "SERVERCONFIG_ServerPassword", "type": "text", "label": "Mot de passe",
   "group": "Serveur", "required": false,
   "description": "Vide = serveur ouvert a tous."},

  {"key": "SERVERCONFIG_ServerMaxPlayerCount", "type": "number", "label": "Joueurs maximum",
   "group": "Serveur", "default": "8", "min": 1, "max": 64, "required": false,
   "warning": "7 Days to Die consomme beaucoup par joueur : compte environ 1 Go par tranche de 4, en plus du serveur."},

  {"key": "SERVERCONFIG_ServerVisibility", "type": "enum", "label": "Visibilite",
   "group": "Serveur", "default": "2", "required": false, "options": ["0", "1", "2"],
   "description": "0 invisible, 1 amis seulement, 2 public."},

  {"key": "SERVERCONFIG_EACEnabled", "type": "boolean", "label": "Anti-triche EAC",
   "group": "Serveur", "default": "true", "required": false,
   "warning": "A desactiver OBLIGATOIREMENT pour jouer avec des mods : EAC les refuse et bloque la connexion."},

  {"key": "SERVERCONFIG_GameWorld", "type": "enum", "label": "Monde",
   "group": "Monde", "default": "Navezgane", "required": false,
   "options": ["Navezgane", "RWG"],
   "description": "Navezgane est la carte dessinee a la main ; RWG genere un monde aleatoire.",
   "warning": "Changer de monde demarre une partie VIERGE. L''ancienne reste sur le disque mais n''est plus chargee."},

  {"key": "SERVERCONFIG_WorldGenSeed", "type": "text", "label": "Graine du monde",
   "group": "Monde", "required": false,
   "description": "Ne sert qu''avec RWG. Une graine identique regenere le meme monde.",
   "warning": "La modifier regenere le monde entierement."},

  {"key": "SERVERCONFIG_WorldGenSize", "type": "enum", "label": "Taille du monde",
   "group": "Monde", "default": "8192", "required": false,
   "options": ["4096", "6144", "8192", "10240", "12288", "16384"],
   "description": "En metres. Ne sert qu''avec RWG.",
   "warning": "Au-dela de 10240, la generation prend de longues minutes et la memoire necessaire grimpe fortement."},

  {"key": "SERVERCONFIG_GameMode", "type": "enum", "label": "Mode de jeu",
   "group": "Monde", "default": "GameModeSurvival", "required": false,
   "options": ["GameModeSurvival"]},

  {"key": "SERVERCONFIG_DayNightLength", "type": "number", "label": "Duree d''une journee (min)",
   "group": "Monde", "default": "60", "min": 10, "max": 180, "required": false,
   "description": "Duree reelle d''un cycle complet."},

  {"key": "SERVERCONFIG_DayCount", "type": "number", "label": "Jour de depart",
   "group": "Monde", "default": "1", "min": 1, "required": false},

  {"key": "SERVERCONFIG_ZombieMove", "type": "enum", "label": "Deplacement des zombies — jour",
   "group": "Zombies", "default": "0", "required": false, "options": ["0", "1", "2", "3", "4"],
   "description": "0 marche, 1 trottine, 2 court, 3 sprinte, 4 aleatoire."},

  {"key": "SERVERCONFIG_ZombieMoveNight", "type": "enum", "label": "Deplacement des zombies — nuit",
   "group": "Zombies", "default": "3", "required": false, "options": ["0", "1", "2", "3", "4"],
   "description": "Par defaut ils sprintent la nuit. C''est ce qui rend les nuits redoutables."},

  {"key": "SERVERCONFIG_ZombieFeralMove", "type": "enum", "label": "Deplacement des feraux",
   "group": "Zombies", "default": "3", "required": false, "options": ["0", "1", "2", "3", "4"]},

  {"key": "SERVERCONFIG_ZombieBMMove", "type": "enum", "label": "Deplacement — lune de sang",
   "group": "Zombies", "default": "3", "required": false, "options": ["0", "1", "2", "3", "4"]},

  {"key": "SERVERCONFIG_EnemyDifficulty", "type": "enum", "label": "Force des ennemis",
   "group": "Zombies", "default": "0", "required": false, "options": ["0", "1"],
   "description": "0 normal, 1 feral."},

  {"key": "SERVERCONFIG_MaxSpawnedZombies", "type": "number", "label": "Zombies simultanes maximum",
   "group": "Zombies", "default": "64", "min": 8, "max": 256, "required": false,
   "warning": "C''est le reglage qui pese le PLUS sur le processeur. Au-dela de 90, un serveur modeste s''effondre pendant les lunes de sang."},

  {"key": "SERVERCONFIG_MaxSpawnedAnimals", "type": "number", "label": "Animaux simultanes maximum",
   "group": "Zombies", "default": "50", "min": 0, "max": 200, "required": false},

  {"key": "SERVERCONFIG_BloodMoonFrequency", "type": "number", "label": "Lune de sang tous les N jours",
   "group": "Lune de sang", "default": "7", "min": 0, "max": 60, "required": false,
   "description": "0 desactive completement les lunes de sang."},

  {"key": "SERVERCONFIG_BloodMoonRange", "type": "number", "label": "Variation aleatoire (jours)",
   "group": "Lune de sang", "default": "0", "min": 0, "max": 10, "required": false,
   "description": "0 = toujours pile le meme jour. Au-dela, la date varie et la surprise revient."},

  {"key": "SERVERCONFIG_BloodMoonWarning", "type": "number", "label": "Heure d''annonce",
   "group": "Lune de sang", "default": "8", "min": -1, "max": 23, "required": false,
   "description": "-1 = aucune annonce."},

  {"key": "SERVERCONFIG_BloodMoonEnemyCount", "type": "number", "label": "Zombies par joueur",
   "group": "Lune de sang", "default": "8", "min": 0, "max": 64, "required": false,
   "warning": "Se multiplie par le nombre de joueurs connectes. A huit joueurs, 8 ici fait 64 zombies d''un coup."},

  {"key": "SERVERCONFIG_LootAbundance", "type": "number", "label": "Abondance du butin (%)",
   "group": "Regles du jeu", "default": "100", "min": 25, "max": 600, "required": false},

  {"key": "SERVERCONFIG_LootRespawnDays", "type": "number", "label": "Reapparition du butin (jours)",
   "group": "Regles du jeu", "default": "30", "min": 1, "max": 365, "required": false},

  {"key": "SERVERCONFIG_AirDropFrequency", "type": "number", "label": "Largage aerien (heures)",
   "group": "Regles du jeu", "default": "72", "min": 0, "max": 999, "required": false,
   "description": "0 desactive les largages."},

  {"key": "SERVERCONFIG_AirDropMarker", "type": "boolean", "label": "Marqueur sur la carte",
   "group": "Regles du jeu", "default": "true", "required": false},

  {"key": "SERVERCONFIG_PlayerKillingMode", "type": "enum", "label": "Joueur contre joueur",
   "group": "Regles du jeu", "default": "0", "required": false, "options": ["0", "1", "2", "3"],
   "description": "0 interdit, 1 allies seulement, 2 inconnus seulement, 3 tout le monde.",
   "warning": "Au-dela de 0, les joueurs peuvent s''entretuer et se depouiller."},

  {"key": "SERVERCONFIG_DropOnQuit", "type": "number", "label": "Objets perdus a la deconnexion",
   "group": "Regles du jeu", "default": "0", "min": 0, "max": 4, "required": false,
   "description": "0 rien, 1 tout, 2 barre d''outils, 3 sac, 4 tout supprime."},

  {"key": "SERVERCONFIG_PlayerSafeZoneLevel", "type": "number", "label": "Zone sure jusqu''au niveau",
   "group": "Regles du jeu", "default": "5", "min": 0, "max": 100, "required": false,
   "description": "Protege les nouveaux venus a leur arrivee."},

  {"key": "SERVERCONFIG_PlayerSafeZoneHours", "type": "number", "label": "Duree de la zone sure (h)",
   "group": "Regles du jeu", "default": "5", "min": 0, "max": 48, "required": false},

  {"key": "SERVERCONFIG_LandClaimCount", "type": "number", "label": "Revendications par joueur",
   "group": "Territoires", "default": "3", "min": 1, "max": 50, "required": false},

  {"key": "SERVERCONFIG_LandClaimSize", "type": "number", "label": "Taille d''une revendication",
   "group": "Territoires", "default": "41", "min": 1, "max": 255, "required": false,
   "description": "En blocs, de cote."},

  {"key": "SERVERCONFIG_LandClaimDeadZone", "type": "number", "label": "Distance entre territoires",
   "group": "Territoires", "default": "30", "min": 0, "max": 255, "required": false,
   "description": "Ecart minimum entre les revendications de joueurs differents."},

  {"key": "SERVERCONFIG_LandClaimExpiryTime", "type": "number", "label": "Expiration (jours)",
   "group": "Territoires", "default": "7", "min": 1, "max": 365, "required": false,
   "warning": "Passe ce delai sans connexion, la base d''un joueur redevient destructible par les autres."},

  {"key": "SERVERCONFIG_BedrollDeadZoneSize", "type": "number", "label": "Zone sans zombies autour du lit",
   "group": "Territoires", "default": "15", "min": 0, "max": 100, "required": false},

  {"key": "SERVERCONFIG_MaxUncoveredMapChunksPerPlayer", "type": "number", "label": "Carte revelee maximum",
   "group": "Performance", "default": "131072", "min": 1000, "required": false,
   "description": "Limite la memoire prise par la carte de chaque joueur."},

  {"key": "SERVERCONFIG_PersistentPlayerProfiles", "type": "boolean", "label": "Profils persistants",
   "group": "Performance", "default": "false", "required": false,
   "description": "Conserve le personnage meme si le joueur ne s''est pas connecte depuis longtemps."},

  {"key": "SERVERCONFIG_ServerDisabledNetworkProtocols", "type": "text", "label": "Protocoles reseau desactives",
   "group": "Performance", "default": "SteamNetworking", "required": false,
   "description": "Laisser tel quel sauf probleme de connexion avere."},

  {"key": "MODS", "type": "text", "label": "Mods (URLs)",
   "group": "Mods", "required": false,
   "description": "URLs directes d''archives de mods, separees par virgule. Installees au demarrage.",
   "warning": "Chaque joueur doit installer les MEMES mods, et l''anti-triche EAC doit etre desactive — sinon personne ne peut se connecter."}
]'::jsonb
WHERE slug = '7dtd';
