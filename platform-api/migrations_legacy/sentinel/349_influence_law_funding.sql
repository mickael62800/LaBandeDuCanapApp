-- Lobbying : une organisation finance une loi depuis sa tresorerie, ajoutant du
-- poids a un camp (pour/contre) en plus des votes. Le poids de financement
-- s'additionne au poids des votes a la cloture. Idempotent.
ALTER TABLE influence_laws
    ADD COLUMN IF NOT EXISTS funding_pour BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS funding_contre BIGINT NOT NULL DEFAULT 0;
