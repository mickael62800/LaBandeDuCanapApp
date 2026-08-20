-- 054_nouveaux_jeux.sql
--
-- Six jeux entrent au catalogue : Core Keeper, Enshrouded, V Rising,
-- Project Zomboid, Necesse et Vintage Story.
--
-- Regles appliquees a chaque fiche, apprises des migrations 018 et 041-048 :
--
-- 1. Les noms de variables sont ceux de l'IMAGE, releves dans sa
--    documentation. Une variable inventee est acceptee par le conteneur, qui
--    l'ignore : le reglage s'affiche, se modifie, et ne commande rien.
-- 2. Un reglage dont le nom n'a pas pu etre confirme n'est PAS ecrit. La
--    plateforme refuse d'ailleurs toute cle absente de `config_schema`
--    (`GameTemplate::validate_config_value`), donc un jeu incomplet reste sur
--    ses defauts plutot que d'accepter n'importe quelle variable d'env.
-- 3. `supports_rcon` reste FAUX partout. Aucune de ces images ne suit la
--    convention `ENABLE_RCON` / `RCON_PASSWORD` des images Minecraft, ni le
--    format de reponse attendu par `presence::parse_players`. Activer RCON
--    donnerait une console muette et surtout un comptage a zero joueur — de
--    quoi faire eteindre par le worker un serveur ou des gens jouent.
-- 4. Le port du JEU dans le conteneur ne se regle pas depuis l'interface : il
--    doit rester celui que publie le mapping Docker. Le port cote joueurs est
--    celui de l'hote, alloue par la plateforme.
-- 5. `run_as_root` est vrai pour les images qui demarrent en root pour
--    s'approprier leur volume (PUID/PGID) ou qui tournent sous Wine.
--
-- Les couvertures Steam viennent du CDN deja utilise par les sept premiers
-- jeux, sur l'identifiant Steam de chaque titre.


-- ─────────────────────────────────────────────────────────────────────
-- Core Keeper — escaping/core-keeper-dedicated
-- ─────────────────────────────────────────────────────────────────────
--
-- Le serveur fonctionne par defaut a travers le relais Steam (SDR) et publie
-- un GAME_ID a partager. Renseigner SERVER_PORT bascule en connexion directe,
-- ce qui est le seul mode compatible avec un port publie par la plateforme :
-- il est donc fixe cote defaut et absent des reglages.
--
-- GAME_ID est laisse configurable : il permet de RETROUVER un monde deja
-- partage. Vide, l'image en genere un au demarrage.

INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, extra_ports,
    default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods,
    idle_shutdown_days, cover_image_url, volume_path, run_as_root
) VALUES (
    'core-keeper',
    'Core Keeper',
    'Survie et minage en cooperatif jusqu''a 10 joueurs, vue de dessus. Connexion directe par IP et port.',
    'escaping/core-keeper-dedicated:latest',
    'Survie',
    '🪨',
    '8a6d3b',
    27015, 'udp', '[]'::jsonb,
    4096, 2048, 8192,
    '{"WORLD_INDEX": "0", "WORLD_NAME": "Core Keeper Sentinel", "WORLD_MODE": "0", "MAX_PLAYERS": "10", "SERVER_PORT": "27015", "SERVER_IP": "0.0.0.0", "DATA_PATH": "/home/steam/core-keeper-data"}'::jsonb,
    '[
      {"key": "WORLD_NAME", "type": "text", "label": "Nom du serveur",
       "group": "Serveur", "default": "Core Keeper Sentinel", "max_length": 64},

      {"key": "MAX_PLAYERS", "type": "number", "label": "Joueurs maximum",
       "group": "Joueurs", "default": 10, "min": 1, "max": 100},

      {"key": "PASSWORD", "type": "text", "label": "Mot de passe", "group": "Acces",
       "default": "", "max_length": 28,
       "description": "Vide = l''image en genere un au premier demarrage, visible dans les logs du serveur."},

      {"key": "WORLD_INDEX", "type": "number", "label": "Emplacement de monde",
       "group": "Monde", "default": 0, "min": 0, "max": 9,
       "description": "Le serveur garde plusieurs mondes cote a cote. Changer d''emplacement charge un AUTRE monde.",
       "warning": "Un emplacement encore vide demarre une partie neuve. L''ancien monde reste sur le disque."},

      {"key": "WORLD_SEED", "type": "text", "label": "Graine du monde",
       "group": "Monde", "default": "",
       "description": "Vide = monde aleatoire. Ne sert qu''a la creation du monde.",
       "warning": "Sans effet sur un monde deja genere : il faut un emplacement vierge."},

      {"key": "WORLD_MODE", "type": "enum", "label": "Mode de jeu",
       "group": "Monde", "default": "0", "options": ["0", "1", "2", "4"],
       "description": "0 normal, 1 difficile, 2 creatif, 4 detendu."},

      {"key": "GAME_ID", "type": "text", "label": "Identifiant de partie",
       "group": "Serveur", "default": "",
       "description": "Vide = genere au demarrage. Renseigner un identifiant deja connu permet de le conserver entre deux recreations du serveur.",
       "warning": "15 a 28 caracteres alphanumeriques. Une valeur invalide est ignoree et remplacee par un identifiant genere."},

      {"key": "SEASON", "type": "enum", "label": "Evenement saisonnier",
       "group": "Monde", "default": "-1", "options": ["-1", "0", "1", "2", "3", "4", "5", "6", "7"],
       "description": "-1 laisse le jeu decider selon la date reelle."},

      {"key": "MODS_ENABLED", "type": "boolean", "label": "Activer les mods",
       "group": "Mods", "default": "false"},

      {"key": "MODS", "type": "text", "label": "Mods mod.io",
       "group": "Mods", "default": "",
       "description": "Identifiants separes par des virgules.",
       "warning": "Tous les joueurs doivent avoir exactement les memes mods, sinon la connexion est refusee."}
    ]'::jsonb,
    false, true,
    7,
    'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/1621690/header.jpg',
    '/home/steam/core-keeper-data',
    false
) ON CONFLICT (slug) DO NOTHING;


-- ─────────────────────────────────────────────────────────────────────
-- Enshrouded — mornedhels/enshrouded-server
-- ─────────────────────────────────────────────────────────────────────
--
-- L'image demarre en root pour s'approprier /opt/enshrouded selon PUID/PGID :
-- lui imposer --user l'empecherait d'ecrire son propre repertoire de jeu.
--
-- Seize joueurs est un plafond du JEU, pas un choix d'exploitation.

INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, extra_ports,
    default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods,
    idle_shutdown_days, cover_image_url, volume_path, run_as_root
) VALUES (
    'enshrouded',
    'Enshrouded',
    'Survie et construction en monde ouvert, jusqu''a 16 joueurs. Tres gourmand : 12 Go conseilles.',
    'mornedhels/enshrouded-server:latest',
    'Survie',
    '🌫️',
    '6b5b95',
    15637, 'udp', '[]'::jsonb,
    12288, 8192, 16384,
    '{"SERVER_NAME": "Enshrouded Sentinel", "SERVER_SLOT_COUNT": "16", "SERVER_PORT": "15637", "PUID": "1000", "PGID": "1000"}'::jsonb,
    '[
      {"key": "SERVER_NAME", "type": "text", "label": "Nom du serveur",
       "group": "Serveur", "default": "Enshrouded Sentinel", "max_length": 64},

      {"key": "SERVER_PASSWORD", "type": "text", "label": "Mot de passe", "group": "Acces",
       "default": "",
       "description": "Vide = serveur ouvert a tous."},

      {"key": "SERVER_SLOT_COUNT", "type": "number", "label": "Joueurs maximum",
       "group": "Joueurs", "default": 16, "min": 1, "max": 16,
       "warning": "Le jeu refuse toute valeur superieure a 16."},

      {"key": "UPDATE_CRON", "type": "text", "label": "Mise a jour automatique (cron)",
       "group": "Maintenance", "default": "",
       "description": "Expression cron, par exemple 0 4 * * *. Vide = aucune mise a jour automatique.",
       "warning": "Une mise a jour COUPE le serveur le temps du telechargement, joueurs connectes compris."},

      {"key": "BACKUP_CRON", "type": "text", "label": "Sauvegarde automatique (cron)",
       "group": "Sauvegardes", "default": "",
       "description": "Expression cron. Vide = aucune sauvegarde automatique."}
    ]'::jsonb,
    false, false,
    7,
    'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/1203620/header.jpg',
    '/opt/enshrouded',
    true
) ON CONFLICT (slug) DO NOTHING;


-- ─────────────────────────────────────────────────────────────────────
-- V Rising — trueosiris/vrising
-- ─────────────────────────────────────────────────────────────────────
--
-- Serveur Windows execute sous Wine : l'image tourne avec son propre
-- utilisateur, d'ou run_as_root.
--
-- Deux ports UDP consecutifs : 9876 (jeu) et 9877 (requete). Le second est
-- declare dans `extra_ports` — l'allocateur reserve donc deux ports hote.
--
-- L'image utilise deux volumes : les fichiers du serveur et les donnees
-- persistantes. La plateforme n'en monte qu'un, et c'est celui des SAUVEGARDES
-- qui est monte. Consequence assumee : recreer le conteneur retelecharge les
-- fichiers du serveur (plusieurs Go, plusieurs minutes) mais ne perd jamais le
-- monde.

INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, extra_ports,
    default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods,
    idle_shutdown_days, cover_image_url, volume_path, run_as_root
) VALUES (
    'vrising',
    'V Rising',
    'Survie vampirique en cooperatif ou PvP. Serveur Windows sous Wine : le premier demarrage peut prendre une dizaine de minutes.',
    'trueosiris/vrising:latest',
    'Survie',
    '🧛',
    '8b1a1a',
    9876, 'udp', '[{"offset": 1, "protocol": "udp"}]'::jsonb,
    6144, 4096, 12288,
    '{"SERVERNAME": "V Rising Sentinel", "WORLDNAME": "world1", "GAMEPORT": "9876", "QUERYPORT": "9877", "TZ": "Europe/Paris", "LOGDAYS": "30"}'::jsonb,
    '[
      {"key": "SERVERNAME", "type": "text", "label": "Nom du serveur",
       "group": "Serveur", "default": "V Rising Sentinel", "max_length": 64},

      {"key": "WORLDNAME", "type": "text", "label": "Nom du monde",
       "group": "Monde", "default": "world1", "max_length": 32,
       "description": "Nom du repertoire de sauvegarde.",
       "warning": "Changer ce nom demarre un monde VIERGE. L''ancien reste sur le volume et revient si l''on remet son nom."},

      {"key": "TZ", "type": "text", "label": "Fuseau horaire",
       "group": "Serveur", "default": "Europe/Paris",
       "description": "Determine l''heure des evenements planifies du serveur."},

      {"key": "LOGDAYS", "type": "number", "label": "Retention des journaux (jours)",
       "group": "Maintenance", "default": 30, "min": 1, "max": 365}
    ]'::jsonb,
    false, false,
    7,
    'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/1604030/header.jpg',
    '/mnt/vrising/persistentdata',
    true
) ON CONFLICT (slug) DO NOTHING;


-- ─────────────────────────────────────────────────────────────────────
-- Project Zomboid — renegademaster/zomboid-dedicated-server
-- ─────────────────────────────────────────────────────────────────────
--
-- Deux ports UDP consecutifs : 16261 (connexion) et 16262 (flux direct).
-- L'image tourne sans privilege sous son utilisateur `steam`.
--
-- L'image expose une console RCON, mais son format de reponse n'est pas celui
-- que sait lire la plateforme : `supports_rcon` reste faux (cf. entete).

INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, extra_ports,
    default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods,
    idle_shutdown_days, cover_image_url, volume_path, run_as_root
) VALUES (
    'project-zomboid',
    'Project Zomboid',
    'Survie zombie isometrique et sans pitie. Compte environ 4 Go de memoire pour 8 joueurs.',
    'renegademaster/zomboid-dedicated-server:latest',
    'Survie',
    '🧟‍♂️',
    '4a7c2f',
    16261, 'udp', '[{"offset": 1, "protocol": "udp"}]'::jsonb,
    4096, 3072, 8192,
    '{"SERVER_NAME": "ZomboidSentinel", "ADMIN_USERNAME": "superuser", "MAX_PLAYERS": "16", "DEFAULT_PORT": "16261", "MAP_NAMES": "Muldraugh, KY", "GAME_VERSION": "public"}'::jsonb,
    '[
      {"key": "SERVER_NAME", "type": "text", "label": "Nom du serveur",
       "group": "Serveur", "default": "ZomboidSentinel", "max_length": 64,
       "warning": "Ce nom designe aussi les fichiers de sauvegarde : en changer demarre une partie VIERGE."},

      {"key": "ADMIN_USERNAME", "type": "text", "label": "Compte administrateur",
       "group": "Acces", "default": "superuser", "max_length": 32},

      {"key": "ADMIN_PASSWORD", "type": "text", "label": "Mot de passe administrateur",
       "group": "Acces", "default": "",
       "description": "Vide = l''image en genere un et l''ecrit dans ses journaux au premier demarrage.",
       "warning": "Ce compte a tous les pouvoirs en jeu."},

      {"key": "MAX_PLAYERS", "type": "number", "label": "Joueurs maximum",
       "group": "Joueurs", "default": 16, "min": 1, "max": 64,
       "warning": "Chaque joueur ouvre une portion de carte simulee : la memoire monte vite au-dela d''une dizaine."},

      {"key": "MAP_NAMES", "type": "text", "label": "Cartes chargees",
       "group": "Monde", "default": "Muldraugh, KY",
       "description": "Liste ordonnee separee par des points-virgules. La carte de base se met en DERNIER.",
       "warning": "Modifier cette liste sur une partie en cours corrompt les zones deja explorees."},

      {"key": "GAME_VERSION", "type": "text", "label": "Branche du jeu",
       "group": "Serveur", "default": "public",
       "description": "public pour la version stable.",
       "warning": "Changer de branche impose la meme version a TOUS les joueurs."}
    ]'::jsonb,
    false, false,
    7,
    'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/108600/header.jpg',
    '/home/steam/Zomboid',
    false
) ON CONFLICT (slug) DO NOTHING;


-- ─────────────────────────────────────────────────────────────────────
-- Necesse — andreasgl4ser/necesse-server
-- ─────────────────────────────────────────────────────────────────────
--
-- Le plus leger du lot : un serveur Java qui tient dans 2 Go et supporte de
-- tourner en permanence. PAUSE_WHEN_EMPTY suspend meme la simulation quand
-- personne n'est connecte.

INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, extra_ports,
    default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods,
    idle_shutdown_days, cover_image_url, volume_path, run_as_root
) VALUES (
    'necesse',
    'Necesse',
    'Aventure et construction en vue de dessus, proche de Terraria. Leger : 2 Go suffisent.',
    'andreasgl4ser/necesse-server:latest',
    'Aventure',
    '🗺️',
    '2f7c9c',
    14159, 'udp', '[]'::jsonb,
    2048, 1024, 4096,
    '{"SERVER_PORT": "14159", "WORLD_NAME": "Sentinel", "SERVER_SLOTS": "10", "PAUSE_WHEN_EMPTY": "true", "PUID": "1000", "PGID": "1000"}'::jsonb,
    '[
      {"key": "WORLD_NAME", "type": "text", "label": "Nom du monde",
       "group": "Monde", "default": "Sentinel", "max_length": 32,
       "warning": "Un nom inconnu cree un monde VIERGE. L''ancien reste sur le volume."},

      {"key": "SERVER_SLOTS", "type": "number", "label": "Joueurs maximum",
       "group": "Joueurs", "default": 10, "min": 1, "max": 250},

      {"key": "SERVER_PASSWORD", "type": "text", "label": "Mot de passe", "group": "Acces",
       "default": "",
       "description": "Vide = serveur ouvert a tous."},

      {"key": "SERVER_MOTD", "type": "text", "label": "Message d''accueil",
       "group": "Serveur", "default": "", "max_length": 128},

      {"key": "SERVER_OWNER", "type": "text", "label": "Proprietaire",
       "group": "Acces", "default": "",
       "description": "Pseudo en jeu qui recoit les droits d''administration."},

      {"key": "PAUSE_WHEN_EMPTY", "type": "boolean", "label": "Mettre en pause si vide",
       "group": "Serveur", "default": "true",
       "description": "Suspend la simulation tant qu''aucun joueur n''est connecte, et donc la consommation processeur."},

      {"key": "GIVE_CLIENTS_POWER", "type": "boolean", "label": "Deleguer la simulation aux clients",
       "group": "Serveur", "default": "false",
       "description": "Soulage le serveur en confiant une part du calcul aux joueurs.",
       "warning": "Rend la triche nettement plus facile."},

      {"key": "ZIP_SAVES", "type": "boolean", "label": "Compresser les sauvegardes",
       "group": "Sauvegardes", "default": "true"},

      {"key": "ENABLE_LOGGING", "type": "boolean", "label": "Journalisation",
       "group": "Maintenance", "default": "true"},

      {"key": "SERVER_LANGUAGE", "type": "text", "label": "Langue du serveur",
       "group": "Serveur", "default": ""},

      {"key": "UPDATE_ON_START", "type": "boolean", "label": "Mettre a jour au demarrage",
       "group": "Maintenance", "default": "false",
       "warning": "Allonge le demarrage et impose aux joueurs la meme version."}
    ]'::jsonb,
    false, false,
    7,
    'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/1169040/header.jpg',
    '/home/necesse/.config/Necesse',
    false
) ON CONFLICT (slug) DO NOTHING;


-- ─────────────────────────────────────────────────────────────────────
-- Vintage Story — ghcr.io/darkmatterproductions/vintagestory
-- ─────────────────────────────────────────────────────────────────────
--
-- Le jeu ecoute sur 42420 en TCP ET en UDP. C'est le cas du decalage NUL :
-- meme port, second protocole. Le bloc de ports reserve reste donc large d'un
-- seul port.
--
-- Les reglages passent par des variables prefixees VS_CFG_, qui ecrasent le
-- fichier server-config.yaml au demarrage.

INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, extra_ports,
    default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods,
    idle_shutdown_days, cover_image_url, volume_path, run_as_root
) VALUES (
    'vintage-story',
    'Vintage Story',
    'Survie artisanale exigeante, tres modable. Sobre en ressources : 3 Go conviennent a une petite equipe.',
    'ghcr.io/darkmatterproductions/vintagestory:latest',
    'Survie',
    '🏺',
    'a0522d',
    42420, 'tcp', '[{"offset": 0, "protocol": "udp"}]'::jsonb,
    3072, 1024, 8192,
    '{"VS_CFG_SERVER_NAME": "Vintage Story Sentinel", "VS_CFG_MAX_CLIENTS": "16", "VS_CFG_ADVERTISE_SERVER": "false", "VS_RCON_ENABLED": "false"}'::jsonb,
    '[
      {"key": "VS_CFG_SERVER_NAME", "type": "text", "label": "Nom du serveur",
       "group": "Serveur", "default": "Vintage Story Sentinel", "max_length": 64},

      {"key": "VS_CFG_MAX_CLIENTS", "type": "number", "label": "Joueurs maximum",
       "group": "Joueurs", "default": 16, "min": 1, "max": 64},

      {"key": "VS_CFG_SERVER_PASSWORD", "type": "text", "label": "Mot de passe", "group": "Acces",
       "default": "",
       "description": "Vide = serveur ouvert a tous."},

      {"key": "VS_CFG_ADVERTISE_SERVER", "type": "boolean", "label": "Publier dans la liste publique",
       "group": "Acces", "default": "false",
       "warning": "Rend le serveur visible de tous les joueurs du jeu. A n''activer qu''avec un mot de passe."}
    ]'::jsonb,
    false, true,
    7,
    'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/1234100/header.jpg',
    '/vintagestory/data',
    false
) ON CONFLICT (slug) DO NOTHING;


-- ─────────────────────────────────────────────────────────────────────
-- Autorisation : sans whitelist, un jeu ajoute reste invisible
-- ─────────────────────────────────────────────────────────────────────
--
-- `allowed_templates` est une whitelist CSV par guilde (fail closed) : un slug
-- absent ne peut pas etre instancie, meme present au catalogue. Les guildes
-- qui ont deja enregistre ce reglage ne verraient donc jamais ces six jeux.
--
-- On AJOUTE les slugs manquants a la valeur existante, sans jamais retirer ni
-- reordonner ce qui s'y trouve : la liste est un choix d'administration.

UPDATE bot_guild_config AS c
SET config_value = (
    SELECT string_agg(slug, ',' ORDER BY ord)
    FROM (
        SELECT slug, ord FROM unnest(string_to_array(c.config_value, ',')) WITH ORDINALITY AS deja(slug, ord)
        UNION ALL
        SELECT nouveau, 1000000 + ord
        FROM unnest(ARRAY[
            'core-keeper', 'enshrouded', 'vrising',
            'project-zomboid', 'necesse', 'vintage-story'
        ]) WITH ORDINALITY AS ajouts(nouveau, ord)
        -- Comparaison sur la valeur nettoyee : la liste est saisie a la
        -- main et peut contenir des espaces autour des virgules.
        WHERE NOT EXISTS (
            SELECT 1
            FROM unnest(string_to_array(c.config_value, ',')) AS existant
            WHERE btrim(existant) = nouveau
        )
    ) AS fusion
),
    updated_at = now()
WHERE c.bot_name = 'game-portal'
  AND c.config_key = 'allowed_templates'
  AND btrim(c.config_value) <> '';
