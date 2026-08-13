-- Phase 6A — State tracking pour discord-audit-sync-worker
--
-- Le worker poll l'API Discord `GET /guilds/{id}/audit-logs` pour importer
-- les actions de moderation effectuees HORS du bot (via le client Discord
-- directement, ou un autre bot). Cette table garde l'`entry_id` du dernier
-- audit log import par guild pour eviter de re-fetcher depuis le debut a
-- chaque tick.
--
-- `last_entry_id` est un snowflake Discord (numerique en string). Le worker
-- utilise `GET ...?after={last_entry_id}` pour recuperer uniquement les
-- entries plus recentes que le dernier sync.

CREATE TABLE IF NOT EXISTS discord_audit_sync_state (
    guild_id        VARCHAR(20) PRIMARY KEY,
    last_entry_id   TEXT,
    last_synced_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_error      TEXT,
    consecutive_errors INT NOT NULL DEFAULT 0
);
