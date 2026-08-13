-- Phase RBAC granulaire — table d'overrides du min_role par composant
-- sensible (purge DB, reset wallets, etc.). Sert UNIQUEMENT pour les
-- actions gates par `check_component_role` cote API. La table existante
-- `rbac_component_visibility` reste pour le masquage UI uniquement.
--
-- Modele:
--   - default_role et floor_role sont definis en code (component_gates.rs
--     cote API + componentRegistry.ts cote front). Ils ne sont PAS stockes
--     ici pour eviter la divergence : la source de verite des defauts est
--     le code.
--   - Cette table ne stocke QUE les overrides explicites par guild.
--   - L'API verifie au moment du gate :
--       1. Si override -> applique min(override, floor) (le floor protege
--          contre un override accidentel trop permissif)
--       2. Sinon -> default du code
--
-- Roles valides : viewer, moderator, admin, owner (cf. systeme RBAC actuel).
CREATE TABLE IF NOT EXISTS rbac_component_min_role (
    guild_id        VARCHAR(20) NOT NULL,
    component_key   VARCHAR(100) NOT NULL,
    min_role        VARCHAR(20) NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by      VARCHAR(20),
    PRIMARY KEY (guild_id, component_key),
    CONSTRAINT chk_rbac_min_role CHECK (min_role IN ('viewer', 'moderator', 'admin', 'owner'))
);

CREATE INDEX IF NOT EXISTS idx_rbac_component_min_role_guild
    ON rbac_component_min_role(guild_id);
