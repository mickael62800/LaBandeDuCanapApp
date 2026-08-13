-- Phase 9 Part C — Boost voleur en abonnements temps-base.
--
-- Symmetrique de `coude_steal_protections` (Part B) : au lieu de reduire
-- la chance de vol d'une cible, ces items augmentent la chance de reussite
-- du voleur en ajoutant un bonus plat a son roll.
--
-- Chaque item a son `roll_bonus` propre (defini dans le domain), et
-- plusieurs items actifs se cumulent (somme des bonus).
--
-- Items : Crochet (+5), Passe-partout (+10), Deguisement (+15),
-- Fumigene (+20), Marteau (+25).
--
-- Invisible a la victime : le voleur active en silence via /boost-voleur,
-- la victime ne sait pas que son attaquant a rate/reussi avec un bonus.

CREATE TABLE IF NOT EXISTS coude_steal_boosts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    item_key TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_coude_steal_boosts_unique
  ON coude_steal_boosts (guild_id, user_id, item_key);

CREATE INDEX idx_coude_steal_boosts_active
  ON coude_steal_boosts (guild_id, user_id, expires_at);
