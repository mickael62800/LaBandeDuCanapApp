-- Phase 5H — Persistance des slowmodes anti-raid actifs.
-- Meme pattern que security_lockdown_active : JSON des etats originaux
-- pour restauration via worker + consumer Redis.

CREATE TABLE IF NOT EXISTS security_slowmode_active (
    guild_id        TEXT PRIMARY KEY,
    previous_states JSONB NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_security_slowmode_expires
    ON security_slowmode_active (expires_at);
