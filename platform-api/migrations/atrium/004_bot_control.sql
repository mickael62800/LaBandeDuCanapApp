-- Activation d'Atrium par serveur, pilotee par une commande Discord admin.
CREATE TABLE IF NOT EXISTS atrium_guild_settings (
    guild_id TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    updated_by TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
