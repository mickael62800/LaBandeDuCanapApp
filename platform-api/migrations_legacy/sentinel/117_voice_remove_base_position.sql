-- Migration 117 : voice-bot — retirer voice_base_position
--
-- voice_base_position (index numerique) est remplace par
-- voice_anchor_category_id (plus intuitif, pas besoin de calcul).
-- On retire le field du schema ET les valeurs existantes en DB.

-- 1. Retirer le field du config_schema (recalcule sans voice_base_position)
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM jsonb_array_elements(config_schema) elem
    WHERE elem->>'key' != 'voice_base_position'
)
WHERE bot_name = 'voice-bot';

-- 2. Nettoyer les valeurs existantes dans bot_guild_config
DELETE FROM bot_guild_config
WHERE bot_name = 'voice-bot' AND config_key = 'voice_base_position';
