-- Coup de Coude — socle Nexus (joueurs, duels et inventaire).
CREATE TABLE nexus_coude_players (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), guild_id VARCHAR(20) NOT NULL,
    user_id VARCHAR(20) NOT NULL, username VARCHAR(100) NOT NULL,
    class VARCHAR(16) NOT NULL DEFAULT 'bourrin' CHECK (class IN ('bourrin','agile','fourbe','tank')),
    coins BIGINT NOT NULL DEFAULT 100 CHECK (coins >= 0), total_wins INT NOT NULL DEFAULT 0,
    total_losses INT NOT NULL DEFAULT 0, total_draws INT NOT NULL DEFAULT 0,
    total_earned BIGINT NOT NULL DEFAULT 0, total_lost BIGINT NOT NULL DEFAULT 0,
    total_stolen BIGINT NOT NULL DEFAULT 0, cowardice_count INT NOT NULL DEFAULT 0,
    chaos_events INT NOT NULL DEFAULT 0, level INT NOT NULL DEFAULT 1 CHECK (level BETWEEN 1 AND 25),
    xp BIGINT NOT NULL DEFAULT 0, stat_points INT NOT NULL DEFAULT 0, atk INT NOT NULL DEFAULT 1,
    def INT NOT NULL DEFAULT 1, hp_current INT NOT NULL DEFAULT 100, hp_max INT NOT NULL DEFAULT 100,
    hp_last_regen TIMESTAMPTZ, repos_last_used TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (guild_id, user_id)
);
CREATE INDEX idx_nexus_coude_players_rank ON nexus_coude_players (guild_id, coins DESC);

CREATE TABLE nexus_coude_combats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), guild_id VARCHAR(20) NOT NULL,
    channel_id VARCHAR(20) NOT NULL, attacker_id VARCHAR(20) NOT NULL, attacker_name VARCHAR(100) NOT NULL,
    defender_id VARCHAR(20) NOT NULL, defender_name VARCHAR(100) NOT NULL,
    mise BIGINT NOT NULL DEFAULT 10 CHECK (mise > 0), status VARCHAR(16) NOT NULL DEFAULT 'pending',
    winner_id VARCHAR(20), attacker_roll INT, defender_roll INT, chaos_event TEXT, special_attack TEXT,
    result_message TEXT, coins_transferred BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), resolved_at TIMESTAMPTZ
);
CREATE INDEX idx_nexus_coude_pending ON nexus_coude_combats (defender_id, status) WHERE status = 'pending';

CREATE TABLE nexus_coude_inventory (
    guild_id VARCHAR(20) NOT NULL, user_id VARCHAR(20) NOT NULL, item_key VARCHAR(64) NOT NULL,
    quantity INT NOT NULL DEFAULT 1 CHECK (quantity >= 0), created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id, item_key)
);
