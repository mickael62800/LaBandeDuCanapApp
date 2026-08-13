-- Phase composants — Ajoute le toggle `enabled` au module `announcements`
-- + dependances `depends_on` pour griser les sous-options quand `enabled`
-- est OFF.
--
-- Sans toggle principal, l'utilisateur n'avait aucun moyen de "couper"
-- proprement le module — il devait jouer avec `max_announcements_per_guild`
-- ou supprimer manuellement chaque annonce. Maintenant : un seul switch.
--
-- Le format `depends_on: {key, equals}` est interprete par
-- `ComponentConfigForm.isFieldDisabled` pour appliquer un look grise +
-- pointer-events:none. Le worker `announcements` consume `enabled` via
-- `is_worker_enabled(pool, guild_id, "announcements")`.

UPDATE bot_definitions
SET config_schema = '[
    {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active la publication automatique des annonces planifiees pour ce serveur."},
    {"key": "default_color_hex", "label": "Couleur par defaut (embed)", "type": "text", "required": false, "default": "#5865f2", "description": "Couleur d''accent pour les embeds, en hex (ex: #5865f2). Surchargeable par annonce.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "max_announcements_per_guild", "label": "Nombre max d''annonces par serveur", "type": "number", "required": false, "default": "100", "min": 1, "max": 1000, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "default_mention_everyone", "label": "Activer @everyone par defaut", "type": "boolean", "required": false, "default": "false", "description": "Si actif, les nouvelles annonces ont @everyone coche par defaut.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "history_retention_days", "label": "Retention historique (jours)", "type": "number", "required": false, "default": "90", "unit": "jours", "min": 7, "max": 365, "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false, "description": "Salon ou poster les events publication / erreurs (optionnel).", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "fetch_limit", "label": "Annonces fetchees par tick worker", "type": "number", "required": false, "default": "50", "min": 1, "max": 500, "description": "Nombre max d''annonces traitees par le worker a chaque heure pile. Ne touche pas sauf si tu vois des delais de publication.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'announcements';
