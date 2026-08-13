-- Nettoyage : retire du config_schema de security-bot des cles obsoletes,
-- lues par AUCUN code (verifie : 0 occurrence dans tous les crates .rs).
--
-- Contexte : le schema (mig 152) portait d'anciens noms d'avant une refonte
-- du module security. Le code lit desormais d'autres cles (raid_pattern_enabled,
-- min_account_age_secs, raid_pattern_score_threshold, slowmode_seconds, etc.),
-- toujours presentes dans le schema. Les anciennes ne faisaient plus rien :
--   log_channel_id        : security n'utilise pas de salon de log
--   alert_channel_id      : jamais lu
--   raid_detection_enabled: remplace par raid_pattern_enabled
--   captcha_role_id       : le code utilise quarantine_role_id
--   alt_min_account_age_days : remplace par min_account_age_secs
--   lockdown_role_id      : jamais lu
--   ban_threshold         : jamais lu
--
-- En plus, 5 cles n'etaient lues QUE depuis des variables d'env
-- (SecurityConfig::from_env), jamais depuis la config guild -> les editer dans
-- le web n'avait aucun effet par serveur. On les retire du schema web (elles
-- restent reglables via env). Concernees :
--   raid_join_threshold, raid_join_window_secs, alt_retention_secs,
--   alt_name_distance, lockdown_duration_secs
--
-- Idempotent : filtre puis re-agrege (rejouable sans effet de bord).

UPDATE bot_definitions
SET config_schema = (
    SELECT COALESCE(jsonb_agg(elem), '[]'::jsonb)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem->>'key' NOT IN (
        -- mortes (aucune lecture)
        'log_channel_id',
        'alert_channel_id',
        'raid_detection_enabled',
        'captcha_role_id',
        'alt_min_account_age_days',
        'lockdown_role_id',
        'ban_threshold',
        -- env-only (aucun effet par serveur via le web)
        'raid_join_threshold',
        'raid_join_window_secs',
        'alt_retention_secs',
        'alt_name_distance',
        'lockdown_duration_secs'
    )
)
WHERE bot_name = 'security-bot';

-- Purge des valeurs eventuellement enregistrees pour ces cles.
DELETE FROM bot_guild_config
WHERE bot_name = 'security-bot'
  AND config_key IN (
    'log_channel_id', 'alert_channel_id', 'raid_detection_enabled',
    'captcha_role_id', 'alt_min_account_age_days', 'lockdown_role_id', 'ban_threshold',
    'raid_join_threshold', 'raid_join_window_secs', 'alt_retention_secs',
    'alt_name_distance', 'lockdown_duration_secs'
  );
