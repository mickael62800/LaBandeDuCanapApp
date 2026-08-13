-- Audit-bot — salon de logs vocaux (connexions/deconnexions/deplacements, TOUS
-- les salons vocaux, pas seulement les temporaires). Entree = vert, sortie =
-- rouge, deplacement = bleu. Si vide, fallback sur log_channel_id.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'voice_log_channel_id'
        UNION ALL SELECT '{
            "key": "voice_log_channel_id",
            "label": "Salon de logs vocaux (connexion / deconnexion)",
            "type": "channel",
            "required": false,
            "default": "",
            "description": "Salon ou le bot poste un embed colore a chaque connexion (vert), deconnexion (rouge) ou deplacement (bleu) vocal, pour TOUS les salons vocaux. Si vide, utilise log_channel_id."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'audit-bot';
