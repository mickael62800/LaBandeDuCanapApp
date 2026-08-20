-- 055_satisfactory_et_ports_manquants.sql
--
-- Deux choses.
--
-- 1. Satisfactory entre au catalogue.
-- 2. ARK et 7 Days to Die recuperent les ports qui leur manquaient DEPUIS LE
--    DEBUT.
--
-- Le second point est une correction, pas une amelioration : ces deux jeux
-- n'ont jamais publie qu'une seule de leurs ouvertures. La migration 053 a
-- donne au catalogue le moyen de les declarer ; il restait a verifier chaque
-- fiche existante, ce qui est fait ici.
--
-- Etat verifie des sept jeux d'origine :
--
--   Minecraft, Terraria, Factorio  un seul port, rien a corriger
--   Valheim                        deja corrige par la migration 053
--   Palworld                       un seul port de JEU (voir plus bas)
--   ARK                            il manquait le port +1
--   7 Days to Die                  il manquait les trois ports UDP
--
-- Les ports de requete Steam (27015) ne sont volontairement declares nulle
-- part. Ils ne servent qu'a figurer dans le navigateur de serveurs public :
-- la plateforme, elle, communique une adresse et un port directs. Les publier
-- couterait un port hote par serveur pour une annonce qui serait de toute
-- facon fausse derriere la traduction d'adresses.
--
-- Aucun serveur existant ne change d'adresse : le port deja attribue est
-- conserve, et les voisins ne sont reserves qu'a la recreation du conteneur
-- (`reserve_block_at`). Si un voisin appartient a un autre serveur, celui-ci
-- est deplace sur un bloc entier plutot que de rester en erreur.


-- ─────────────────────────────────────────────────────────────────────
-- 1. ARK : le port +1 porte le trafic de jeu
-- ─────────────────────────────────────────────────────────────────────
--
-- ARK ecoute sur 7777 et sur 7778 (« raw socket »). Le second n'est pas un
-- accessoire : sans lui, une partie des connexions ne s'etablit pas. La fiche
-- ne publiait que 7777 depuis la migration 007.

UPDATE game_templates
SET extra_ports = '[{"offset": 1, "protocol": "udp"}]'::jsonb,
    updated_at = now()
WHERE slug = 'ark'
  AND extra_ports = '[]'::jsonb;


-- ─────────────────────────────────────────────────────────────────────
-- 2. 7 Days to Die : un port TCP et trois ports UDP
-- ─────────────────────────────────────────────────────────────────────
--
-- Le serveur reserve 26900 en TCP **et** en UDP, plus 26901 et 26902 en UDP.
-- La fiche ne publiait que le TCP : le decalage nul retablit le meme port en
-- UDP, les deux suivants completent le bloc.
--
-- Le jeu derive lui-meme ces ports de son port principal : aucune variable
-- supplementaire n'est necessaire.

UPDATE game_templates
SET extra_ports = '[{"offset": 0, "protocol": "udp"}, {"offset": 1, "protocol": "udp"}, {"offset": 2, "protocol": "udp"}]'::jsonb,
    updated_at = now()
WHERE slug = '7dtd'
  AND extra_ports = '[]'::jsonb;


-- ─────────────────────────────────────────────────────────────────────
-- 3. Satisfactory — wolveix/satisfactory-server
-- ─────────────────────────────────────────────────────────────────────
--
-- Trois ouvertures : 7777 en UDP et en TCP pour le jeu, plus un port de
-- messagerie que le client interroge. Ce dernier vaut 8888 par defaut, ce qui
-- le placerait a 1111 ports du port principal — un bloc de cette largeur ne
-- tient pas dans la plage allouee aux jeux. `SERVERMESSAGINGPORT` le ramene
-- donc a 7778, colle au port de jeu, et le bloc reste large de deux.
--
-- L'image demarre en root pour appliquer PUID/PGID a /config.
--
-- Memoire : 8 Go est le minimum vivable, 16 Go le confort sur une usine
-- avancee. Le plafond du modele est donc au maximum autorise par la
-- plateforme.

INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, extra_ports,
    default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods,
    idle_shutdown_days, cover_image_url, volume_path, run_as_root
) VALUES (
    'satisfactory',
    'Satisfactory',
    'Construction d''usines automatisees en monde ouvert, jusqu''a 4 joueurs. Gourmand : 12 Go conseilles sur une usine avancee.',
    'wolveix/satisfactory-server:latest',
    'Gestion',
    '🏭',
    'e08a1e',
    7777, 'udp',
    '[{"offset": 0, "protocol": "tcp"}, {"offset": 1, "protocol": "tcp"}]'::jsonb,
    12288, 8192, 16384,
    '{"MAXPLAYERS": "4", "SERVERGAMEPORT": "7777", "SERVERMESSAGINGPORT": "7778", "PUID": "1000", "PGID": "1000", "AUTOSAVENUM": "5", "MAXTICKRATE": "30", "SERVERSTREAMING": "true", "STEAMBETA": "false"}'::jsonb,
    '[
      {"key": "MAXPLAYERS", "type": "number", "label": "Joueurs maximum",
       "group": "Joueurs", "default": 4, "min": 1, "max": 16,
       "warning": "Au-dela de 4, le jeu n''offre aucune garantie : la simulation de l''usine est partagee et la memoire monte vite."},

      {"key": "AUTOSAVENUM", "type": "number", "label": "Sauvegardes automatiques conservees",
       "group": "Sauvegardes", "default": 5, "min": 1, "max": 20,
       "description": "Nombre de sauvegardes tournantes gardees sur le disque."},

      {"key": "MAXTICKRATE", "type": "number", "label": "Frequence de simulation maximale",
       "group": "Performances", "default": 30, "min": 5, "max": 120,
       "description": "Nombre de pas de simulation par seconde.",
       "warning": "Monter cette valeur augmente directement la charge processeur ; la baisser rend l''usine saccadee."},

      {"key": "SERVERSTREAMING", "type": "boolean", "label": "Chargement progressif des decors",
       "group": "Performances", "default": "true",
       "description": "Desactiver charge tout en memoire : plus fluide, nettement plus gourmand."},

      {"key": "DISABLESEASONALEVENTS", "type": "boolean", "label": "Desactiver les evenements saisonniers",
       "group": "Monde", "default": "false"},

      {"key": "TIMEOUT", "type": "number", "label": "Delai avant deconnexion (secondes)",
       "group": "Serveur", "default": 30, "min": 10, "max": 300},

      {"key": "STEAMBETA", "type": "boolean", "label": "Version experimentale",
       "group": "Serveur", "default": "false",
       "warning": "Impose la meme version experimentale a TOUS les joueurs, et une sauvegarde experimentale ne revient pas toujours en version stable."},

      {"key": "SKIPUPDATE", "type": "boolean", "label": "Ne pas mettre a jour au demarrage",
       "group": "Maintenance", "default": "false",
       "description": "Accelere le demarrage en gardant la version installee.",
       "warning": "Un serveur en retard sur la version des joueurs refuse leurs connexions."}
    ]'::jsonb,
    false, false,
    7,
    'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/526870/header.jpg',
    '/config',
    true
) ON CONFLICT (slug) DO NOTHING;


-- ─────────────────────────────────────────────────────────────────────
-- 4. Autorisation
-- ─────────────────────────────────────────────────────────────────────
--
-- Meme raison qu'en migration 054 : sans ajout a la whitelist de la guilde,
-- le jeu reste au catalogue sans pouvoir etre instancie.

UPDATE bot_guild_config AS c
SET config_value = c.config_value || ',satisfactory',
    updated_at = now()
WHERE c.bot_name = 'game-portal'
  AND c.config_key = 'allowed_templates'
  AND btrim(c.config_value) <> ''
  AND NOT EXISTS (
      SELECT 1
      FROM unnest(string_to_array(c.config_value, ',')) AS existant
      WHERE btrim(existant) = 'satisfactory'
  );
