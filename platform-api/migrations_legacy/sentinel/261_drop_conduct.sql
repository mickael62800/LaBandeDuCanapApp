-- 261 — Suppression complete du systeme de points de conduite.
--
-- Nouvelle philosophie de moderation : plus de score / points / regen / propositions
-- automatiques de ban. La moderation se resume desormais a un simple listing des
-- infractions (warn / mute / ban / note + /history + dossier membre). Les tables
-- de points, leur config et leur journal sont supprimees.
--
-- Conserve : moderation_actions, infractions, user_strikes, user_notes, audit_logs.

-- 1) Suppression des tables de points de conduite.
DROP TABLE IF EXISTS conduct_points_log CASCADE;
DROP TABLE IF EXISTS user_conduct_points CASCADE;
DROP TABLE IF EXISTS conduct_config CASCADE;

-- 2) Nettoyage du config_schema du module moderation : retire les cles liees
--    aux points de conduite et au worker de sync des propositions de ban
--    (jobs supprimes cote sentinel-worker).
UPDATE bot_definitions
SET config_schema = (
    SELECT COALESCE(jsonb_agg(elem), '[]'::jsonb)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem->>'key' NOT LIKE 'conduct_%'
      AND elem->>'key' <> 'sync_ban_proposals_interval'
)
WHERE bot_name = 'moderation-bot'
  AND jsonb_typeof(config_schema) = 'array';

-- 3) Nettoyage des valeurs de config deja stockees pour ces cles supprimees.
DELETE FROM bot_guild_config
WHERE bot_name = 'moderation-bot'
  AND (config_key LIKE 'conduct_%' OR config_key = 'sync_ban_proposals_interval');
