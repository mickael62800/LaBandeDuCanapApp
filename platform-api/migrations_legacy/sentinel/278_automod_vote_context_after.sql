-- Automod — nombre de messages de contexte APRÈS l'infraction dans le salon
-- de discussion ("Ouvrir une discussion"). Complète vote_context_before.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'vote_context_after'
        UNION ALL SELECT '{
            "key": "vote_context_after",
            "label": "Messages de contexte APRÈS l''infraction (salon de discussion)",
            "type": "number",
            "required": false,
            "default": "10",
            "description": "Nombre de messages postés APRÈS la dernière infraction à afficher dans le message d''ancrage du salon de discussion (0 = aucun)."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'automod-bot';
