-- Quotas DeepSeek persistants : resistent aux redemarrages de l'API.
CREATE TABLE IF NOT EXISTS atrium_ai_usage_users (
    usage_date DATE NOT NULL DEFAULT CURRENT_DATE,
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 0 CHECK (request_count >= 0),
    last_request_at TIMESTAMPTZ,
    PRIMARY KEY (usage_date, guild_id, user_id)
);

CREATE TABLE IF NOT EXISTS atrium_ai_usage_global (
    usage_date DATE PRIMARY KEY DEFAULT CURRENT_DATE,
    request_count INTEGER NOT NULL DEFAULT 0 CHECK (request_count >= 0)
);

CREATE INDEX IF NOT EXISTS idx_atrium_ai_usage_users_date
    ON atrium_ai_usage_users (usage_date);
