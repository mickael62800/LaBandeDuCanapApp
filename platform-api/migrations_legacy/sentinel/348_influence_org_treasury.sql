-- Tresorerie d'organisation : cagnotte commune libellee dans la monnaie
-- partagee (user_wallets). Le solde vit sur influence_organizations.treasury
-- (deja cree, migration 329). On ajoute la contrainte anti-negatif + le journal
-- append-only des mouvements. Idempotent.

DO $$ BEGIN
    ALTER TABLE influence_organizations
        ADD CONSTRAINT influence_org_treasury_non_negative CHECK (treasury >= 0);
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

CREATE TABLE IF NOT EXISTS influence_org_treasury_movements (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id       TEXT NOT NULL,
    org_id         UUID NOT NULL REFERENCES influence_organizations(id) ON DELETE CASCADE,
    kind           TEXT NOT NULL,      -- 'deposit' | 'withdrawal'
    amount         BIGINT NOT NULL,    -- toujours > 0 (le signe est porte par kind)
    treasury_after BIGINT NOT NULL,    -- solde apres le mouvement (audit)
    actor_user_id  TEXT NOT NULL,
    actor_username TEXT NOT NULL DEFAULT '',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_influence_treasury_mov_org
    ON influence_org_treasury_movements(org_id, created_at DESC);
