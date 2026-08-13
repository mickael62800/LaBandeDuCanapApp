-- Automod — mention systematique du droit d'appel sur les messages de sanction.
--
--   sanction_appeal_enabled : si ON, chaque message de sanction adresse au
--   membre (avertissement / suppression / mute / ban, via automod ou review
--   1-clic) inclut un rappel qu'il peut contester la decision via /appeal
--   (conformite DSA). Gabarit de ton uniforme (shared::embeds::sanction_notice).
--
-- Idempotent.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'sanction_appeal_enabled'
        UNION ALL SELECT '{
            "key": "sanction_appeal_enabled",
            "label": "Mention du droit d''appel sur les messages de sanction",
            "type": "boolean",
            "required": false,
            "default": "true",
            "description": "Si ON, chaque message de sanction adresse au membre rappelle qu''il peut contester la decision via la commande /appeal (conformite DSA). Desactiver si le module d''appel n''est pas utilise."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'automod-bot';
