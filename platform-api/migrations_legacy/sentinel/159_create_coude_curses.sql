-- Migration 159 : Maledictions (cf. COUPE_AMELIORATIONS section 5.1).
--
-- Un joueur peut "maudire" un autre pour 300c pendant 24h. Six types de
-- maledictions ridicules (poulet, banane, portefeuille troue, lenteur,
-- insomnie, malchance amoureuse). Une seule active par cible/guild.
-- La cible peut lever en payant le double a l auteur.

CREATE TABLE IF NOT EXISTS coude_curses (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    VARCHAR(20) NOT NULL,
    target_id   VARCHAR(20) NOT NULL,
    source_id   VARCHAR(20) NOT NULL,
    kind        VARCHAR(40) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL,
    lifted_at   TIMESTAMPTZ,
    lifted_by   VARCHAR(20)
);

-- Une seule curse active par couple (guild, target). On considere
-- "active" = lifted_at IS NULL ; l expiration est filtree au runtime
-- via expires_at > NOW() pour eviter de partial-indexer sur NOW().
-- Cet index sert aussi de lookup principal (par target).
CREATE UNIQUE INDEX IF NOT EXISTS uniq_coude_curses_one_active_per_target
    ON coude_curses (guild_id, target_id)
    WHERE lifted_at IS NULL;

-- Lookup historique par auteur (pour stats / list_active_by_source).
CREATE INDEX IF NOT EXISTS idx_coude_curses_source
    ON coude_curses (guild_id, source_id, created_at DESC);
