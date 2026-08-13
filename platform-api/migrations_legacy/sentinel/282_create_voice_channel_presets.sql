-- Preset de parametres de salon vocal, memorise par proprietaire.
-- A la creation d'un nouveau salon temporaire, le bot reapplique ces
-- parametres (nom, limite, visibilite, verrou, file d'attente). La whitelist
-- des membres autorises est geree separement dans voice_channel_whitelists.
CREATE TABLE IF NOT EXISTS voice_channel_presets (
    guild_id       TEXT NOT NULL,
    owner_id       TEXT NOT NULL,
    channel_name   TEXT,
    member_limit   INT,
    visibility     TEXT NOT NULL DEFAULT 'visible',
    locked         BOOLEAN NOT NULL DEFAULT FALSE,
    queue_enabled  BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, owner_id)
);
