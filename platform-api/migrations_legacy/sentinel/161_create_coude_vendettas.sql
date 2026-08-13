-- Migration 161 : Vendetta (cf. COUPE_AMELIORATIONS section 5.3).
--
-- Apres avoir perdu un combat contre X, le challenger peut declarer
-- une vendetta. Dans les 7 jours :
--   - S il gagne la revanche, son gain est double.
--   - S il perd, X est renomme "Bourreau de @challenger" pour 7 jours.
--
-- Une seule vendetta active par couple (guild, challenger, target).

CREATE TABLE IF NOT EXISTS coude_vendettas (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id      VARCHAR(20) NOT NULL,
    challenger_id VARCHAR(20) NOT NULL,
    target_id     VARCHAR(20) NOT NULL,
    declared_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ NOT NULL,
    status        VARCHAR(16) NOT NULL DEFAULT 'active',
    resolved_at   TIMESTAMPTZ,
    CHECK (status IN ('active', 'won', 'lost', 'expired'))
);

-- Lookup principal : vendetta active sur un couple.
CREATE INDEX IF NOT EXISTS idx_coude_vendettas_pair_active
    ON coude_vendettas (guild_id, challenger_id, target_id)
    WHERE status = 'active';

-- Lookup par challenger pour /profil ("vendettas en cours").
CREATE INDEX IF NOT EXISTS idx_coude_vendettas_challenger
    ON coude_vendettas (guild_id, challenger_id, declared_at DESC);

-- Lookup par target pour /profil ("rancunes contre toi").
CREATE INDEX IF NOT EXISTS idx_coude_vendettas_target
    ON coude_vendettas (guild_id, target_id, declared_at DESC);

-- Garde-fou : une seule vendetta active par couple ordonne.
CREATE UNIQUE INDEX IF NOT EXISTS uniq_coude_vendettas_one_active_per_pair
    ON coude_vendettas (guild_id, challenger_id, target_id)
    WHERE status = 'active';
