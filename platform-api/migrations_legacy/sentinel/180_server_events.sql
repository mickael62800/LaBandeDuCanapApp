-- Audit log SERVEUR (vs audit_logs qui contient des events Discord du bot).
-- Stocke les actions admin sur l'infra : Docker (start/stop/prune), cleanup
-- logs, RBAC grant/revoke, invitation create/redeem, login OAuth, etc.
-- Lu par /api/security/server-events sur la page Securite serveur.

CREATE TABLE IF NOT EXISTS server_events (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actor       TEXT,                            -- discord_user_id, 'system', 'cron'
    actor_name  TEXT,                            -- username Discord si dispo
    action      TEXT NOT NULL,                   -- 'docker.container.start', 'security.cleanup', etc.
    target      TEXT,                            -- container_id, count, etc.
    severity    TEXT NOT NULL DEFAULT 'info'
                CHECK (severity IN ('info', 'warn', 'critical')),
    details     JSONB NOT NULL DEFAULT '{}'      -- payload libre
);

CREATE INDEX IF NOT EXISTS idx_server_events_ts ON server_events (timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_server_events_actor ON server_events (actor);
CREATE INDEX IF NOT EXISTS idx_server_events_action ON server_events (action);
CREATE INDEX IF NOT EXISTS idx_server_events_severity ON server_events (severity);
