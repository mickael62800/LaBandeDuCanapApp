-- Cleanup : auto_escalation_* + default_temp_* sont vestigials.
-- Aucun code (worker, bot, API) ne lit ces cles. Pas d'escalation auto
-- implementee aujourd'hui. Notification channel inutile sans le job.
--
-- A reimplementer le jour ou on cable l'escalation (warn->mute->ban
-- selon historique des sanctions).

UPDATE bot_definitions
   SET config_schema = (
       SELECT jsonb_agg(entry)
         FROM jsonb_array_elements(config_schema) AS entry
        WHERE entry->>'key' NOT IN (
            'auto_escalation_enabled',
            'escalation_warn_to_mute',
            'escalation_mute_to_ban',
            'default_temp_ban_duration_secs',
            'default_temp_mute_duration_secs',
            'notification_channel_id'
        )
   )
 WHERE bot_name = 'moderation-bot';

DELETE FROM bot_guild_config
 WHERE bot_name = 'moderation-bot'
   AND config_key IN (
       'auto_escalation_enabled',
       'escalation_warn_to_mute',
       'escalation_mute_to_ban',
       'default_temp_ban_duration_secs',
       'default_temp_mute_duration_secs',
       'notification_channel_id'
   );

-- Game Portal : auto_restart_on_crash + max_auto_restart_attempts +
-- notify_on_* sont dead config (aucun reader). A reimplementer si on
-- decide de cabler le watchdog crash + les notifs Discord par event.

UPDATE bot_definitions
   SET config_schema = (
       SELECT jsonb_agg(entry)
         FROM jsonb_array_elements(config_schema) AS entry
        WHERE entry->>'key' NOT IN (
            'auto_restart_on_crash',
            'max_auto_restart_attempts',
            'notify_on_crash',
            'notify_on_idle_shutdown',
            'notify_on_player_join'
        )
   )
 WHERE bot_name = 'game-portal';

DELETE FROM bot_guild_config
 WHERE bot_name = 'game-portal'
   AND config_key IN (
       'auto_restart_on_crash',
       'max_auto_restart_attempts',
       'notify_on_crash',
       'notify_on_idle_shutdown',
       'notify_on_player_join'
   );
