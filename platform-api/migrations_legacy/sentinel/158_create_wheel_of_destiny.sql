-- Migration 158 : Roue du Destin (wheel-bot).
--
-- Mecanique : 1 spin par joueur par jour. 10 cases ponderees, payout en
-- coins (positif ou negatif), broadcast PUBLIC dans le salon configure.
-- Pas de salon prive contrairement a slot — c est la signature commune
-- du serveur (rituel quotidien collectif).

-- ══════════════════════════════════════════════════════════
-- Tables
-- ══════════════════════════════════════════════════════════

-- Historique des spins. Une row par spin, broadcast public.
CREATE TABLE IF NOT EXISTS wheel_spin_log (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    VARCHAR(20) NOT NULL,
    user_id     VARCHAR(20) NOT NULL,
    username    VARCHAR(100) NOT NULL,
    case_key    VARCHAR(40) NOT NULL,
    case_label  VARCHAR(100) NOT NULL,
    payout      BIGINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_wheel_spin_log_guild_created
    ON wheel_spin_log (guild_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_wheel_spin_log_user_guild
    ON wheel_spin_log (guild_id, user_id, created_at DESC);

-- Tracking du daily : 1 row par (user, day). Existence = deja claim.
CREATE TABLE IF NOT EXISTS wheel_daily_claims (
    guild_id    VARCHAR(20) NOT NULL,
    user_id     VARCHAR(20) NOT NULL,
    day         DATE        NOT NULL,
    claimed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id, day)
);

-- ══════════════════════════════════════════════════════════
-- bot_definitions : wheel-bot avec schema enrichi
-- ══════════════════════════════════════════════════════════
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'wheel-bot',
    'Roue du Destin',
    'Rituel quotidien : 1 spin par joueur par jour, resultat broadcast publiquement. Le destin decide.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true",
         "description": "Active ou desactive la Roue du Destin. Si OFF, le panel ne repond plus."},

        {"key": "panel_channel_id", "label": "Salon du panel", "type": "channel", "required": false,
         "description": "Salon ou est poste le panel persistant avec le bouton Tirer la roue. Configure via /wheel-setup."},

        {"key": "broadcast_channel_id", "label": "Salon d annonces", "type": "channel", "required": false,
         "description": "Salon ou sont broadcastes les resultats de spin (publics). Vide = poste dans le salon courant du panel."},

        {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false,
         "description": "Salon ou sont logges les jackpots et licornes. Vide = pas de log dedie."},

        {"key": "panel_message", "label": "Message du panel", "type": "text", "required": false,
         "default": "🪙 **La Roue du Destin** 🪙\n\nUne fois par jour, tente ta chance.\nLe destin peut te rendre riche... ou ridicule.",
         "description": "Texte affiche dans le panel persistant. Markdown supporte."}
    ]'::jsonb
)
ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
