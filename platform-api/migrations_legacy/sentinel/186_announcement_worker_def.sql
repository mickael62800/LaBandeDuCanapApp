-- Ajoute la definition du worker announcement-worker dans bot_definitions
-- pour qu'il apparaisse dans la page Composants (/component-config) sous
-- la section Workers avec ses parametres configurables.
--
-- Le worker poll /api/announcements/internal/due chaque heure pile UTC,
-- publie sur Redis stream sentinel:events event="announcement_publish"
-- consume par le bot pour poster sur Discord.

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'announcement-worker',
    'Worker Annonces planifiees',
    'Polle l''API toutes les heures pile et publie les annonces dues sur Redis stream pour le bot Discord.',
    '[
        {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true"},
        {"key": "fetch_limit", "label": "Nombre max d''annonces fetchees par tick", "type": "number", "required": false, "default": "50"},
        {"key": "log_channel_id", "label": "Salon de logs (succes / erreurs)", "type": "channel", "required": false}
    ]'
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
