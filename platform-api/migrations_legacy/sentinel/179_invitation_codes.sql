-- Phase invitation : codes a usage unique pour onboarder de nouveaux users.
-- Workflow :
--   1. Owner/admin genere un code via /api/invitations (associe a guild + role)
--   2. Owner partage le code avec l'utilisateur cible (par DM Discord, etc.)
--   3. User va sur le site, colle le code, fait login Discord
--   4. API redeem : valide code -> insert api_user_guilds(user, guild, role)
--      -> marque code used_at + used_by_discord_id
--   5. User a maintenant acces

CREATE TABLE IF NOT EXISTS invitation_codes (
    code TEXT PRIMARY KEY,
    guild_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('viewer', 'moderator', 'admin', 'owner')),
    created_by TEXT NOT NULL,         -- discord_user_id du createur
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,            -- NULL = pas d'expiration
    used_at TIMESTAMPTZ,               -- NULL = non utilise
    used_by_discord_id TEXT,           -- NULL si non utilise
    notes TEXT                         -- ex: "Pour Bob, modo backup"
);

CREATE INDEX IF NOT EXISTS idx_invitation_codes_guild ON invitation_codes (guild_id);
CREATE INDEX IF NOT EXISTS idx_invitation_codes_unused
    ON invitation_codes (used_at) WHERE used_at IS NULL;
