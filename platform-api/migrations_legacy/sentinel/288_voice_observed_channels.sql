-- Ajoute le champ `observed_voice_channels` au schema de config voice-bot.
--
-- Permet de selectionner les vocaux PERMANENTS a observer pour les logs
-- (cartes de session). Les vocaux temporaires sont deja logues automatique-
-- ment a leur creation ; les permanents ne l'etaient pas faute de carte. Le
-- bot lit cette cle (IDs separes par virgule) et cree une carte paresseuse au
-- premier join d'un salon observe.
--
-- Idempotent : on n'ajoute le champ que s'il n'est pas deja present.
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "observed_voice_channels", "label": "Vocaux permanents a observer pour les logs (IDs separes par virgule)", "type": "text", "required": false, "default": ""}
]'::jsonb
WHERE bot_name = 'voice-bot'
  AND NOT (config_schema @> '[{"key": "observed_voice_channels"}]'::jsonb);
