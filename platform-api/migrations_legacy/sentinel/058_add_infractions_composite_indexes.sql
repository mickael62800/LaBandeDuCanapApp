-- Index composite pour les requetes filtrees par guild + date (analytics, listings pagines)
-- Couvre : GET /api/infractions/{guild_id}?limit=50&offset=0
--          GET /api/analytics (action_distribution, top_infractors, moderation_trend)
CREATE INDEX IF NOT EXISTS idx_infractions_guild_created
    ON infractions (guild_id, created_at DESC);

-- Index composite pour les requetes filtrees par guild + action + date
-- Couvre : analytics action_distribution (WHERE guild_id = $1 AND action != 'none')
CREATE INDEX IF NOT EXISTS idx_infractions_guild_action_created
    ON infractions (guild_id, action, created_at DESC);

-- Index composite sur moderation_actions pour les bans pagines
-- Couvre : GET /api/moderation/bans?guild_id={id}&limit=50
CREATE INDEX IF NOT EXISTS idx_mod_actions_guild_type_created
    ON moderation_actions (guild_id, action_type, created_at DESC);

-- Index composite sur audit_logs pour les requetes filtrees par guild + date
-- Couvre : GET /api/audit-logs?guild_id={id}&limit=100
CREATE INDEX IF NOT EXISTS idx_audit_logs_guild_created
    ON audit_logs (guild_id, created_at DESC);
