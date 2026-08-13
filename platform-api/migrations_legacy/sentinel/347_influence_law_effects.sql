-- Effets reels des lois : une loi ADOPTEE peut fixer un reglage gameplay
-- (whitelist cote code : influence_investigation_cost, org_creation_cost,
-- org_role_cost, scandal_reputation_loss, investigation_success_pct,
-- law_debate_hours). `effect_key` NULL = loi purement narrative (aucun effet).
-- Idempotent.
ALTER TABLE influence_laws
    ADD COLUMN IF NOT EXISTS effect_key TEXT,
    ADD COLUMN IF NOT EXISTS effect_value BIGINT;
