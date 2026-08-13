-- Ajoute les definitions de bot virtuels "announcements" et "confessions"
-- pour qu'ils apparaissent dans la page Composants (/component-config)
-- avec leurs parametres configurables.
--
-- Note : ce ne sont pas de vrais "bots" au sens services Rust - ce sont
-- des modules dont la config est consumee par sentinel-api + sentinel-bot.
-- Mais on les expose comme des composants pour reutiliser l'UI generique
-- de configuration.

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES
(
    'announcements',
    'Annonces planifiees',
    'Messages Discord postes automatiquement (ponctuel, quotidien, hebdo, mensuel) avec embed riche, mentions, boutons interactifs et reactions automatiques.',
    '[
        {"key": "default_color_hex", "label": "Couleur par defaut (embed) en hex (ex: #5865f2)", "type": "text", "required": false, "default": "#5865f2"},
        {"key": "max_announcements_per_guild", "label": "Nombre max d''annonces par serveur", "type": "number", "required": false, "default": "100"},
        {"key": "default_mention_everyone", "label": "Activer @everyone par defaut", "type": "boolean", "required": false, "default": "false"},
        {"key": "history_retention_days", "label": "Retention historique (jours)", "type": "number", "required": false, "default": "90"},
        {"key": "log_channel_id", "label": "Salon de logs (publication / erreurs)", "type": "channel", "required": false}
    ]'
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES
(
    'confessions',
    'Confessions anonymes',
    'Systeme de confessions anonymes : les utilisateurs postent des messages sans reveler leur identite, replies dans des threads, signalements et moderation.',
    '[
        {"key": "enabled", "label": "Module active", "type": "boolean", "required": false, "default": "true"},
        {"key": "channel_id", "label": "Salon ou poster les confessions", "type": "channel", "required": true},
        {"key": "panel_message_id", "label": "ID du message du bouton Submit (auto-rempli)", "type": "text", "required": false},
        {"key": "cooldown_secs", "label": "Cooldown entre 2 confessions (secondes)", "type": "number", "required": false, "default": "60"},
        {"key": "max_per_day", "label": "Nombre max de confessions par jour par utilisateur", "type": "number", "required": false, "default": "20"},
        {"key": "min_chars", "label": "Longueur minimum d''une confession", "type": "number", "required": false, "default": "5"},
        {"key": "max_chars", "label": "Longueur maximum d''une confession (max 4000)", "type": "number", "required": false, "default": "2000"},
        {"key": "automod_enabled", "label": "Filtre AutoMod actif (refuse contenu toxique)", "type": "boolean", "required": false, "default": "true"},
        {"key": "default_embed_color_hex", "label": "Couleur embed des confessions (hex, ex: #ff5e5e)", "type": "text", "required": false, "default": "#ff5e5e"},
        {"key": "moderation_log_channel_id", "label": "Salon de logs de moderation (signalements, suppressions)", "type": "channel", "required": false},
        {"key": "show_report_button", "label": "Afficher le bouton Report sous chaque confession", "type": "boolean", "required": false, "default": "true"},
        {"key": "show_reply_button", "label": "Afficher le bouton Reply sous chaque confession", "type": "boolean", "required": false, "default": "true"}
    ]'
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
