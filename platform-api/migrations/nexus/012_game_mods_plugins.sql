-- 012_game_mods_plugins.sql
--
-- Mods, plugins et personnalisation poussee des serveurs de jeu.
--
-- Rien de tout cela ne demande de code : `itzg/minecraft-server` telecharge
-- et installe seul ce qu'on lui nomme en variables d'environnement. Le
-- schema de reglages etant deja pilote par la base, ajouter une option est
-- une ligne SQL — l'application n'a pas a savoir qu'elle existe.
--
-- Les groupes comptent : cinquante champs a plat sont inutilisables. Le
-- front construit ses sections a partir de la cle `group`.

-- ─────────────────────────────────────────────────────────────────────
-- Minecraft : moteur, mods et plugins
-- ─────────────────────────────────────────────────────────────────────
--
-- Le TYPE decide de ce que le serveur sait faire :
--   VANILLA — le jeu nu, aucun ajout possible
--   PAPER   — compatible plugins Bukkit/Spigot, et bien plus rapide
--   FABRIC  — mods legers, suit vite les nouvelles versions
--   FORGE   — gros modpacks
--
-- On ne peut pas melanger : un plugin Paper ne tourne pas sur Forge. C'est
-- pour cela que chaque champ d'installation depend du bon TYPE, sinon on
-- proposerait des combinaisons qui echouent au demarrage.

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "SPIGET_RESOURCES", "type": "text", "label": "Plugins Spigot (identifiants)",
   "group": "Mods et plugins", "required": false,
   "description": "Identifiants SpigotMC separes par virgule, par exemple 9089,34315. L''identifiant est le nombre dans l''URL de la page du plugin. Telecharges et mis a jour a chaque demarrage. Demande TYPE=PAPER ou SPIGOT."},

  {"key": "MODRINTH_PROJECTS", "type": "text", "label": "Mods et plugins Modrinth",
   "group": "Mods et plugins", "required": false,
   "description": "Noms de projets Modrinth separes par virgule, par exemple fabric-api,lithium. Fonctionne avec FABRIC, FORGE et PAPER — Modrinth choisit la bonne version tout seul."},

  {"key": "CURSEFORGE_FILES", "type": "text", "label": "Mods CurseForge",
   "group": "Mods et plugins", "required": false,
   "description": "Slugs ou identifiants de fichiers CurseForge separes par virgule. Demande une cle d''API CurseForge renseignee ci-dessous."},

  {"key": "CF_API_KEY", "type": "text", "label": "Cle d''API CurseForge",
   "group": "Mods et plugins", "required": false,
   "description": "Obligatoire pour telecharger depuis CurseForge, qui refuse les acces anonymes. Se cree gratuitement sur console.curseforge.com."},

  {"key": "MODPACK", "type": "text", "label": "Modpack (URL d''archive)",
   "group": "Mods et plugins", "required": false,
   "description": "URL directe d''un zip de modpack. Decompresse au demarrage. Pratique pour rejouer un pack precis sans lister ses mods un par un."},

  {"key": "REMOVE_OLD_MODS", "type": "boolean", "label": "Nettoyer les anciens mods au demarrage",
   "group": "Mods et plugins", "default": "false", "required": false,
   "description": "Vide le dossier des mods avant de reinstaller. A activer quand on change de liste : sans cela, les anciens restent et provoquent des conflits difficiles a comprendre."},

  {"key": "GENERIC_PACKS", "type": "text", "label": "Packs de donnees (URL)",
   "group": "Mods et plugins", "required": false,
   "description": "URLs de datapacks separees par virgule."}
]'::jsonb
WHERE slug = 'minecraft-vanilla';

-- Reglages de jeu supplementaires : ce qui change reellement une partie.
UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "LEVEL_TYPE", "type": "select", "label": "Type de monde",
   "group": "Monde", "default": "minecraft:normal", "required": false,
   "options": ["minecraft:normal", "minecraft:flat", "minecraft:large_biomes", "minecraft:amplified", "minecraft:single_biome_surface"],
   "description": "Plat pour construire, amplifie pour des reliefs extremes."},

  {"key": "SEED", "type": "text", "label": "Graine du monde",
   "group": "Monde", "required": false,
   "description": "Laisser vide pour un monde aleatoire. Une graine identique regenere exactement le meme terrain."},

  {"key": "GENERATE_STRUCTURES", "type": "boolean", "label": "Generer les structures",
   "group": "Monde", "default": "true", "required": false,
   "description": "Villages, temples, avant-postes. Desactive, le monde reste naturel mais vide."},

  {"key": "MAX_WORLD_SIZE", "type": "number", "label": "Rayon max du monde (blocs)",
   "group": "Monde", "default": "29999984", "min": 1000, "required": false,
   "description": "Borne l''exploration. Un monde plus petit reste plus leger a sauvegarder."},

  {"key": "SPAWN_ANIMALS", "type": "boolean", "label": "Animaux",
   "group": "Monde", "default": "true", "required": false},

  {"key": "SPAWN_MONSTERS", "type": "boolean", "label": "Monstres",
   "group": "Monde", "default": "true", "required": false},

  {"key": "SPAWN_NPCS", "type": "boolean", "label": "Villageois",
   "group": "Monde", "default": "true", "required": false},

  {"key": "ALLOW_NETHER", "type": "boolean", "label": "Nether accessible",
   "group": "Monde", "default": "true", "required": false},

  {"key": "ENABLE_COMMAND_BLOCK", "type": "boolean", "label": "Blocs de commande",
   "group": "Regles du jeu", "default": "false", "required": false},

  {"key": "FORCE_GAMEMODE", "type": "boolean", "label": "Imposer le mode de jeu",
   "group": "Regles du jeu", "default": "false", "required": false,
   "description": "Remet chaque joueur dans le mode par defaut a la connexion."},

  {"key": "PLAYER_IDLE_TIMEOUT", "type": "number", "label": "Deconnexion apres inactivite (min)",
   "group": "Regles du jeu", "default": "0", "min": 0, "max": 1440, "required": false,
   "description": "0 = jamais. Utile pour liberer des places sur un serveur frequente."},

  {"key": "SIMULATION_DISTANCE", "type": "number", "label": "Distance de simulation (tronçons)",
   "group": "Performance", "default": "10", "min": 3, "max": 32, "required": false,
   "description": "Le reglage qui pese le PLUS sur le processeur. La baisser a 6 ou 8 suffit presque toujours et change tout sur un serveur qui rame."},

  {"key": "MAX_TICK_TIME", "type": "number", "label": "Temps max par tick (ms)",
   "group": "Performance", "default": "60000", "min": -1, "required": false,
   "description": "-1 desactive le chien de garde. A mettre a -1 avec de gros modpacks, qui depassent legitimement ce delai au chargement."},

  {"key": "USE_AIKAR_FLAGS", "type": "boolean", "label": "Reglages memoire optimises (Aikar)",
   "group": "Performance", "default": "true", "required": false,
   "description": "Parametres de ramasse-miettes reconnus comme les meilleurs pour Minecraft. A laisser actif sauf raison precise."},

  {"key": "SPAWN_PROTECTION", "type": "number", "label": "Zone protegee autour du spawn (blocs)",
   "group": "Regles du jeu", "default": "16", "min": 0, "max": 1000, "required": false,
   "description": "0 pour laisser construire partout."}
]'::jsonb
WHERE slug = 'minecraft-vanilla';


-- ─────────────────────────────────────────────────────────────────────
-- Valheim, Terraria, Factorio, 7 Days to Die
-- ─────────────────────────────────────────────────────────────────────
--
-- Ces jeux n'avaient qu'un schema minimal. Les images utilisees acceptent
-- toutes des mods ; ce qui manquait, c'etait de l'exposer.

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "BEPINEX", "type": "boolean", "label": "Activer BepInEx (mods)",
   "group": "Mods", "default": "false", "required": false,
   "description": "Chargeur de mods de Valheim. Necessaire pour tout mod, et il doit etre installe AUSSI par chaque joueur."},

  {"key": "VALHEIM_PLUS", "type": "boolean", "label": "Activer Valheim Plus",
   "group": "Mods", "default": "false", "required": false,
   "description": "Modification tres repandue : vitesses de recolte, portails d''objets, confort de construction."},

  {"key": "SERVER_PUBLIC", "type": "boolean", "label": "Serveur visible publiquement",
   "group": "Serveur", "default": "false", "required": false},

  {"key": "UPDATE_INTERVAL", "type": "number", "label": "Verification des mises a jour (s)",
   "group": "Serveur", "default": "900", "min": 60, "required": false},

  {"key": "BACKUPS_INTERVAL", "type": "number", "label": "Intervalle de sauvegarde (s)",
   "group": "Sauvegardes", "default": "3600", "min": 300, "required": false},

  {"key": "BACKUPS_MAX_COUNT", "type": "number", "label": "Sauvegardes conservees",
   "group": "Sauvegardes", "default": "12", "min": 1, "max": 200, "required": false}
]'::jsonb
WHERE slug = 'valheim';

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "TSHOCK", "type": "boolean", "label": "Utiliser TShock (plugins)",
   "group": "Mods", "default": "false", "required": false,
   "description": "Serveur alternatif qui apporte les plugins, les permissions et des commandes d''administration."},

  {"key": "WORLD_SIZE", "type": "select", "label": "Taille du monde",
   "group": "Monde", "default": "2", "required": false, "options": ["1", "2", "3"],
   "description": "1 petit, 2 moyen, 3 grand."},

  {"key": "DIFFICULTY", "type": "select", "label": "Difficulte",
   "group": "Monde", "default": "0", "required": false, "options": ["0", "1", "2", "3"],
   "description": "0 classique, 1 expert, 2 maitre, 3 voyage."},

  {"key": "AUTOCREATE", "type": "boolean", "label": "Creer le monde automatiquement",
   "group": "Monde", "default": "true", "required": false},

  {"key": "SECURE", "type": "boolean", "label": "Anti-triche",
   "group": "Serveur", "default": "true", "required": false},

  {"key": "LANGUAGE", "type": "text", "label": "Langue du serveur",
   "group": "Serveur", "default": "fr-FR", "required": false}
]'::jsonb
WHERE slug = 'terraria';

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "MODS", "type": "text", "label": "Mods (noms Factorio)",
   "group": "Mods", "required": false,
   "description": "Noms de mods du portail officiel, separes par virgule. Telecharges au demarrage."},

  {"key": "PORT", "type": "number", "label": "Port UDP", "group": "Serveur",
   "default": "34197", "min": 1024, "max": 65535, "required": false},

  {"key": "SAVE_NAME", "type": "text", "label": "Nom de la partie",
   "group": "Monde", "default": "sentinel", "required": false},

  {"key": "AUTOSAVE_INTERVAL", "type": "number", "label": "Sauvegarde auto (min)",
   "group": "Sauvegardes", "default": "10", "min": 1, "max": 120, "required": false},

  {"key": "AUTOSAVE_SLOTS", "type": "number", "label": "Sauvegardes conservees",
   "group": "Sauvegardes", "default": "5", "min": 1, "max": 50, "required": false}
]'::jsonb
WHERE slug = 'factorio';

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "SERVERCONFIG_GameDifficulty", "type": "number", "label": "Difficulte",
   "group": "Monde", "default": "2", "min": 0, "max": 5, "required": false},

  {"key": "SERVERCONFIG_DayLightLength", "type": "number", "label": "Duree du jour (h)",
   "group": "Monde", "default": "18", "min": 1, "max": 24, "required": false},

  {"key": "SERVERCONFIG_ZombiesRun", "type": "select", "label": "Zombies courent",
   "group": "Monde", "default": "0", "required": false, "options": ["0", "1", "2", "3"],
   "description": "0 la nuit seulement, 1 jamais, 2 toujours, 3 sans jamais courir."},

  {"key": "SERVERCONFIG_BuildCreate", "type": "boolean", "label": "Mode creatif autorise",
   "group": "Regles du jeu", "default": "false", "required": false},

  {"key": "SERVERCONFIG_DropOnDeath", "type": "number", "label": "Objets perdus a la mort",
   "group": "Regles du jeu", "default": "1", "min": 0, "max": 4, "required": false,
   "description": "0 rien, 1 tout, 2 barre d''outils, 3 sac, 4 tout supprime."},

  {"key": "SERVERCONFIG_XPMultiplier", "type": "number", "label": "Multiplicateur d''experience (%)",
   "group": "Regles du jeu", "default": "100", "min": 1, "max": 1000, "required": false}
]'::jsonb
WHERE slug = '7dtd';
