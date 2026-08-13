-- voice-bot — refonte du schema apres audit complet.
--
-- Probleme : nombreuses regressions accumulees.
--   1. Cles schema absentes du code : afk_action (enum), afk_timeout_secs,
--      max_channels_per_user, default_user_limit (lu depuis theme cache),
--      vote_kick_threshold_pct, public/private/game_creator_channel_id +
--      log_channel_id (env-only).
--   2. Cles consommees absentes du schema : afk_enabled, afk_move_owner,
--      voice_creation_cooldown_secs, voice_flood_max_messages,
--      voice_flood_time_window_secs, voice_vote_kick_timeout_secs.
--   3. Cles mal nommees : delete_empty_after_secs (code lit
--      voice_empty_cleanup_delay_secs), flood_mute_duration_secs (code
--      lit voice_flood_mute_duration_secs).
--
-- Solution : schema reecrit pour matcher exactement ce que le code lit.
-- Migration des valeurs existantes pour les renommages.

-- 1. Migration des valeurs : ancien nom -> nom prefixe voice_*
UPDATE bot_guild_config SET config_key = 'voice_empty_cleanup_delay_secs'
    WHERE bot_name = 'voice-bot' AND config_key = 'delete_empty_after_secs';
UPDATE bot_guild_config SET config_key = 'voice_flood_mute_duration_secs'
    WHERE bot_name = 'voice-bot' AND config_key = 'flood_mute_duration_secs';

-- 2. Schema reecrit (cles uniquement consommees par le code).
UPDATE bot_definitions SET
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active le systeme de salons vocaux temporaires (lobby create -> join -> salon perso)."},

        {"key": "afk_enabled", "label": "AFK sweep actif", "type": "boolean", "required": false, "default": "false", "description": "Tache periodique qui deplace/kick les membres AFK (self_mute + self_deaf trop longtemps).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "afk_timeout_minutes", "label": "Delai AFK", "type": "number", "required": false, "default": "10", "min": 1, "max": 1440, "unit": "min", "description": "Apres combien de minutes en self_mute + self_deaf un membre est considere AFK.", "depends_on": {"key": "afk_enabled", "equals": "true"}},
        {"key": "afk_channel_id", "label": "Salon AFK", "type": "channel", "required": false, "description": "Salon vocal ou les membres AFK sont deplaces.", "depends_on": {"key": "afk_enabled", "equals": "true"}},
        {"key": "afk_move_owner", "label": "Deplacer aussi les owners", "type": "boolean", "required": false, "default": "false", "description": "Si OFF, le proprietaire d un salon temporaire ne sera jamais deplace en AFK (evite que le salon se ferme).", "depends_on": {"key": "afk_enabled", "equals": "true"}},

        {"key": "voice_creation_cooldown_secs", "label": "Cooldown creation salon", "type": "number", "required": false, "default": "5", "min": 0, "max": 600, "unit": "s", "description": "Delai minimum entre 2 creations de salon par un meme user (anti-spam).", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "voice_empty_cleanup_delay_secs", "label": "Delai suppression salon vide", "type": "number", "required": false, "default": "2", "min": 0, "max": 60, "unit": "s", "description": "Anti-race : on attend N secondes avant de supprimer un salon vide (le owner peut revenir vite).", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "voice_flood_max_messages", "label": "Seuil flood (messages)", "type": "number", "required": false, "default": "5", "min": 1, "max": 50, "description": "Nombre de clics panel admin dans la fenetre avant mute auto.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "voice_flood_time_window_secs", "label": "Fenetre flood", "type": "number", "required": false, "default": "5", "min": 1, "max": 60, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "voice_flood_mute_duration_secs", "label": "Duree mute si flood", "type": "number", "required": false, "default": "30", "min": 30, "max": 3600, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "voice_vote_kick_timeout_secs", "label": "Duree vote-kick", "type": "number", "required": false, "default": "60", "min": 30, "max": 600, "unit": "s", "description": "Apres ce delai, le vote-kick expire automatiquement (sans verdict).", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'voice-bot';

-- 3. Cleanup des rows orphelines (cles maintenant supprimees du schema).
DELETE FROM bot_guild_config
 WHERE bot_name = 'voice-bot'
   AND config_key IN (
       'afk_action',
       'afk_timeout_secs',
       'max_channels_per_user',
       'default_user_limit',
       'vote_kick_threshold_pct',
       'public_creator_channel_id',
       'private_creator_channel_id',
       'game_creator_channel_id',
       'log_channel_id'
   );
