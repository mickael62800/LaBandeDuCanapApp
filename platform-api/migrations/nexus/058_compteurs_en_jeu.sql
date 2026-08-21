-- 058_compteurs_en_jeu.sql
--
-- Deux salons compteurs pour Nexus, sur le modele de ceux de l'accueil
-- (« Membres : 128 », « En vocal : 4 ») : un salon dont le NOM porte le
-- chiffre, mis a jour periodiquement par le bot.
--
--   joueurs   — combien de personnes sont en jeu
--   serveurs  — combien de serveurs tournent
--
-- POURQUOI DEUX. Le nombre de joueurs vient de la console RCON du jeu, et la
-- plateforme ne sait lire que celle de Minecraft et de Palworld : ailleurs,
-- `last_player_count` vaut zero, faute d'un format de reponse connu (voir
-- `presence::parse_players`). Un compteur de joueurs seul afficherait donc
-- « 0 en jeu » pendant qu'une soiree Valheim bat son plein. Le compteur de
-- SERVEURS, lui, ne depend d'aucune console : il reste vrai pour les quatorze
-- jeux, et dit au moins que quelque chose tourne.
--
-- SALON VIDE = COMPTEUR ETEINT. Pas de booleen separe : un interrupteur
-- « active » sans salon designe ne produit rien et laisse croire le contraire.
--
-- Le format porte `{count}`, remplace par le chiffre. Sans ce marqueur, le nom
-- du salon serait fixe — le bot n'ecrirait jamais, ce que le reglage annonce
-- explicitement.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "players_counter_channel_id", "type": "channel",
   "label": "Salon compteur : joueurs en jeu",
   "required": false,
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "Salon vocal dont le nom affiche le nombre de personnes en jeu. Vide : compteur eteint.",
   "warning": "Le comptage repose sur la console du jeu. Aujourd''hui seuls Minecraft et Palworld la rendent lisible : les autres jeux comptent zero joueur, meme peuples. Le compteur de serveurs, lui, reste juste."},

  {"key": "players_counter_format", "type": "text",
   "label": "Format du compteur de joueurs",
   "default": "🎮 En jeu : {count}", "max_length": 90, "required": false,
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "{count} est remplace par le nombre de joueurs.",
   "warning": "Un format sans {count} donne un nom fixe : le compteur n''affiche plus rien de vivant."},

  {"key": "servers_counter_channel_id", "type": "channel",
   "label": "Salon compteur : serveurs allumes",
   "required": false,
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "Salon vocal dont le nom affiche le nombre de serveurs de jeu en ligne. Vide : compteur eteint."},

  {"key": "servers_counter_format", "type": "text",
   "label": "Format du compteur de serveurs",
   "default": "🖥️ Serveurs actifs : {count}", "max_length": 90, "required": false,
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "{count} est remplace par le nombre de serveurs en ligne."}
]'::jsonb
WHERE bot_name = 'game-portal'
  AND NOT jsonb_path_exists(config_schema, '$[*] ? (@.key == "players_counter_channel_id")');
