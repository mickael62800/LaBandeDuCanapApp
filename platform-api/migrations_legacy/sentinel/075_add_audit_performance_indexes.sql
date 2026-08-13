-- Index pour les requetes audit-bot frequentes

-- User activity par guild + user + event type
CREATE INDEX IF NOT EXISTS idx_user_activity_guild_user_type
    ON user_activity_log (guild_id, user_id, event_type);

-- Audit logs par guild + event type + date
CREATE INDEX IF NOT EXISTS idx_audit_logs_guild_type_date
    ON audit_logs (guild_id, event_type, created_at DESC);
