-- Rules: index on guild_id for /analyze lookups (WHERE guild_id = $1)
-- The UNIQUE(guild_id, flag_type) constraint creates a composite index,
-- but PostgreSQL can only use it efficiently when guild_id is the leading column
-- AND flag_type is also in the query. A standalone index on guild_id is faster
-- for queries that only filter by guild_id.
CREATE INDEX IF NOT EXISTS idx_rules_guild ON rules (guild_id);

-- Tickets: index on common query patterns
CREATE INDEX IF NOT EXISTS idx_tickets_author ON tickets (author_id);
CREATE INDEX IF NOT EXISTS idx_tickets_assigned ON tickets (assigned_to) WHERE assigned_to IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tickets_server ON tickets (server);
CREATE INDEX IF NOT EXISTS idx_tickets_created ON tickets (created_at DESC);

-- Infractions: index for sorted pagination
CREATE INDEX IF NOT EXISTS idx_infractions_created ON infractions (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_infractions_action ON infractions (action);

-- Moderation actions: index for filtering by action type (ban list, warn list)
CREATE INDEX IF NOT EXISTS idx_mod_actions_type ON moderation_actions (action_type);
CREATE INDEX IF NOT EXISTS idx_mod_actions_created ON moderation_actions (created_at DESC);

-- Security events: index for sorted queries
CREATE INDEX IF NOT EXISTS idx_security_events_created ON security_events (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_security_events_severity ON security_events (severity);
