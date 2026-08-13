-- Migration 052 : Ajoute les cles de config pour les features avancees de l'image-bot
-- (hash cache, seuils par salon, file d'attente)

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "hash_cache_enabled", "label": "Cache hash images (evite doublons)", "type": "boolean", "required": false, "default": "true"},
  {"key": "hash_cache_ttl_secs", "label": "TTL du cache hash (secondes)", "type": "number", "required": false, "default": "300"},
  {"key": "channel_thresholds", "label": "Seuils par salon (channel_id:seuil par ligne)", "type": "text", "required": false, "default": ""},
  {"key": "queue_enabled", "label": "File d attente (au lieu de suppression preventive)", "type": "boolean", "required": false, "default": "false"},
  {"key": "queue_max_retries", "label": "Max retries file d attente", "type": "number", "required": false, "default": "3"}
]'::jsonb
WHERE bot_name = 'image-bot';
