-- Table des membres de serveurs Discord pour la page Membres
CREATE TABLE IF NOT EXISTS guild_members (
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    username        TEXT NOT NULL,
    display_name    TEXT,
    avatar          TEXT,
    roles           JSONB DEFAULT '[]',
    joined_at       TIMESTAMPTZ,
    account_created TIMESTAMPTZ,
    is_bot          BOOLEAN DEFAULT FALSE,
    last_seen_at    TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_guild_members_guild ON guild_members (guild_id);
CREATE INDEX IF NOT EXISTS idx_guild_members_username ON guild_members (guild_id, username);
