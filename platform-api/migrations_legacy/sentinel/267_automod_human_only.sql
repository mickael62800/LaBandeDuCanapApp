-- Automod — mode "modération 100% humaine".
--
-- Si human_only_enabled, le bot n'applique JAMAIS de sanction automatiquement :
-- toute detection actionnable (texte IA, flood, fichier suspect) passe par une
-- carte de review/vote dans le salon de review, et un humain decide. Sans salon
-- de review configure, aucune sanction n'est appliquee (rien d'automatique).
-- Couvre aussi le fallback "backend injoignable" (pas de suppression auto).

-- Idempotent : on retire d'abord la cle si presente, puis on la (re)ajoute.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'human_only_enabled'
        UNION ALL SELECT '{
            "key": "human_only_enabled",
            "label": "Modération 100% humaine (aucune sanction auto)",
            "type": "boolean",
            "required": false,
            "default": "false",
            "description": "Si ON, aucune sanction n''est appliquee automatiquement : chaque detection genere une carte que les moderateurs traitent (vote + finalisation). Necessite un salon de review configure."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'automod-bot';
