-- Panels de roles (un panel = un message avec des boutons/reactions)
CREATE TABLE IF NOT EXISTS role_panels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    message_id TEXT,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    mode TEXT NOT NULL DEFAULT 'button',
    max_roles INTEGER,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Entrees du panel (chaque entree = un bouton/reaction → un role)
CREATE TABLE IF NOT EXISTS role_panel_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    panel_id UUID NOT NULL REFERENCES role_panels(id) ON DELETE CASCADE,
    role_id TEXT NOT NULL,
    role_name TEXT NOT NULL DEFAULT '',
    emoji TEXT,
    label TEXT NOT NULL DEFAULT '',
    style TEXT NOT NULL DEFAULT 'primary',
    position INTEGER NOT NULL DEFAULT 0
);

-- Auto-role a l'arrivee (roles attribues automatiquement aux nouveaux membres)
CREATE TABLE IF NOT EXISTS auto_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    role_name TEXT NOT NULL DEFAULT '',
    delay_secs INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    CONSTRAINT uq_auto_roles_guild_role UNIQUE (guild_id, role_id)
);

CREATE INDEX idx_role_panels_guild ON role_panels (guild_id);
CREATE INDEX idx_role_panels_message ON role_panels (message_id);
CREATE INDEX idx_role_panel_entries_panel ON role_panel_entries (panel_id);
CREATE INDEX idx_auto_roles_guild ON auto_roles (guild_id);
