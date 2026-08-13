-- Quota par moderateur : garde-fou anti-emballement / modo compromis.
-- Ajoute deux cles au schema de config du module moderation-bot (append jsonb,
-- pour ne pas reecrire tout le schema). Le bot bloque une action (ban/kick/
-- mute/warn) si le moderateur depasse `mod_quota_max` actions sur la fenetre
-- `mod_quota_window_secs`. mod_quota_max = 0 (defaut) = quota desactive.
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "mod_quota_max", "label": "Quota d actions par moderateur", "type": "number", "required": false, "default": "0", "min": 0, "max": 1000, "unit": "actions", "description": "Nombre max d actions (ban/kick/mute/warn) qu un moderateur peut poser sur la fenetre. 0 = illimite (desactive)."},
    {"key": "mod_quota_window_secs", "label": "Fenetre du quota", "type": "number", "required": false, "default": "3600", "min": 60, "max": 86400, "unit": "s", "description": "Duree de la fenetre glissante du quota, en secondes (defaut 3600 = 1h).", "depends_on": {"key": "mod_quota_max", "not_equals": "0"}}
]'::jsonb
WHERE bot_name = 'moderation-bot'
  -- Idempotent : ne pas dupliquer si deja present.
  AND NOT (config_schema @> '[{"key": "mod_quota_max"}]'::jsonb);
