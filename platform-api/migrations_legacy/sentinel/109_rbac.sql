-- Phase 7 B — RBAC fin par guild.
--
-- Etend le multi-tenant de Phase 2 B avec une notion de role applicatif :
-- chaque utilisateur Discord (identifie par son user_id) peut avoir un role
-- different sur chaque guild ou il est autorise.
--
-- Hierarchie des roles (du plus fort au plus faible) :
--   owner     : acces total, peut gerer le RBAC (ajouter/retirer des roles)
--   admin     : acces CRUD complet sauf RBAC
--   moderator : read + writes limitees (sanctions, tickets, notes)
--   viewer    : read-only
--
-- Bootstrap : seeder les premiers `owner` en SQL direct au deploiement initial
-- (pas d'auto-promote pour eviter la prise de contr le par un nouveau membre).

-- ═══════════════════════════════════════════════════
-- Users applicatifs (identifies par leur Discord user_id)
-- ═══════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS api_users (
    discord_user_id VARCHAR(20) PRIMARY KEY,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ═══════════════════════════════════════════════════
-- Roles par (user, guild)
-- ═══════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS api_user_guilds (
    discord_user_id VARCHAR(20) NOT NULL REFERENCES api_users(discord_user_id) ON DELETE CASCADE,
    guild_id VARCHAR(20) NOT NULL,
    role TEXT NOT NULL
        CHECK (role IN ('owner', 'admin', 'moderator', 'viewer')),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    granted_by VARCHAR(20),
    PRIMARY KEY (discord_user_id, guild_id)
);

-- Lookup inverse : qui a acces a cette guild ?
CREATE INDEX IF NOT EXISTS idx_api_user_guilds_guild
    ON api_user_guilds (guild_id, role);
