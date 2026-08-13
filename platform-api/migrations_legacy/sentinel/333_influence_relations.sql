-- Jeu « Influence » — Phase 5 : monde vivant & memoire.
-- Relations dirigees entre organisations (alliance / rivalite / boycott).

CREATE TABLE IF NOT EXISTS influence_org_relations (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     TEXT NOT NULL,
    org_id       UUID NOT NULL REFERENCES influence_organizations(id) ON DELETE CASCADE,
    other_org_id UUID NOT NULL REFERENCES influence_organizations(id) ON DELETE CASCADE,
    relation     TEXT NOT NULL,        -- alliance|rivalite|boycott
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (org_id, other_org_id)
);
CREATE INDEX IF NOT EXISTS idx_influence_org_relations_org
    ON influence_org_relations (org_id);

-- Config Phase 5 (parametrable) : taille du fil d'actualite.
UPDATE bot_definitions SET config_schema = config_schema || '[
    {"key":"influence_feed_size","label":"Taille du fil d actualite / archives","type":"number","required":false,"default":"10","description":"Nombre d evenements affiches par /actu et /archives."}
]'::jsonb
WHERE bot_name = 'influence-bot'
  AND NOT (config_schema @> '[{"key":"influence_feed_size"}]'::jsonb);
