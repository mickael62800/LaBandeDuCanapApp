-- Phase composants — Fusions/normalisations restantes :
--   1. ai-worker -> standalone 'ai' (pas de module bot)
--   2. export-worker -> standalone 'export' (pas de module bot)
--   3. appeal-sla-worker -> standalone 'appeal_sla' (sera mergé dans
--      ticket-bot ulterieurement, en attendant on normalise)
--   4. discord-audit-sync-worker -> standalone 'discord_audit_sync'
--   5. security-bot et ticket-bot : restaurer les rows dans
--      bot_guild_config (mig 204 les avait renommees vers
--      security/tickets pour le worker, mais le code Rust attend
--      toujours security-bot et ticket-bot pour les configs metier).

-- ── 1. ai ──────────────────────────────────────────────────────────
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'ai',
    'Workers IA (texte + vision)',
    'Traite les jobs IA en arriere-plan : analyse texte (toxicite, spam, hate speech) et vision (NSFW, violence). Les bots publient des jobs dans la file ai_jobs, le worker les depile et appelle les modeles ONNX.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Si OFF : aucun job IA n est traite. Les bots qui publient des jobs verront leur file s accumuler."},
        {"key": "ai_poll_interval", "label": "Intervalle polling jobs IA", "type": "number", "required": false, "default": "5", "min": 1, "max": 60, "unit": "s", "description": "Frequence de depilage de la file ai_jobs.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "ai_job_timeout", "label": "Timeout job IA", "type": "number", "required": false, "default": "60", "min": 5, "max": 600, "unit": "s", "description": "Duree max d un job IA avant de le marquer failed.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
DELETE FROM bot_definitions WHERE bot_name = 'ai-worker';

-- ── 2. export ──────────────────────────────────────────────────────
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'export',
    'Export de donnees',
    'Traite les demandes d export RGPD / sauvegarde de donnees Discord. Les requetes web sont mises en file export_jobs, le worker les depile et genere un fichier ZIP downloadable.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Si OFF : les demandes d export restent en attente."},
        {"key": "export_scan_interval", "label": "Intervalle depilage file export", "type": "number", "required": false, "default": "5", "min": 1, "max": 300, "unit": "s", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
DELETE FROM bot_definitions WHERE bot_name = 'export-worker';

-- ── 3. appeal_sla (en attendant fusion dans ticket-bot) ────────────
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'appeal_sla',
    'SLA des appels',
    'Escalade les tickets d appel de sanction qui depassent le SLA configure (alerte des superviseurs).',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true"},
        {"key": "appeal_sla_scan_interval", "label": "Intervalle scan SLA", "type": "number", "required": false, "default": "300", "min": 30, "max": 3600, "unit": "s", "description": "Frequence de scan des tickets d appel pour detection de depassement SLA.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
DELETE FROM bot_definitions WHERE bot_name = 'appeal-sla-worker';

-- ── 4. discord_audit_sync ──────────────────────────────────────────
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'discord_audit_sync',
    'Sync audit logs Discord',
    'Synchronise periodiquement les audit logs Discord (bans, deletes, role changes) via l API REST pour rattraper les events qu on aurait rates en gateway (offline, rate limit).',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true"},
        {"key": "audit_sync_interval", "label": "Intervalle sync", "type": "number", "required": false, "default": "300", "min": 60, "max": 3600, "unit": "s", "description": "Frequence de polling des audit logs Discord. Plus court = plus reactif mais consomme du rate limit.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
DELETE FROM bot_definitions WHERE bot_name = 'discord-audit-sync-worker';

-- ── 5. security-bot / ticket-bot : restaure les bot_name (le code
-- Rust des bots utilise toujours security-bot et ticket-bot, mig 204
-- les avait basculees vers security/tickets pour le worker — on
-- remet sous le nom du module et on bascule WORKER_MODULES cote
-- worker pour matcher).
DELETE FROM bot_guild_config wkr
    WHERE wkr.bot_name = 'security'
      AND EXISTS (
          SELECT 1 FROM bot_guild_config m
           WHERE m.bot_name = 'security-bot'
             AND m.guild_id = wkr.guild_id
             AND m.config_key = wkr.config_key
      );
UPDATE bot_guild_config SET bot_name = 'security-bot'
    WHERE bot_name = 'security';

DELETE FROM bot_guild_config wkr
    WHERE wkr.bot_name = 'tickets'
      AND EXISTS (
          SELECT 1 FROM bot_guild_config m
           WHERE m.bot_name = 'ticket-bot'
             AND m.guild_id = wkr.guild_id
             AND m.config_key = wkr.config_key
      );
UPDATE bot_guild_config SET bot_name = 'ticket-bot'
    WHERE bot_name = 'tickets';
