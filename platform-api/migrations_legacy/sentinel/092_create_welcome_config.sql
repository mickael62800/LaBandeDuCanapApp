-- Configuration du systeme de bienvenue par serveur.
CREATE TABLE IF NOT EXISTS welcome_config (
    guild_id                TEXT PRIMARY KEY,

    -- Bienvenue
    welcome_enabled         BOOLEAN NOT NULL DEFAULT true,
    welcome_channel_id      TEXT,
    welcome_message         TEXT NOT NULL DEFAULT 'Bienvenue {user} sur **{server}** ! Tu es le **{count}e** membre.',
    welcome_embed_color     TEXT NOT NULL DEFAULT '3498db',
    welcome_dm_enabled      BOOLEAN NOT NULL DEFAULT false,
    welcome_dm_message      TEXT NOT NULL DEFAULT 'Bienvenue sur **{server}** ! N''oublie pas de lire les regles.',

    -- Depart
    leave_enabled           BOOLEAN NOT NULL DEFAULT true,
    leave_channel_id        TEXT,
    leave_message           TEXT NOT NULL DEFAULT '{user} nous a quittes. Nous sommes maintenant **{count}** membres.',

    -- Reglement
    rules_enabled           BOOLEAN NOT NULL DEFAULT false,
    rules_channel_id        TEXT,
    rules_message           TEXT NOT NULL DEFAULT 'Lis les regles et clique sur le bouton pour acceder au serveur.',
    rules_role_id           TEXT,
    rules_button_label      TEXT NOT NULL DEFAULT 'J''accepte les regles',

    -- Compteur de membres (canal vocal renomme)
    counter_enabled         BOOLEAN NOT NULL DEFAULT false,
    counter_channel_id      TEXT,
    counter_format          TEXT NOT NULL DEFAULT 'Membres : {count}',

    -- Anniversaires serveur
    anniversary_enabled     BOOLEAN NOT NULL DEFAULT false,
    anniversary_channel_id  TEXT,
    anniversary_message     TEXT NOT NULL DEFAULT 'Felicitations {user}, ca fait **{years} an(s)** que tu es sur **{server}** !',

    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed bot_definitions
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'welcome-bot',
    'Welcome Bot',
    'Accueil des nouveaux membres — bienvenue, depart, reglement, compteur, anniversaires.',
    '[
        {"key": "welcome_channel_id", "label": "Salon de bienvenue", "type": "channel", "required": false},
        {"key": "welcome_message", "label": "Message de bienvenue ({user}, {server}, {count})", "type": "text", "required": false},
        {"key": "welcome_dm_enabled", "label": "Envoyer un DM de bienvenue", "type": "boolean", "required": false, "default": "false"},
        {"key": "leave_channel_id", "label": "Salon de depart", "type": "channel", "required": false},
        {"key": "leave_message", "label": "Message de depart", "type": "text", "required": false},
        {"key": "rules_enabled", "label": "Validation du reglement", "type": "boolean", "required": false, "default": "false"},
        {"key": "rules_channel_id", "label": "Salon du reglement", "type": "channel", "required": false},
        {"key": "rules_role_id", "label": "Role apres validation", "type": "role", "required": false},
        {"key": "counter_enabled", "label": "Compteur de membres", "type": "boolean", "required": false, "default": "false"},
        {"key": "counter_channel_id", "label": "Canal vocal compteur", "type": "channel", "required": false},
        {"key": "anniversary_enabled", "label": "Anniversaires serveur", "type": "boolean", "required": false, "default": "false"},
        {"key": "anniversary_channel_id", "label": "Salon anniversaires", "type": "channel", "required": false}
    ]'
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
