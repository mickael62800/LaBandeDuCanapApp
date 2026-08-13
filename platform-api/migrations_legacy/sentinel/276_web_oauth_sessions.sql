-- Sessions web OAuth (persistance "rester connecté" via refresh token).
--
-- Le navigateur ne garde QUE le token d'accès Discord (court, en sessionStorage)
-- + un cookie httpOnly `ds_session` opaque. Le refresh_token (secret long) reste
-- côté serveur ici. À la réouverture du navigateur, le front appelle
-- /auth/refresh (cookie) → on ré-émet un token d'accès via grant_type=refresh_token
-- sans re-validation Discord interactive.

CREATE TABLE IF NOT EXISTS web_oauth_sessions (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    discord_user_id   TEXT NOT NULL,
    username          TEXT NOT NULL DEFAULT '',
    global_name       TEXT,
    avatar            TEXT,
    access_token      TEXT NOT NULL,
    refresh_token     TEXT NOT NULL,
    access_expires_at TIMESTAMPTZ NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_web_oauth_sessions_user ON web_oauth_sessions (discord_user_id);
CREATE INDEX IF NOT EXISTS idx_web_oauth_sessions_last_used ON web_oauth_sessions (last_used_at);
