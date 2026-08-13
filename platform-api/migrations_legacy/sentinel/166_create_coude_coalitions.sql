-- Migration 166 : Coalitions (cf. COUPE_AMELIORATIONS 5.3).
--
-- 3+ joueurs se liguent contre une cible. Chacun paye 500c. La cible
-- subit -20% sur tous ses gains pendant 48h, OU jusqu a ce qu elle
-- batte UN des conspirateurs en combat direct.

CREATE TABLE IF NOT EXISTS coude_coalitions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     VARCHAR(20) NOT NULL,
    target_id    VARCHAR(20) NOT NULL,
    opened_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at   TIMESTAMPTZ NOT NULL,
    status       VARCHAR(16) NOT NULL DEFAULT 'forming',
    broken_by    VARCHAR(20),
    broken_at    TIMESTAMPTZ,
    CHECK (status IN ('forming', 'active', 'broken', 'expired'))
);

CREATE TABLE IF NOT EXISTS coude_coalition_members (
    coalition_id   UUID NOT NULL REFERENCES coude_coalitions(id) ON DELETE CASCADE,
    member_id      VARCHAR(20) NOT NULL,
    member_name    VARCHAR(100) NOT NULL,
    joined_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (coalition_id, member_id)
);

CREATE INDEX IF NOT EXISTS idx_coude_coalitions_target_active
    ON coude_coalitions (guild_id, target_id)
    WHERE status IN ('forming', 'active');

CREATE UNIQUE INDEX IF NOT EXISTS uniq_coude_coalitions_one_active_per_target
    ON coude_coalitions (guild_id, target_id)
    WHERE status IN ('forming', 'active');

CREATE INDEX IF NOT EXISTS idx_coude_coalition_members_member
    ON coude_coalition_members (member_id);
