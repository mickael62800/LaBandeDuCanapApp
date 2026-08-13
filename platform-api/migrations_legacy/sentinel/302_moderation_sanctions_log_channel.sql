-- Moderation-bot — salon dedie aux "cards de sanction".
--
-- Le bot poste une card compacte (embed 2 lignes, colore par type de sanction)
-- a chaque sanction appliquee : warn / mute / ban / kick + auto-automod.
-- Cle dediee (PAS de reutilisation de log_channel_id). Vide = desactive.
--
-- Idempotent : on retire toute entree existante avec cette cle avant de la
-- re-ajouter, donc rejouer la migration ne cree pas de doublon.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'sanctions_log_channel_id'
        UNION ALL SELECT '{
            "key": "sanctions_log_channel_id",
            "label": "Salon des cards de sanction",
            "type": "channel",
            "required": false,
            "description": "Salon ou une card 2 lignes confirme chaque sanction appliquee (warn/mute/ban/kick + auto-automod). Vide = desactive.",
            "depends_on": {"key": "enabled", "equals": "true"}
        }'::jsonb
    ) sub
)
WHERE bot_name = 'moderation-bot';
