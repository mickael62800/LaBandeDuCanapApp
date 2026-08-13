-- Phase RBAC visibility — overrides per-guild de la visibilite des composants UI
-- selon le role applicatif. Le defaut (minRole) est defini cote frontend dans
-- componentRegistry.ts ; cette table ne stocke QUE les overrides explicites.
--
-- Modele : (guild_id, component_key, role) -> visible bool
--   - guild_id : guilde concernee
--   - component_key : identifiant stable du composant (ex: "docker.prune")
--   - role : viewer | moderator | admin | owner
--   - visible : true=affiche, false=masque
-- Superadmin (defini via SUPERADMIN_USER_IDS) bypass toujours et voit tout.
--
-- Si pas de ligne pour (guild, key, role) -> on retombe sur la regle frontend
-- par defaut (role >= minRole du registry).

CREATE TABLE IF NOT EXISTS rbac_component_visibility (
    guild_id        TEXT    NOT NULL,
    component_key   TEXT    NOT NULL,
    role            TEXT    NOT NULL CHECK (role IN ('viewer','moderator','admin','owner')),
    visible         BOOLEAN NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by      TEXT,
    PRIMARY KEY (guild_id, component_key, role)
);

CREATE INDEX IF NOT EXISTS idx_rbac_visibility_guild
    ON rbac_component_visibility (guild_id);
