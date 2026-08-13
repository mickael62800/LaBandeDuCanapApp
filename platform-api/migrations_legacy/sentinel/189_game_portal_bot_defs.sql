-- ============================================================================
-- Game Portal — bot_definitions paramétrables
-- ============================================================================
-- Deux entrees :
--  1. game-portal       : config systeme cote API (quotas, range ports,
--                          intervals workers, allowed templates...). Visible
--                          dans la page Composants (categorie Bots).
--  2. game-portal-worker: config du worker unifie (intervals, log channel,
--                          notifications). Visible dans la categorie Workers.
--
-- L'API et le worker lisent ces configs via state.bot_config_repo
-- (BaseApiClient::config_*). Si une cle est manquante, on retombe sur le
-- default declare ici.

-- ── 1. game-portal (config systeme) ──────────────────────────────────
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'game-portal',
    'Game Portal — configuration systeme',
    'Quotas, range de ports, securite et templates autorises pour la plateforme Game Portal. Configure les limites globales par guild.',
    '[
        {"key": "enabled", "label": "Game Portal actif", "type": "boolean", "required": false, "default": "true",
         "description": "Active la plateforme Game Portal pour cette guild. Si desactive, aucun serveur ne peut etre cree ou demarre."},

        {"key": "max_servers_per_guild", "label": "Nombre maximum de serveurs par guild", "type": "number", "required": false, "default": "5",
         "description": "Limite stricte du nombre de serveurs (toutes templates confondues) qu''une guild peut creer simultanement. Defaut : 5."},

        {"key": "max_memory_total_mb", "label": "Memoire RAM totale autorisee (Mo)", "type": "number", "required": false, "default": "8192",
         "description": "Plafond cumule de la memoire allouee a tous les serveurs running de la guild. Bloque toute nouvelle allocation au-dela. Defaut : 8 Go."},

        {"key": "port_range_start", "label": "Port minimum du range alloue", "type": "number", "required": false, "default": "25500",
         "description": "Borne inferieure du range de ports HOST utilises pour exposer les serveurs (mapping container_port:host_port). Defaut : 25500."},

        {"key": "port_range_end", "label": "Port maximum du range alloue", "type": "number", "required": false, "default": "25599",
         "description": "Borne superieure du range de ports HOST. Le range total (end - start + 1) doit etre >= max_servers_per_guild. Defaut : 25599 (100 ports)."},

        {"key": "rcon_port_range_start", "label": "Port minimum range RCON", "type": "number", "required": false, "default": "25700",
         "description": "Borne inferieure du range de ports RCON (admin a distance). Doit etre disjoint du range jeu. Defaut : 25700."},

        {"key": "rcon_port_range_end", "label": "Port maximum range RCON", "type": "number", "required": false, "default": "25799",
         "description": "Borne superieure du range de ports RCON. Defaut : 25799."},

        {"key": "allowed_templates", "label": "Templates Docker autorises (slugs CSV)", "type": "text", "required": false, "default": "minecraft-vanilla",
         "description": "Liste des slugs de templates utilisables (separes par virgule). Whitelist stricte : seuls les templates listes peuvent etre instancies. Ex: minecraft-vanilla,valheim,terraria."},

        {"key": "default_idle_shutdown_days", "label": "Jours d''inactivite avant shutdown auto (defaut)", "type": "number", "required": false, "default": "7",
         "description": "Si aucun joueur ne se connecte pendant ce nombre de jours, le worker arrete automatiquement le serveur. 0 = desactive. Override possible par template et par instance."},

        {"key": "docker_network_name", "label": "Nom du network Docker isole", "type": "text", "required": false, "default": "sentinel-games",
         "description": "Network Docker dedie aux containers de jeux. Cree automatiquement si absent. Isole les jeux du reseau interne (postgres, redis...)."},

        {"key": "container_user", "label": "User UID:GID pour les containers", "type": "text", "required": false, "default": "1000:1000",
         "description": "User non-root applique aux containers de jeux (--user). Securite : evite l''escalade privilege root container -> host. Format: UID:GID."},

        {"key": "host_data_dir", "label": "Repertoire host pour donnees persistantes", "type": "text", "required": false, "default": "/var/lib/sentinel/games",
         "description": "Repertoire host (en dehors du container API) ou les volumes Docker nommes sont stockes. Doit etre writable par l''API. Defaut : /var/lib/sentinel/games."},

        {"key": "auto_create_world_volume", "label": "Creer un volume world au lancement", "type": "boolean", "required": false, "default": "true",
         "description": "Cree automatiquement un volume Docker nomme pour persister le monde du jeu. Si desactive, le container redemarre toujours en monde neuf."},

        {"key": "rcon_enabled", "label": "RCON active globalement", "type": "boolean", "required": false, "default": "true",
         "description": "Active la console RCON (commandes admin a distance) pour tous les serveurs supportant RCON. Peut etre desactive globalement pour la guild."},

        {"key": "log_channel_id", "label": "Salon Discord de logs (events serveurs)", "type": "channel", "required": false,
         "description": "Salon ou le bot poste les events critiques (crash detected, idle shutdown, backup created, etc.). Optionnel."}
    ]'
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;


-- ── 2. game-portal-worker (config du worker unifie) ──────────────────
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'game-portal-worker',
    'Worker Game Portal',
    'Worker unifie qui gere le cycle de vie des serveurs : health check (player count, RCON ping), idle shutdown, reconciliation DB <-> Docker.',
    '[
        {"key": "enabled", "label": "Worker actif", "type": "boolean", "required": false, "default": "true"},

        {"key": "health_check_interval_secs", "label": "Intervalle health check (secondes)", "type": "number", "required": false, "default": "30",
         "description": "Frequence de verification de l''etat des serveurs running : RCON ping, player count, status container. Min 10, max 600."},

        {"key": "idle_shutdown_check_interval_secs", "label": "Intervalle check idle shutdown (secondes)", "type": "number", "required": false, "default": "3600",
         "description": "Frequence de scan des serveurs idle (sans joueur depuis N jours). Defaut : 1h. Inutile de scanner souvent."},

        {"key": "reconciler_interval_secs", "label": "Intervalle reconciler DB <-> Docker (secondes)", "type": "number", "required": false, "default": "3600",
         "description": "Frequence de reconciliation entre la table game_servers et la realite Docker. Detecte les orphelins / divergences."},

        {"key": "rcon_timeout_secs", "label": "Timeout RCON par requete (secondes)", "type": "number", "required": false, "default": "5",
         "description": "Timeout d''une commande RCON. Si depasse, le serveur est marque non-responsive."},

        {"key": "auto_restart_on_crash", "label": "Redemarrer automatiquement les containers crashes", "type": "boolean", "required": false, "default": "true",
         "description": "Si un container passe en exited(1) (crash), le worker tente jusqu''a max_auto_restart_attempts redemarrages avec backoff exponentiel."},

        {"key": "max_auto_restart_attempts", "label": "Nb max de redemarrages auto", "type": "number", "required": false, "default": "3",
         "description": "Apres N crashes consecutifs, le serveur est marque error et plus redemarre auto. Reset si le container reste running > 5 min."},

        {"key": "notify_on_idle_shutdown", "label": "Notifier Discord lors d''un idle shutdown", "type": "boolean", "required": false, "default": "true"},
        {"key": "notify_on_crash", "label": "Notifier Discord lors d''un crash", "type": "boolean", "required": false, "default": "true"},
        {"key": "notify_on_player_join", "label": "Notifier Discord chaque connexion joueur", "type": "boolean", "required": false, "default": "false",
         "description": "Bavard si serveur public — desactive par defaut."}
    ]'
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
