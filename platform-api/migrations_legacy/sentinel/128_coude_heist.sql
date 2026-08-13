-- Phase 10 — Systeme de braquage de la caisse communautaire.
--
-- Une fois par semaine, un joueur peut tenter de braquer la caisse
-- communautaire (cf. Phase 9 Part A). Taux de reussite de base : 5 %.
-- Chaque item de braquage actif dans l'inventaire ajoute +5 % (cap 50 %).
-- Sur succes, le voleur empoche 30-75 % de la caisse (aleatoire).
-- Sur echec, le joueur part en prison 24 h et ne peut plus rien faire.

CREATE TABLE IF NOT EXISTS coude_heist_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    success BOOLEAN NOT NULL,
    amount_stolen BIGINT NOT NULL DEFAULT 0,
    chance_percent INTEGER NOT NULL,
    tools_used TEXT[] NOT NULL DEFAULT '{}',
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Check du cooldown : la derniere tentative (reussite ou echec) decide.
CREATE INDEX IF NOT EXISTS idx_coude_heist_attempts_user_time
    ON coude_heist_attempts (guild_id, user_id, attempted_at DESC);

-- ── Prison ──
--
-- Une ligne par user en prison. `released_at` dans le futur = toujours
-- en prison. On garde la ligne apres liberation pour historique (on
-- pourra lui ajouter un `released_at_actual` plus tard si utile).
CREATE TABLE IF NOT EXISTS coude_prison (
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    released_at TIMESTAMPTZ NOT NULL,
    reason TEXT NOT NULL DEFAULT 'heist_failed',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_coude_prison_released
    ON coude_prison (guild_id, released_at);
