-- Permet d'utiliser DeepSeek seul sans charger/executer le classifieur local.
UPDATE bot_definitions
SET config_schema = config_schema || '[
  {
    "key": "local_onnx_enabled",
    "type": "boolean",
    "label": "Analyse locale ONNX/tokenizer",
    "default": "true",
    "required": false,
    "depends_on": {"key": "text_enabled", "equals": "true"},
    "description": "Exécute le modèle local ONNX et son tokenizer en complément de DeepSeek. Désactive-le pour utiliser uniquement l IA distante et réduire la charge du serveur."
  }
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key":"local_onnx_enabled"}]'::jsonb);
