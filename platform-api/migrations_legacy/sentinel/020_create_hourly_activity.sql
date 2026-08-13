-- Activite par heure pour heatmaps et pics d'activite
CREATE TABLE IF NOT EXISTS hourly_activity (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id   TEXT    NOT NULL,
    day        DATE    NOT NULL,
    hour       SMALLINT NOT NULL CHECK (hour >= 0 AND hour <= 23),
    messages   BIGINT  NOT NULL DEFAULT 0,
    infractions INTEGER NOT NULL DEFAULT 0,
    UNIQUE (guild_id, day, hour)
);

CREATE INDEX IF NOT EXISTS idx_hourly_activity_guild ON hourly_activity(guild_id);
CREATE INDEX IF NOT EXISTS idx_hourly_activity_day ON hourly_activity(day DESC);
