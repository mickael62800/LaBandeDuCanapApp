-- Coup de Coude v2 : systeme de dons et historique

CREATE TABLE IF NOT EXISTS coude_dons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    donor_id TEXT NOT NULL,
    receiver_id TEXT NOT NULL,
    don_type TEXT NOT NULL,       -- 'coins' ou item_key
    quantity INTEGER NOT NULL,
    tax INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_coude_dons_guild ON coude_dons (guild_id);
CREATE INDEX IF NOT EXISTS idx_coude_dons_donor ON coude_dons (guild_id, donor_id);
