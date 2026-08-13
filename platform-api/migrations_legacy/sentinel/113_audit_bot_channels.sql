-- Migration 113 : audit-bot — ajout des 4 salons dedies
--
-- Avant : un seul `log_channel_id` pour TOUT (rapport hebdo, anomalies,
-- events). Les anomalies et events standards n'etaient meme pas postes
-- dans Discord — seulement loggues en DB. Seul le rapport hebdo etait
-- effectivement envoye dans le salon.
--
-- Apres : 4 salons dedies pour router les events par type :
--   - join_leave_channel_id   : member_join, member_leave, ban, unban
--   - profile_edit_channel_id : nickname/avatar/roles/timeout update
--   - anomaly_channel_id      : alertes urgentes (mass_ban/kick/delete/role)
--   - weekly_report_channel_id: rapport hebdomadaire lundi matin
--
-- Fallback : si un de ces 4 fields est vide, le bot poste dans
-- `log_channel_id` (retrocompat). Si log_channel_id est aussi vide,
-- rien n'est poste.

UPDATE bot_definitions
SET config_schema = '[
    {"key": "enabled", "label": "Bot actif", "type": "boolean", "required": false, "default": "true"},
    {"key": "log_channel_id", "label": "Salon de logs (fallback general)", "type": "channel", "required": false, "default": ""},
    {"key": "join_leave_channel_id", "label": "Salon entrees/sorties (join, leave, ban, unban)", "type": "channel", "required": false, "default": ""},
    {"key": "profile_edit_channel_id", "label": "Salon modifications de profil (pseudo, avatar, roles, mute)", "type": "channel", "required": false, "default": ""},
    {"key": "anomaly_channel_id", "label": "Salon alertes d''urgence (mass ban/kick/delete/role)", "type": "channel", "required": false, "default": ""},
    {"key": "weekly_report_channel_id", "label": "Salon rapport hebdomadaire", "type": "channel", "required": false, "default": ""},
    {"key": "message_cache_size", "label": "Taille cache messages", "type": "number", "required": false, "default": "10000"},
    {"key": "anomaly_enabled", "label": "Detection d''anomalies", "type": "boolean", "required": false, "default": "true"},
    {"key": "anomaly_mass_ban_threshold", "label": "Seuil mass ban (en 60s)", "type": "number", "required": false, "default": "5"},
    {"key": "anomaly_mass_delete_threshold", "label": "Seuil mass delete (en 60s)", "type": "number", "required": false, "default": "20"},
    {"key": "anomaly_mass_role_threshold", "label": "Seuil mass role change (en 60s)", "type": "number", "required": false, "default": "10"},
    {"key": "weekly_report_enabled", "label": "Rapport hebdomadaire", "type": "boolean", "required": false, "default": "true"}
]'::jsonb
WHERE bot_name = 'audit-bot';
