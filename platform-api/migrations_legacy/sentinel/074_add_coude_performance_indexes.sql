-- Index pour les requetes frequentes du coude-bot

-- Combats en attente par attaquant/defenseur
CREATE INDEX IF NOT EXISTS idx_combats_attacker_pending
    ON coude_combats (guild_id, attacker_id, status) WHERE status IN ('pending', 'betting');
CREATE INDEX IF NOT EXISTS idx_combats_defender_pending
    ON coude_combats (guild_id, defender_id, status) WHERE status IN ('pending', 'betting');

-- Combats en phase betting (utilise par le worker)
CREATE INDEX IF NOT EXISTS idx_combats_betting_accepted
    ON coude_combats (status, accepted_at) WHERE status IN ('betting', 'resolving');

-- Cooldowns actifs
CREATE INDEX IF NOT EXISTS idx_cooldowns_check
    ON coude_cooldowns (guild_id, user_id, action);

-- Paris par combat
CREATE INDEX IF NOT EXISTS idx_bets_combat
    ON coude_bets (combat_id);

-- Joueurs par guild (leaderboard)
CREATE INDEX IF NOT EXISTS idx_players_guild_coins
    ON coude_players (guild_id, coins DESC);
