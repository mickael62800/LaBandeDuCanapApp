-- Auto-nettoyage : frequence PAR salon (au lieu d'une frequence globale).
-- Remplace autopurge_channel_ids + autopurge_interval_hours par un unique champ
-- autopurge_schedule (type channel_schedule_list) : liste de {salon, heures}.

UPDATE bot_definitions SET
    config_schema = COALESCE(
        (
            SELECT jsonb_agg(entry)
            FROM jsonb_array_elements(config_schema) AS entry
            WHERE entry->>'key' NOT IN ('autopurge_channel_ids', 'autopurge_interval_hours')
        ),
        '[]'::jsonb
    ) || '[
        {"key": "autopurge_schedule", "type": "channel_schedule_list", "label": "Salons a auto-nettoyer (salon + frequence)", "required": false, "depends_on": {"key": "autopurge_enabled", "equals": "true"}, "description": "Chaque salon avec sa propre frequence (en heures). Les messages non epingles plus vieux que la periode de grace y sont supprimes."}
    ]'::jsonb
WHERE bot_name = 'cleanup'
  AND NOT (config_schema @> '[{"key": "autopurge_schedule"}]'::jsonb);
