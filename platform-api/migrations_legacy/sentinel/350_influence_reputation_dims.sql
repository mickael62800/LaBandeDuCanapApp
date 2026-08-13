-- Reputation multi-dimensionnelle (03.md §10) : en plus du capital scalaire
-- `reputation` (standing global, conserve pour conversions/tiers/puissance
-- d'org), on modelise 4 dimensions riches, affectees par des actions
-- distinctes et affichees au profil. Une ligne par citoyen (upsert), 0 par
-- defaut. Idempotent.
CREATE TABLE IF NOT EXISTS influence_reputation_dims (
    citizen_id   UUID PRIMARY KEY REFERENCES influence_citizens(id) ON DELETE CASCADE,
    reliability  BIGINT NOT NULL DEFAULT 0,  -- fiabilite (tenir parole, ne pas etre pris en scandale)
    popularity   BIGINT NOT NULL DEFAULT 0,  -- popularite
    notoriety    BIGINT NOT NULL DEFAULT 0,  -- notoriete (etre connu, mener des enquetes)
    transparency BIGINT NOT NULL DEFAULT 0,  -- transparence
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
