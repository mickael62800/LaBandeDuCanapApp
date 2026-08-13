-- Tables de blackjack multijoueur.
-- Une table = un channel Discord avec plusieurs joueurs.
-- Chaque joueur a sa propre partie (blackjack_games) liee a la table.

CREATE TABLE IF NOT EXISTS blackjack_tables (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    channel_id      TEXT NOT NULL UNIQUE,
    owner_id        TEXT NOT NULL,
    owner_name      TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'open',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bj_tables_guild ON blackjack_tables (guild_id);
CREATE INDEX idx_bj_tables_status ON blackjack_tables (status) WHERE status = 'open';

-- Lier les parties a une table (optionnel — NULL = partie solo legacy)
ALTER TABLE blackjack_games ADD COLUMN IF NOT EXISTS table_id UUID REFERENCES blackjack_tables(id) ON DELETE SET NULL;

-- Joueurs invites a une table
CREATE TABLE IF NOT EXISTS blackjack_table_players (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    table_id        UUID NOT NULL REFERENCES blackjack_tables(id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL,
    user_name       TEXT NOT NULL,
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (table_id, user_id)
);

CREATE INDEX idx_bj_players_table ON blackjack_table_players (table_id);
