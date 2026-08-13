-- voice-bot — restauration de panel_post_enabled + log_channel_id.
--
-- Suite de l'audit post-fusion (cf. 244/245). La refonte 239 a egalement
-- perdu deux champs encore pertinents :
--
--   1. panel_post_enabled (boolean) : CONSOMME par le code
--      (channel_lifecycle.rs:185) mais absent du schema -> impossible de
--      desactiver la pose du panneau de controle depuis la page Composants.
--      (Avait deja ete restaure par mig 234 puis redrop par 239.)
--
--   2. log_channel_id (channel) : salon ou sont postees les cartes de
--      session vocale (creation/join/leave/close). Historiquement
--      configurable (mig 032). Le code ne le lisait plus que via l'env ;
--      embeds.rs lit desormais d'abord la config guild (puis fallback env).
--
-- Note : panel_sync_grace_secs (mig 177) n'est PAS restaure : purement
-- cosmetique et jamais consomme par le code.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "panel_post_enabled", "label": "Poster le panneau de controle dans le chat vocal", "type": "boolean", "required": false, "default": "true", "description": "Si OFF, aucun panneau de controle n est poste a la creation d un salon vocal temporaire.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'voice-bot'
  AND NOT (config_schema @> '[{"key": "panel_post_enabled"}]'::jsonb);

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "log_channel_id", "label": "Salon des logs vocaux", "type": "channel", "required": false, "description": "Salon textuel ou sont postees les cartes de session (creation, arrivees/departs, fermeture des salons vocaux temporaires). Vide = pas de logs.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'voice-bot'
  AND NOT (config_schema @> '[{"key": "log_channel_id"}]'::jsonb);
