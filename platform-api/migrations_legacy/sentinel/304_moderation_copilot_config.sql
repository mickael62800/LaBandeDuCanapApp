-- Copilote de moderation — cles de config par serveur (editables au dashboard).
--
-- Active et parametre la commande /copilote du moderation-bot : fenetre
-- d'historique et seuil de precedents pour la jurisprudence. La suggestion est
-- consultative (le bot n'applique jamais rien).
--
-- Idempotent (miroir de 302) : on retire toute entree existante avec ces cles
-- avant de les re-ajouter, donc rejouer la migration ne cree pas de doublon.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' NOT IN (
                'copilot_enabled',
                'copilot_lookback_days',
                'copilot_min_precedents'
            )
        UNION ALL SELECT '{
            "key": "copilot_enabled",
            "type": "boolean",
            "required": false,
            "default": "false",
            "label": "Copilote de modération",
            "description": "Active la commande /copilote (fiche membre + suggestion de sanction proportionnée basée sur l''historique et la jurisprudence du serveur).",
            "depends_on": {"key": "enabled", "equals": "true"}
        }'::jsonb
        UNION ALL SELECT '{
            "key": "copilot_lookback_days",
            "type": "number",
            "required": false,
            "default": "90",
            "min": 1,
            "max": 365,
            "unit": "jours",
            "label": "Copilote — fenêtre d''historique",
            "description": "Ancienneté max des précédents pris en compte.",
            "depends_on": {"key": "copilot_enabled", "equals": "true"}
        }'::jsonb
        UNION ALL SELECT '{
            "key": "copilot_min_precedents",
            "type": "number",
            "required": false,
            "default": "3",
            "min": 1,
            "max": 100,
            "label": "Copilote — précédents minimum",
            "description": "Nombre de cas similaires requis avant de suggérer sur la jurisprudence (sinon repli sur l''échelle d''escalade).",
            "depends_on": {"key": "copilot_enabled", "equals": "true"}
        }'::jsonb
    ) sub
)
WHERE bot_name = 'moderation-bot';
