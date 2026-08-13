-- Phase composants — Fusion `game-portal-worker` dans `game-portal`.
--
-- game-portal : 16 cles systeme (quotas, ports, securite Docker, templates).
-- game-portal-worker : 10 cles infra (intervals health/idle/reconciler,
--   notifications, auto-restart).
--
-- L'API lit deja les configs sous bot_name='game-portal'. Le worker lisait
-- sous 'game_portal' (mig 204 a renomme game-portal-worker -> game_portal).
-- On unifie tout sous 'game-portal'.

-- 1) Restaure les configs worker sous bot_name='game-portal', en
-- supprimant les doublons (typiquement 'enabled').
DELETE FROM bot_guild_config wkr
    WHERE wkr.bot_name = 'game_portal'
      AND EXISTS (
          SELECT 1 FROM bot_guild_config m
           WHERE m.bot_name = 'game-portal'
             AND m.guild_id = wkr.guild_id
             AND m.config_key = wkr.config_key
      );
UPDATE bot_guild_config SET bot_name = 'game-portal'
    WHERE bot_name = 'game_portal';

-- 2) Schema fusionne avec cascade depends_on.
UPDATE bot_definitions SET
    display_name = 'Game Portal',
    description = 'Plateforme de serveurs de jeux (Minecraft, Valheim, Terraria, Palworld, etc.). Gere quotas, allocation de ports, cycle de vie des containers (health check RCON, idle shutdown, reconciliation DB <-> Docker, image cleanup) et notifications Discord.',
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active la plateforme Game Portal pour cette guild. Si OFF : aucun serveur ne peut etre cree, demarre, ou monitore."},

        {"key": "log_channel_id", "label": "Salon Discord de logs", "type": "channel", "required": false, "description": "Salon ou le bot poste les events critiques (crash detected, idle shutdown, backup created, etc.).", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "max_servers_per_guild", "label": "Serveurs max par guild", "type": "number", "required": false, "default": "5", "min": 1, "max": 100, "description": "Limite stricte du nombre de serveurs (toutes templates confondues) qu une guild peut creer simultanement.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "max_memory_total_mb", "label": "Memoire RAM totale autorisee", "type": "number", "required": false, "default": "8192", "min": 512, "unit": "Mo", "description": "Plafond cumule de la memoire allouee a tous les serveurs running.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "allowed_templates", "label": "Templates autorises (CSV slugs)", "type": "text", "required": false, "default": "minecraft-vanilla", "description": "Whitelist des slugs de templates utilisables (separes par virgule).", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "port_range_start", "label": "Port min range jeu", "type": "number", "required": false, "default": "25500", "min": 1024, "max": 65535, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "port_range_end", "label": "Port max range jeu", "type": "number", "required": false, "default": "25599", "min": 1024, "max": 65535, "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "rcon_enabled", "label": "RCON actif globalement", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "rcon_port_range_start", "label": "Port min range RCON", "type": "number", "required": false, "default": "25700", "min": 1024, "max": 65535, "depends_on": {"key": "rcon_enabled", "equals": "true"}},
        {"key": "rcon_port_range_end", "label": "Port max range RCON", "type": "number", "required": false, "default": "25799", "min": 1024, "max": 65535, "depends_on": {"key": "rcon_enabled", "equals": "true"}},
        {"key": "rcon_timeout_secs", "label": "Timeout RCON par requete", "type": "number", "required": false, "default": "5", "min": 1, "max": 60, "unit": "s", "depends_on": {"key": "rcon_enabled", "equals": "true"}},

        {"key": "docker_network_name", "label": "Network Docker isole", "type": "text", "required": false, "default": "sentinel-games", "description": "Network Docker dedie aux containers de jeux. Cree automatiquement si absent.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "container_user", "label": "User UID:GID containers", "type": "text", "required": false, "default": "1000:1000", "description": "User non-root applique aux containers (--user). Format: UID:GID.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "host_data_dir", "label": "Repertoire host volumes", "type": "text", "required": false, "default": "/var/lib/sentinel/games", "description": "Repertoire host ou les volumes Docker nommes sont stockes.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "auto_create_world_volume", "label": "Creer volume world auto", "type": "boolean", "required": false, "default": "true", "description": "Cree automatiquement un volume Docker nomme pour persister le monde du jeu.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "health_check_interval_secs", "label": "Worker : intervalle health check", "type": "number", "required": false, "default": "30", "min": 10, "max": 600, "unit": "s", "description": "Frequence de verification etat serveurs (RCON ping, player count, status container).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "auto_restart_on_crash", "label": "Redemarrage auto sur crash", "type": "boolean", "required": false, "default": "true", "description": "Si un container crashe, le worker tente jusqu a max_auto_restart_attempts redemarrages avec backoff.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "max_auto_restart_attempts", "label": "Nb max redemarrages auto", "type": "number", "required": false, "default": "3", "min": 1, "max": 10, "description": "Apres N crashes consecutifs, le serveur est marque error.", "depends_on": {"key": "auto_restart_on_crash", "equals": "true"}},

        {"key": "default_idle_shutdown_days", "label": "Idle shutdown (jours sans joueur)", "type": "number", "required": false, "default": "7", "min": 0, "max": 365, "unit": "j", "description": "Si aucun joueur ne se connecte pendant ce nombre de jours, le worker arrete automatiquement le serveur. 0 = desactive.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "idle_shutdown_check_interval_secs", "label": "Worker : intervalle scan idle", "type": "number", "required": false, "default": "3600", "min": 300, "max": 86400, "unit": "s", "description": "Frequence de scan des serveurs idle.", "depends_on": {"key": "default_idle_shutdown_days", "equals": ""}},
        {"key": "reconciler_interval_secs", "label": "Worker : intervalle reconciler DB <-> Docker", "type": "number", "required": false, "default": "3600", "min": 300, "max": 86400, "unit": "s", "description": "Frequence de reconciliation entre la table game_servers et la realite Docker.", "depends_on": {"key": "enabled", "equals": "true"}},

        {"key": "auto_remove_unused_images", "label": "Suppression auto images non utilisees", "type": "boolean", "required": false, "default": "true", "description": "Supprime les images Docker des templates non utilises depuis N jours. Liberation disque (Minecraft 500 MB, Palworld 8 GB).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "unused_image_grace_days", "label": "Jours de grace avant suppression image", "type": "number", "required": false, "default": "7", "min": 0, "max": 365, "unit": "j", "description": "0 = desactive. Si tu relances un serveur, l image se re-pull automatiquement.", "depends_on": {"key": "auto_remove_unused_images", "equals": "true"}},

        {"key": "notify_on_idle_shutdown", "label": "Notifier idle shutdown", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "notify_on_crash", "label": "Notifier crash", "type": "boolean", "required": false, "default": "true", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "notify_on_player_join", "label": "Notifier chaque join joueur", "type": "boolean", "required": false, "default": "false", "description": "Bavard si serveur public.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'game-portal';

DELETE FROM bot_definitions WHERE bot_name = 'game-portal-worker';
