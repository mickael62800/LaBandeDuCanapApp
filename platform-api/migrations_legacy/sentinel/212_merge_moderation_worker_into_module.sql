-- Phase composants — Fusion `moderation-worker` dans `moderation-bot`.
--
-- moderation-bot : 10 cles metier (sanctions manuelles, templates, appeals).
-- moderation-worker : 12 cles infra (intervals scan, escalation auto,
--   regen conduite, ban cleanup, sync propositions ban).
--
-- Cascade depends_on :
--   enabled
--   ├─ log_channel_id, appeal_channel_id, dm_on_sanction, templates_enabled,
--   │  review_required_for, auto_archive_appeals_days
--   ├─ default_warn_gravity, default_mute_duration_secs, default_ban_duration_secs
--   ├─ conduct_regen_interval
--   │   ├─ conduct_regen_amount
--   │   └─ conduct_regen_max
--   ├─ ban_cleanup_interval
--   ├─ sync_ban_proposals_interval
--   └─ auto_escalation_enabled
--       ├─ escalation_warn_to_mute
--       │   └─ default_temp_mute_duration_secs
--       ├─ escalation_mute_to_ban
--       │   └─ default_temp_ban_duration_secs
--       └─ notification_channel_id

-- 1) Migration 204 a renomme moderation-worker -> moderation. On
-- restaure vers moderation-bot, en supprimant d'abord les doublons.
DELETE FROM bot_guild_config wkr
    WHERE wkr.bot_name = 'moderation'
      AND EXISTS (
          SELECT 1 FROM bot_guild_config m
           WHERE m.bot_name = 'moderation-bot'
             AND m.guild_id = wkr.guild_id
             AND m.config_key = wkr.config_key
      );
UPDATE bot_guild_config SET bot_name = 'moderation-bot'
    WHERE bot_name = 'moderation';

-- 2) Schema fusionne avec cascade depends_on.
UPDATE bot_definitions SET
    display_name = 'Moderation',
    description = 'Sanctions manuelles (warn / mute / ban / kick), templates de raisons, appels, escalation automatique selon historique, regeneration des points de conduite, nettoyage des bans expires. Les jobs periodiques tournent dans sentinel-worker.',
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active toutes les fonctionnalites moderation (commandes /ban, /mute, /warn, /kick et jobs worker)."},

        {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false, "description": "Salon ou sont logguees les sanctions appliquees.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "appeal_channel_id", "label": "Salon d''appels", "type": "channel", "required": false, "description": "Salon ou les utilisateurs peuvent faire appel d une sanction.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "dm_on_sanction", "label": "DM lors de sanction", "type": "boolean", "required": false, "default": "true", "description": "Envoie un DM au membre sanctionne avec le motif et la duree.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "templates_enabled", "label": "Templates de raisons", "type": "boolean", "required": false, "default": "true", "description": "Active les templates de raisons rapides (/template apply).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "review_required_for", "label": "Actions en review obligatoire", "type": "text", "required": false, "description": "Liste CSV des actions necessitant review avant application (ex: ban,mute,kick).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "auto_archive_appeals_days", "label": "Archivage auto des appels", "type": "number", "required": false, "default": "30", "min": 1, "max": 365, "unit": "j", "description": "Apres combien de jours un appel non resolu est archive automatiquement.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "default_warn_gravity", "label": "Gravite par defaut (warn)", "type": "enum", "required": false, "default": "medium", "options": [{"value": "low", "label": "Faible"}, {"value": "medium", "label": "Moyenne"}, {"value": "high", "label": "Haute"}], "description": "Gravite par defaut quand un mod fait /warn sans la specifier.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "default_mute_duration_secs", "label": "Duree mute par defaut", "type": "number", "required": false, "default": "3600", "min": 60, "max": 2419200, "unit": "s", "description": "Duree du mute si le mod ne specifie pas de duree dans /mute. Max Discord : 28 jours.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "default_ban_duration_secs", "label": "Duree ban par defaut", "type": "number", "required": false, "default": "0", "min": 0, "max": 31536000, "unit": "s", "description": "Duree d un ban temporaire par defaut. 0 = ban permanent.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "conduct_regen_interval", "label": "Worker : intervalle regen conduite", "type": "number", "required": false, "default": "24", "min": 1, "max": 168, "unit": "h", "description": "Frequence de regeneration des points de conduite. Recommande : 24 (1 fois par jour).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "conduct_regen_amount", "label": "Points regeneres par cycle", "type": "number", "required": false, "default": "5", "min": 1, "max": 100, "unit": "pts", "description": "Combien de points de conduite sont regeneres par cycle.", "depends_on": {"key": "conduct_regen_interval", "equals": ""}},
        {"key": "conduct_regen_max", "label": "Plafond score conduite", "type": "number", "required": false, "default": "100", "min": 10, "max": 1000, "unit": "pts", "description": "Plafond du score de conduite (pas de regen au-dela).", "depends_on": {"key": "conduct_regen_interval", "equals": ""}},

        {"key": "ban_cleanup_interval", "label": "Worker : intervalle scan bans expires", "type": "number", "required": false, "default": "1", "min": 1, "max": 60, "unit": "min", "description": "Frequence du scan des bans expires pour les lever automatiquement. Recommande : 1.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "sync_ban_proposals_interval", "label": "Worker : intervalle sync propositions ban", "type": "number", "required": false, "default": "2", "min": 1, "max": 30, "unit": "min", "description": "Frequence de sync des propositions de ban en attente.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "auto_escalation_enabled", "label": "Escalation automatique", "type": "boolean", "required": false, "default": "false", "description": "Si ON, le bot escalade automatiquement (warn -> mute -> ban) selon les seuils ci-dessous.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "escalation_warn_to_mute", "label": "Seuil warns -> mute auto", "type": "number", "required": false, "default": "3", "min": 0, "max": 20, "unit": "warns", "description": "Nombre de warns actifs avant qu un nouveau warn declenche un mute auto. 0 = desactive.", "depends_on": {"key": "auto_escalation_enabled", "equals": "true"}},
        {"key": "default_temp_mute_duration_secs", "label": "Duree mute escalation", "type": "number", "required": false, "default": "3600", "min": 60, "max": 2419200, "unit": "s", "description": "Duree du mute auto declenche par escalation. Max Discord : 28 jours.", "depends_on": {"key": "auto_escalation_enabled", "equals": "true"}},
        {"key": "escalation_mute_to_ban", "label": "Seuil mutes -> ban auto", "type": "number", "required": false, "default": "3", "min": 0, "max": 20, "unit": "mutes", "description": "Nombre de mutes actifs avant qu un nouveau mute declenche un ban auto. 0 = desactive.", "depends_on": {"key": "auto_escalation_enabled", "equals": "true"}},
        {"key": "default_temp_ban_duration_secs", "label": "Duree ban escalation", "type": "number", "required": false, "default": "86400", "min": 3600, "max": 31536000, "unit": "s", "description": "Duree du ban auto declenche par escalation. Defaut : 86400 (1 jour).", "depends_on": {"key": "auto_escalation_enabled", "equals": "true"}},
        {"key": "notification_channel_id", "label": "Salon notifications escalation", "type": "channel", "required": false, "description": "Salon ou les escalations auto sont notifiees.", "depends_on": {"key": "auto_escalation_enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'moderation-bot';

-- 3) Supprime la definition worker.
DELETE FROM bot_definitions WHERE bot_name = 'moderation-worker';
