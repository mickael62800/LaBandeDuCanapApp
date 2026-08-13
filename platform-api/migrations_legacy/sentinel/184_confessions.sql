-- Systeme de confessions anonymes : un bot proxy poste les messages des
-- users dans un canal config sans reveler leur identite. Les replies
-- aussi anonymes via bouton, ou non-anonymes via message normal dans
-- le thread. Bouton Report pour signaler. Modération via web ou slash.

CREATE TABLE IF NOT EXISTS confessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    public_number INT NOT NULL,            -- #350 visible sur Discord
    author_user_id TEXT NOT NULL,          -- plain, owner-only access
    content TEXT NOT NULL,
    -- Discord refs (NULL avant que le bot ait poste)
    message_id TEXT,
    channel_id TEXT,
    thread_id TEXT,
    -- Soft delete pour preserver les replies + numerotation
    deleted_at TIMESTAMPTZ,
    deleted_by TEXT,                       -- user_id ou "system" / "bot"
    deleted_reason TEXT,
    -- Edition
    edited_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (guild_id, public_number)
);

CREATE INDEX IF NOT EXISTS idx_confessions_guild
    ON confessions (guild_id, public_number DESC);
CREATE INDEX IF NOT EXISTS idx_confessions_message
    ON confessions (message_id) WHERE message_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_confessions_author
    ON confessions (guild_id, author_user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS confession_replies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    confession_id UUID NOT NULL REFERENCES confessions(id) ON DELETE CASCADE,
    public_number INT NOT NULL,            -- #357 visible Discord (par-confession)
    author_user_id TEXT NOT NULL,
    content TEXT NOT NULL,
    -- Si is_anonymous = false, le bot poste avec le vrai user (msg normal
    -- via Discord direct, mais on log quand meme pour traçabilite). Si
    -- true, le bot poste comme proxy avec "Anonymous Reply (#N)".
    is_anonymous BOOLEAN NOT NULL DEFAULT TRUE,
    message_id TEXT,
    deleted_at TIMESTAMPTZ,
    deleted_by TEXT,
    edited_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (confession_id, public_number)
);

CREATE INDEX IF NOT EXISTS idx_confession_replies_confession
    ON confession_replies (confession_id, public_number);
CREATE INDEX IF NOT EXISTS idx_confession_replies_message
    ON confession_replies (message_id) WHERE message_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS confession_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    confession_id UUID REFERENCES confessions(id) ON DELETE CASCADE,
    reply_id UUID REFERENCES confession_replies(id) ON DELETE CASCADE,
    reporter_user_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'resolved', 'dismissed')),
    resolved_by TEXT,
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Au moins une cible (confession ou reply)
    CONSTRAINT report_target_required CHECK (
        confession_id IS NOT NULL OR reply_id IS NOT NULL
    )
);

CREATE INDEX IF NOT EXISTS idx_confession_reports_guild_status
    ON confession_reports (guild_id, status, created_at DESC);

CREATE TABLE IF NOT EXISTS confession_config (
    guild_id TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    channel_id TEXT,                       -- canal ou poster
    panel_message_id TEXT,                 -- message du bouton "Submit"
    cooldown_secs INT NOT NULL DEFAULT 60,
    max_per_day INT NOT NULL DEFAULT 20,
    min_chars INT NOT NULL DEFAULT 5,
    max_chars INT NOT NULL DEFAULT 2000,
    automod_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    -- Liste de user_id bannis du systeme (peuvent plus poster)
    banned_user_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Sequence de numerotation par guild. PostgreSQL ne supporte pas
-- nativement les sequences scopées par tenant donc on fait un helper.
CREATE TABLE IF NOT EXISTS confession_counters (
    guild_id TEXT PRIMARY KEY,
    last_number INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
