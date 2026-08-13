-- Migration 053 : Persistance des features bots vers l'API
-- Ajoute les colonnes et tables necessaires pour que toutes les donnees
-- des bots soient visibles dans l'application desktop.

-- ═══════════════════════════════════════════════════
-- 1. Streaks dans user_levels (Progression Bot)
-- ═══════════════════════════════════════════════════

ALTER TABLE user_levels ADD COLUMN IF NOT EXISTS streak_current INTEGER NOT NULL DEFAULT 0;
ALTER TABLE user_levels ADD COLUMN IF NOT EXISTS streak_best INTEGER NOT NULL DEFAULT 0;
ALTER TABLE user_levels ADD COLUMN IF NOT EXISTS streak_last_day INTEGER NOT NULL DEFAULT 0;
ALTER TABLE user_levels ADD COLUMN IF NOT EXISTS streak_last_year INTEGER NOT NULL DEFAULT 0;

-- ═══════════════════════════════════════════════════
-- 2. SLA et satisfaction dans tickets (Ticket Bot)
-- ═══════════════════════════════════════════════════

ALTER TABLE tickets ADD COLUMN IF NOT EXISTS first_response_at TIMESTAMPTZ;
ALTER TABLE tickets ADD COLUMN IF NOT EXISTS resolved_at TIMESTAMPTZ;
ALTER TABLE tickets ADD COLUMN IF NOT EXISTS satisfaction_rating INTEGER;

-- ═══════════════════════════════════════════════════
-- 3. Parrainages (Community Bot)
-- ═══════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS sponsorships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    sponsor_id TEXT NOT NULL,
    sponsored_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (guild_id, sponsored_id)
);

CREATE INDEX IF NOT EXISTS idx_sponsorships_guild ON sponsorships (guild_id);
CREATE INDEX IF NOT EXISTS idx_sponsorships_sponsor ON sponsorships (guild_id, sponsor_id);

-- ═══════════════════════════════════════════════════
-- 4. Roles temporaires (Community Bot)
-- ═══════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS temp_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (guild_id, user_id, role_id)
);

CREATE INDEX IF NOT EXISTS idx_temp_roles_guild ON temp_roles (guild_id);
CREATE INDEX IF NOT EXISTS idx_temp_roles_expires ON temp_roles (expires_at);

-- ═══════════════════════════════════════════════════
-- 5. Actions en attente / mode apprenti (Moderation Bot)
-- ═══════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS pending_mod_actions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    moderator_id TEXT NOT NULL,
    moderator_name TEXT NOT NULL,
    target_id TEXT NOT NULL,
    target_name TEXT NOT NULL,
    action_type TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    gravity TEXT,
    duration BIGINT,
    status TEXT NOT NULL DEFAULT 'pending',
    reviewed_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pending_mod_guild ON pending_mod_actions (guild_id);
CREATE INDEX IF NOT EXISTS idx_pending_mod_status ON pending_mod_actions (guild_id, status);
