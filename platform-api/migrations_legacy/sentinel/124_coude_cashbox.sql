-- ============================================
-- Phase 9 — Caisse communautaire Coup de Coude
-- ============================================
--
-- Probleme : l'economie Coude se contracte. Chaque coin depense au shop,
-- chaque penalite, chaque taxe sort definitivement du circuit. A terme,
-- l'economie se vide.
--
-- Solution : une caisse communautaire par guild. Tous les coins "perdus"
-- y sont reverses. Un worker hebdomadaire redistribue le contenu
-- aleatoirement aux joueurs actifs (qui ont joue dans les 7 derniers
-- jours), avec des gains disparates pour un effet fun / loterie.
--
-- Flux qui alimentent la caisse :
--   - Achats au shop (tous items, toutes protections, tous boosts)
--   - Souscriptions d'assurance combat
--   - Coup de changement de classe (500 coins)
--   - Reset stats (300 coins)
--   - Taxe sur les dons (10%)
--   - Penalite lachete (expire_combats)
--   - Commission pari-mutuel (15% du pot paris)
--
-- Les coins transferes entre joueurs lors d'un combat ne passent PAS par
-- la caisse (c'est un simple transfert peer-to-peer).

CREATE TABLE IF NOT EXISTS coude_cashbox (
    guild_id                TEXT PRIMARY KEY,
    balance                 BIGINT NOT NULL DEFAULT 0,
    total_collected         BIGINT NOT NULL DEFAULT 0,  -- cumulatif historique
    total_redistributed     BIGINT NOT NULL DEFAULT 0,  -- cumulatif historique
    last_redistribution_at  TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT coude_cashbox_balance_non_negative CHECK (balance >= 0)
);

-- Historique des redistributions (pour audit + UI web)
CREATE TABLE IF NOT EXISTS coude_cashbox_redistributions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    total_amount    BIGINT NOT NULL,
    winners_count   INT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cashbox_redistributions_guild_created
    ON coude_cashbox_redistributions(guild_id, created_at DESC);

-- Ligne individuelle par joueur gagnant
CREATE TABLE IF NOT EXISTS coude_cashbox_redistribution_entries (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    redistribution_id   UUID NOT NULL REFERENCES coude_cashbox_redistributions(id) ON DELETE CASCADE,
    user_id             TEXT NOT NULL,
    username            TEXT NOT NULL,
    amount_won          BIGINT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cashbox_entries_redistribution
    ON coude_cashbox_redistribution_entries(redistribution_id);
CREATE INDEX IF NOT EXISTS idx_cashbox_entries_user
    ON coude_cashbox_redistribution_entries(user_id, created_at DESC);
