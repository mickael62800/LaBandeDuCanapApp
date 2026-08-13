CREATE TABLE IF NOT EXISTS rules (
    id         UUID PRIMARY KEY,
    guild_id   TEXT    NOT NULL,
    flag_type  TEXT    NOT NULL,
    weight     DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    threshold_warn   DOUBLE PRECISION NOT NULL DEFAULT 2.0,
    threshold_delete DOUBLE PRECISION NOT NULL DEFAULT 4.0,
    threshold_mute   DOUBLE PRECISION NOT NULL DEFAULT 6.0,
    threshold_ban    DOUBLE PRECISION NOT NULL DEFAULT 9.0,
    enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (guild_id, flag_type)
);
