-- Phase Annonces planifiees : tables pour les annonces recurrentes
-- (one-shot / quotidien / hebdo / mensuel) postees automatiquement par
-- announcement-worker via Redis stream consumee par sentinel-bot.

CREATE TABLE IF NOT EXISTS scheduled_announcements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,

    -- Recurrence
    recurrence_type TEXT NOT NULL CHECK (recurrence_type IN ('once', 'daily', 'weekly', 'monthly')),
    recurrence_hour SMALLINT NOT NULL CHECK (recurrence_hour BETWEEN 0 AND 23),
    recurrence_minute SMALLINT NOT NULL DEFAULT 0 CHECK (recurrence_minute BETWEEN 0 AND 59),
    recurrence_day_of_week SMALLINT CHECK (recurrence_day_of_week BETWEEN 0 AND 6),
    recurrence_day_of_month SMALLINT CHECK (recurrence_day_of_month BETWEEN 1 AND 31),
    scheduled_at TIMESTAMPTZ,

    -- Plage de validite (NULL end_date = indefini)
    start_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    end_date TIMESTAMPTZ,

    -- Contenu
    content_type TEXT NOT NULL CHECK (content_type IN ('text', 'embed')),
    content_text TEXT NOT NULL DEFAULT '',
    embed_title TEXT,
    embed_color INT,
    embed_image_url TEXT,
    embed_thumbnail_url TEXT,

    -- Mentions
    mention_everyone BOOLEAN NOT NULL DEFAULT FALSE,
    mention_here BOOLEAN NOT NULL DEFAULT FALSE,
    mention_role_ids JSONB NOT NULL DEFAULT '[]'::jsonb,

    -- Cibles (JSONB array de channel_id, au moins 1)
    channel_ids JSONB NOT NULL,

    -- Auteur + meta
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Tracking pour le worker
    last_run_at TIMESTAMPTZ,
    next_run_at TIMESTAMPTZ NOT NULL,

    -- Coherence : 'once' doit avoir scheduled_at, 'weekly' doit avoir
    -- recurrence_day_of_week, 'monthly' doit avoir recurrence_day_of_month
    CONSTRAINT recurrence_consistency CHECK (
        (recurrence_type = 'once' AND scheduled_at IS NOT NULL)
        OR (recurrence_type = 'daily')
        OR (recurrence_type = 'weekly' AND recurrence_day_of_week IS NOT NULL)
        OR (recurrence_type = 'monthly' AND recurrence_day_of_month IS NOT NULL)
    )
);

-- Lookup principal du worker : "annonces dues maintenant"
CREATE INDEX IF NOT EXISTS idx_announcements_next_run
    ON scheduled_announcements (next_run_at)
    WHERE enabled = TRUE;

-- Listing par guild dans la page web
CREATE INDEX IF NOT EXISTS idx_announcements_guild
    ON scheduled_announcements (guild_id, created_at DESC);

-- Historique des runs : un INSERT par execution (success/partial/error)
CREATE TABLE IF NOT EXISTS scheduled_announcement_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    announcement_id UUID NOT NULL REFERENCES scheduled_announcements(id) ON DELETE CASCADE,
    guild_id TEXT NOT NULL,
    ran_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- [{channel_id, message_id, success, error?}, ...]
    channels_posted JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL CHECK (status IN ('success', 'partial', 'error', 'pending')),
    error TEXT
);

CREATE INDEX IF NOT EXISTS idx_announcement_runs_announcement
    ON scheduled_announcement_runs (announcement_id, ran_at DESC);
CREATE INDEX IF NOT EXISTS idx_announcement_runs_guild
    ON scheduled_announcement_runs (guild_id, ran_at DESC);
