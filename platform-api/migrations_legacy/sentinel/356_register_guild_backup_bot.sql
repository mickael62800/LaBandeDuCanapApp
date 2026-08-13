-- Composant "Sauvegarde serveur" (guild-backup-bot) : capture/restauration de
-- la structure d'un serveur (roles, salons, settings...). Enregistre dans
-- bot_definitions pour apparaitre dans la page Composants avec sa config.
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'guild-backup-bot',
    'Sauvegarde serveur',
    'Capture et restauration de la structure complete d un serveur (roles, salons, categories, parametres, bans, emojis). Les captures sont declenchees depuis le web ; le bot execute la capture/restauration sur Discord.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active la sauvegarde/restauration du serveur."},
        {"key": "snapshot_quota", "label": "Quota de sauvegardes", "type": "number", "required": false, "default": "10", "min": 1, "max": 100, "unit": "snapshots", "description": "Nombre max de sauvegardes conservees par serveur (les plus anciennes sont evincees).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "auto_backup_enabled", "label": "Sauvegarde automatique", "type": "boolean", "required": false, "default": "false", "description": "Capture automatiquement le serveur a intervalle regulier.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "auto_backup_interval_hours", "label": "Intervalle de sauvegarde auto", "type": "number", "required": false, "default": "24", "min": 1, "max": 168, "unit": "h", "description": "Delai entre deux captures automatiques.", "depends_on": {"key": "auto_backup_enabled", "equals": "true"}},
        {"key": "restore_role_ids", "label": "Roles autorises a restaurer", "type": "text", "required": false, "default": "", "description": "IDs de roles autorises a declencher un restore (vide = Owner uniquement)."}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    config_schema = EXCLUDED.config_schema,
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description;
