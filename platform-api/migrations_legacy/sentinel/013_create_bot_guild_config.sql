-- Configuration des bots par serveur
CREATE TABLE IF NOT EXISTS bot_guild_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id VARCHAR(20) NOT NULL,
    bot_name VARCHAR(50) NOT NULL,
    config_key VARCHAR(100) NOT NULL,
    config_value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, bot_name, config_key)
);

CREATE INDEX idx_bot_guild_config_guild ON bot_guild_config (guild_id);
CREATE INDEX idx_bot_guild_config_bot ON bot_guild_config (guild_id, bot_name);

-- Definition des bots et leurs parametres disponibles (table de reference)
CREATE TABLE IF NOT EXISTS bot_definitions (
    bot_name VARCHAR(50) PRIMARY KEY,
    display_name VARCHAR(100) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    config_schema JSONB NOT NULL DEFAULT '[]'
);

-- Inserer les definitions des bots
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema) VALUES
('automod-bot', 'AutoMod', 'Moderation automatique des messages (spam, insultes, liens)', '[]'),
('moderation-bot', 'Moderation', 'Actions de moderation manuelles (warn, mute, ban)', '[
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false}
]'),
('security-bot', 'Securite', 'Detection de raids et comptes suspects', '[
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
    {"key": "alert_channel_id", "label": "Salon d alertes", "type": "channel", "required": false}
]'),
('stats-bot', 'Statistiques', 'Suivi des messages et du temps vocal', '[]'),
('ticket-bot', 'Tickets', 'Systeme d assistance par tickets', '[
    {"key": "assistance_channel_id", "label": "Salon d assistance", "type": "channel", "required": true},
    {"key": "admin_role_id", "label": "Role Administrateur", "type": "role", "required": true},
    {"key": "moderator_role_id", "label": "Role Moderateur", "type": "role", "required": true}
]'),
('voice-bot', 'Vocaux', 'Salons vocaux temporaires', '[
    {"key": "public_creator_channel_id", "label": "Salon createur public", "type": "channel", "required": true},
    {"key": "private_creator_channel_id", "label": "Salon createur prive", "type": "channel", "required": true},
    {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false}
]')
ON CONFLICT (bot_name) DO NOTHING;
