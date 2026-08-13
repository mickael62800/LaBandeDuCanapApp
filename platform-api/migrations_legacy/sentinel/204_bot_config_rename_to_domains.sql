-- Phase 5 — Renommage des bot_name workers vers les noms de domaines.
--
-- Apres la fusion des 15 workers en `sentinel-worker`, les overrides
-- de config dans `bot_guild_config` etaient stockes avec les anciens
-- noms d'infrastructure (`coude-worker`, `cleanup-worker`, etc.). On
-- bascule vers les noms de **domaines metier** (qui matchent les
-- modules `sentinel-worker/src/domains/{domain}/`).
--
-- Avantages :
--   - Decouplage du nom de processus et du nom de configuration.
--   - Si on renomme un worker plus tard, les configs ne bougent pas.
--   - Liste claire et stable, alignee sur le code.
--
-- Idempotent : ON CONFLICT skip les rows deja renommees.

-- 15 anciens workers -> domaines (snake_case = nom de dossier).
UPDATE bot_guild_config SET bot_name = 'ai'
    WHERE bot_name = 'ai-worker';
UPDATE bot_guild_config SET bot_name = 'analytics'
    WHERE bot_name = 'analytics-worker';
UPDATE bot_guild_config SET bot_name = 'announcements'
    WHERE bot_name = 'announcement-worker';
UPDATE bot_guild_config SET bot_name = 'appeal_sla'
    WHERE bot_name = 'appeal-sla-worker';
UPDATE bot_guild_config SET bot_name = 'audit_cache'
    WHERE bot_name = 'audit-cache-worker';
UPDATE bot_guild_config SET bot_name = 'blackjack'
    WHERE bot_name = 'blackjack-cleanup-worker';
UPDATE bot_guild_config SET bot_name = 'cache'
    WHERE bot_name = 'cache-worker';
UPDATE bot_guild_config SET bot_name = 'cleanup'
    WHERE bot_name = 'cleanup-worker';
UPDATE bot_guild_config SET bot_name = 'coude'
    WHERE bot_name = 'coude-worker';
UPDATE bot_guild_config SET bot_name = 'discord_audit_sync'
    WHERE bot_name = 'discord-audit-sync-worker';
UPDATE bot_guild_config SET bot_name = 'export'
    WHERE bot_name = 'export-worker';
UPDATE bot_guild_config SET bot_name = 'game_portal'
    WHERE bot_name = 'game-portal-worker';
UPDATE bot_guild_config SET bot_name = 'moderation'
    WHERE bot_name = 'moderation-worker';
UPDATE bot_guild_config SET bot_name = 'monitoring'
    WHERE bot_name = 'monitoring-worker';
UPDATE bot_guild_config SET bot_name = 'temp_roles'
    WHERE bot_name = 'temp-roles-worker';

-- ticket-bot et security-bot : ces bots n'existent plus comme processus
-- separes (fusionnes dans sentinel-bot), mais leurs cles SLA / captcha
-- sont consommees par les jobs `tickets` et `security` du worker.
UPDATE bot_guild_config SET bot_name = 'tickets'
    WHERE bot_name = 'ticket-bot';
UPDATE bot_guild_config SET bot_name = 'security'
    WHERE bot_name = 'security-bot';
