-- Suppression complete de la fonctionnalite "role par niveau" (LevelReward).
-- Le niveau total (level_from_xp(xp_text + xp_voice)) est desormais utilise
-- uniquement pour le prefixe pseudo Discord ; plus aucun role n'est attribue
-- par palier de niveau.
--
-- Actions :
--   1. DROP TABLE level_rewards (avec ses index / contraintes en cascade).
--   2. Retire la cle xp_role_mode du config_schema de progression-bot.
--   3. Purge les rows xp_role_mode existantes dans bot_guild_config.

-- 1. Drop la table des recompenses de niveau.
DROP TABLE IF EXISTS level_rewards CASCADE;

-- 2. Retire xp_role_mode du schema progression-bot.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem->>'key' <> 'xp_role_mode'
)
WHERE bot_name = 'progression-bot';

-- 3. Purge les valeurs stockees par les guilds.
DELETE FROM bot_guild_config
 WHERE bot_name = 'progression-bot'
   AND config_key = 'xp_role_mode';
