-- AutoMod : mode qui confie l'analyse comportementale au moteur IA texte.
-- Les protections critiques (phishing/fichiers) restent des chemins locaux.
UPDATE bot_definitions
SET config_schema = config_schema || '[
  {
    "key": "ai_only_enabled",
    "type": "boolean",
    "label": "Modération IA texte uniquement",
    "default": "false",
    "required": false,
    "depends_on": {"key": "text_enabled", "equals": "true"},
    "description": "Désactive les détecteurs comportementaux locaux (insultes, spam, liens, mentions, majuscules) : seules les classifications IA texte alimentent le score. Le phishing et les fichiers dangereux restent protégés localement."
  }
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key":"ai_only_enabled"}]'::jsonb);
