-- Trace des logins Discord OAuth reussis pour la page Securite serveur
-- (onglet "Bans & Protections" -> section "Last successful logins").

CREATE TABLE IF NOT EXISTS successful_logins (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    logged_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    discord_user_id TEXT NOT NULL,
    username        TEXT,
    client_ip       TEXT,
    user_agent      TEXT
);

CREATE INDEX IF NOT EXISTS idx_successful_logins_at ON successful_logins (logged_at DESC);
CREATE INDEX IF NOT EXISTS idx_successful_logins_user ON successful_logins (discord_user_id);
