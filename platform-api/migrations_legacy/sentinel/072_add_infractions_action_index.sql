-- Index pour les requetes filtrees par action (warn, delete, mute, ban)
CREATE INDEX IF NOT EXISTS idx_infractions_guild_action
    ON infractions (guild_id, action);

-- Index pour les requetes par date (tri chronologique)
CREATE INDEX IF NOT EXISTS idx_infractions_guild_created
    ON infractions (guild_id, created_at DESC);
