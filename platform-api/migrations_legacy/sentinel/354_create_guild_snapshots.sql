-- Sauvegarde / restauration de serveur Discord (domaine `guild_backup`).
--
-- A NE PAS confondre avec les analytics (`audit_snapshots` / activite
-- quotidienne). Ici on stocke la STRUCTURE complete d'un serveur (roles,
-- categories, salons, overwrites, reglages, bans, emojis, mapping
-- membre->roles) capturee par le bot, versionnee, pour pouvoir la restaurer
-- sur un serveur neuf.
--
-- Le payload complet (GuildSnapshot serialise) vit en JSONB. Les colonnes
-- promues (label, schema_version, created_by, created_at) dupliquent des
-- champs de meta.* pour lister sans desérialiser le payload (perf).

CREATE TABLE IF NOT EXISTS guild_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    label TEXT,
    schema_version INT NOT NULL DEFAULT 1,
    created_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- GuildSnapshot serialise (contrat serde partage bot<->api).
    payload JSONB NOT NULL
);

-- Liste par guild, du plus recent au plus ancien (cf. list/oldest_id du repo).
CREATE INDEX IF NOT EXISTS idx_guild_snapshots_guild_created
    ON guild_snapshots (guild_id, created_at DESC);
