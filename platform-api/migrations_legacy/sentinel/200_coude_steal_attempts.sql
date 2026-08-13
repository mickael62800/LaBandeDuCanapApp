-- Phase 5 — Persistance des tentatives de vol /voler.
--
-- Avant : le bot lancait `tokio::spawn(sleep 60s)` apres /voler. Si le bot
-- redemarrait dans la fenetre de 60s, la tache mourait avec le process et
-- le vol n'etait jamais resolu (la victime gagnait par defaut sans le savoir,
-- le voleur attendait pour rien).
--
-- Apres : le bot insere une ligne ici a chaque /voler avec
-- `expires_at = now() + 60s`. Le worker `expire_steals` (coude-worker)
-- claim periodiquement les `pending` expires et publie un event Redis
-- `coude:steal_expired`. Le bot consume cet event et execute la
-- resolution comme avant.
--
-- Etats :
--   - 'pending'   : vol en attente (60s pour la victime)
--   - 'defended'  : la victime a clique le bouton -> resolution active
--   - 'expired'   : 60s ecoulees sans defense -> resolution AFK
--   - 'resolved'  : resolution terminee (etat final)

CREATE TABLE IF NOT EXISTS coude_steal_attempts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    thief_id        TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    -- Reference au message Discord poste (avec le bouton "Se defendre"),
    -- pour que le bot puisse l'editer (afficher le resultat) lors de la
    -- resolution.
    message_id      TEXT NOT NULL,
    channel_id      TEXT NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    -- Pour la resolution post-mortem (qui a defendu / quand).
    defended_at     TIMESTAMPTZ,
    resolved_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT coude_steal_attempts_status_check
        CHECK (status IN ('pending','defended','expired','resolved'))
);

-- Le worker scanne les pending expires : index sur (status, expires_at).
CREATE INDEX IF NOT EXISTS idx_coude_steal_attempts_pending_expiry
    ON coude_steal_attempts (status, expires_at)
    WHERE status = 'pending';

-- Lookup par message_id (le bouton "Se defendre" inclut un custom_id qui
-- contient le message_id pour retrouver le row).
CREATE INDEX IF NOT EXISTS idx_coude_steal_attempts_message
    ON coude_steal_attempts (message_id);
