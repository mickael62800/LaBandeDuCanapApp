-- Phase 1 sync Discord <-> Web (cf. SYNC_DISCORD_WEB_DESIGN.md).
--
-- Mapping entite metier <-> message Discord poste pour la representer.
-- Permet a l API de retrouver "quel message edit/delete sur Discord"
-- quand une action change de statut (web ou Discord).
--
-- Cle composite (action_id, kind) car une meme action peut avoir plusieurs
-- representations (ex. embed + thread + DM = 3 rows differentes).

CREATE TABLE IF NOT EXISTS discord_action_messages (
    action_id      UUID NOT NULL,
    kind           TEXT NOT NULL,
    guild_id       TEXT NOT NULL,
    channel_id     TEXT NOT NULL,
    message_id     TEXT NOT NULL,
    posted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_edited_at TIMESTAMPTZ,
    PRIMARY KEY (action_id, kind),
    UNIQUE (guild_id, channel_id, message_id)
);

CREATE INDEX IF NOT EXISTS idx_dam_kind_guild ON discord_action_messages(kind, guild_id);
CREATE INDEX IF NOT EXISTS idx_dam_action ON discord_action_messages(action_id);
