-- Cleanup : monthly_report_enabled + monthly_report_channel_id sont
-- vestigials dans le schema analytics. Aucun code worker ne genere de
-- rapport mensuel, aucun code web ne lit ces cles. Supprime du schema
-- + des configs guild.

UPDATE bot_definitions
   SET config_schema = (
       SELECT jsonb_agg(entry)
         FROM jsonb_array_elements(config_schema) AS entry
        WHERE entry->>'key' NOT IN ('monthly_report_enabled', 'monthly_report_channel_id')
   )
 WHERE bot_name = 'analytics';

DELETE FROM bot_guild_config
 WHERE bot_name = 'analytics'
   AND config_key IN ('monthly_report_enabled', 'monthly_report_channel_id');
