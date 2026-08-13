-- Automod vote — nombre de messages de contexte (avant) sur la carte.
--
-- La carte de vote affiche les N messages precedant le message signale pour
-- donner du contexte aux moderateurs. Le contexte "apres" n'est pas capture
-- (il n'existe quasi jamais a la detection) : un bouton "Aller au message"
-- permet de voir la suite en live dans le salon.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "vote_context_before", "label": "Messages de contexte (avant)", "type": "number", "required": false, "default": "10", "min": 0, "max": 25, "unit": "messages", "description": "Nombre de messages precedant le message signale, affiches sur la carte de vote pour donner du contexte. 0 = desactive.", "depends_on": {"key": "vote_enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "vote_context_before"}]'::jsonb);
