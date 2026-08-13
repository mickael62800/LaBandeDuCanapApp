-- progression-bot — refonte du schema apres audit complet.
--
-- Probleme : 22 cles dans le schema, seulement 11 lues par le code.
-- Plusieurs cles ont des noms differents entre schema et code.
--
-- Migration des valeurs existantes pour les renames :
--   levelup_channel_id (schema) -> level_up_channel_id (code)
--
-- Drops (truly dead, no code reader, no UX value) :
--   - tracking_enabled (redondant avec enabled)
--   - leaderboard_default_size (jamais utilise)
--   - badges_enabled (pas de systeme de badges implemente)
--   - weekly_recap_enabled (pas de job recap)
--   - streak_bonus_xp (le bonus est deja gere via streak_mult)
--   - double_xp_roles (redondant avec xp_role_multipliers)
--
-- Adds :
--   - xp_role_mode (enum separate/max/total) — code lit deja cette cle
--   - default_role_ids (CSV) — code lit deja cette cle
--
-- Keep + wire (nouveau code dans on_message + level-up) :
--   - min_message_length (skip XP si message trop court)
--   - ignored_channels (skip XP dans ces salons)
--   - ignored_roles (skip XP pour les membres avec ces roles)
--   - levelup_announce_enabled (toggle annonce level-up dans channel)
--
-- Keep but TODO wire (UX features non-cablees aujourd'hui) :
--   - max_level (cap niveau)
--   - levelup_message (template custom)
--   - levelup_dm_enabled (DM lors du level-up)

-- 1. Rename row dans bot_guild_config
UPDATE bot_guild_config SET config_key = 'level_up_channel_id'
    WHERE bot_name = 'progression-bot' AND config_key = 'levelup_channel_id';

-- 2. Schema reecrit
UPDATE bot_definitions SET
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active le systeme XP / niveaux. Si OFF : aucun XP n est attribue."},

        {"key": "xp_per_message", "label": "XP par message", "type": "number", "required": false, "default": "15", "min": 0, "max": 1000, "unit": "XP", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "xp_cooldown_secs", "label": "Cooldown XP message", "type": "number", "required": false, "default": "60", "min": 0, "max": 3600, "unit": "s", "description": "Delai min entre 2 gains XP par message (anti-farm).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "xp_per_voice_minute", "label": "XP par minute en vocal", "type": "number", "required": false, "default": "5", "min": 0, "max": 100, "unit": "XP/min", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "min_message_length", "label": "Longueur min message", "type": "number", "required": false, "default": "3", "min": 0, "max": 200, "unit": "chars", "description": "Message plus court = pas d XP (anti-spam 1 lettre).", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "xp_channel_multipliers", "label": "Multiplicateurs XP par salon (CSV)", "type": "text", "required": false, "description": "Format : channel_id:mult,channel_id:mult (ex: 12345:2,67890:0.5).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "xp_role_multipliers", "label": "Multiplicateurs XP par role (CSV)", "type": "text", "required": false, "description": "Format : role_id:mult,role_id:mult.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "xp_role_mode", "label": "Mode attribution roles palier", "type": "enum", "required": false, "default": "separate", "options": [{"value": "separate", "label": "Separe (text/voice)"}, {"value": "max", "label": "Max(text, voice)"}, {"value": "total", "label": "Total (text+voice)"}], "description": "Comment le niveau est calcule pour les role rewards.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "default_role_ids", "label": "Roles attribues par defaut (CSV)", "type": "text", "required": false, "description": "Roles donnes a chaque nouveau membre. IDs separes par virgules.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "ignored_channels", "label": "Salons ignores (CSV)", "type": "text", "required": false, "description": "Aucun XP ne sera attribue dans ces salons. IDs separes par virgules.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "ignored_roles", "label": "Roles ignores (CSV)", "type": "text", "required": false, "description": "Membres avec ces roles ne gagneront pas d XP. IDs separes par virgules.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "streak_enabled", "label": "Streaks de connexion", "type": "boolean", "required": false, "default": "true", "description": "Tracke les jours consecutifs d activite + applique un multiplicateur XP croissant.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "level_up_channel_id", "label": "Salon annonce level-up", "type": "channel", "required": false, "description": "Si vide, l annonce est postee dans le salon courant (ou pas si annonce desactivee).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "levelup_announce_enabled", "label": "Annonce level-up dans le salon", "type": "boolean", "required": false, "default": "true", "description": "Si OFF, aucun message dans le salon (les role rewards restent appliques).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "levelup_dm_enabled", "label": "DM lors du level-up", "type": "boolean", "required": false, "default": "false", "description": "TODO : pas encore cable (envoi DM au membre lors du level-up).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "levelup_message", "label": "Message custom level-up", "type": "text", "required": false, "description": "TODO : template pas encore cable. Variables prevues : {user}, {level}.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "max_level", "label": "Niveau max (0 = illimite)", "type": "number", "required": false, "default": "0", "min": 0, "max": 1000, "description": "TODO : cap pas encore cable.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'progression-bot';

-- 3. Cleanup rows obsoletes
DELETE FROM bot_guild_config
 WHERE bot_name = 'progression-bot'
   AND config_key IN (
       'tracking_enabled',
       'leaderboard_default_size',
       'badges_enabled',
       'weekly_recap_enabled',
       'streak_bonus_xp',
       'double_xp_roles'
   );
