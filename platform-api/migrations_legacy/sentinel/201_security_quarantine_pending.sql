-- Phase 5F — Persistance des quarantaines en attente de captcha.
--
-- Avant : background.rs avait une boucle 30s qui scannait un
-- `QuarantineManager` in-memory pour kicker les users qui n'avaient pas
-- valide le captcha apres `captcha_timeout_secs`. Si le bot redemarrait,
-- le tracker RAM etait perdu et les users restaient indefiniment dans
-- l'etat quarantaine sans jamais etre kickes.
--
-- Apres : le bot insere une ligne ici a chaque mise en quarantaine. Le
-- worker `kick_expired_quarantine` scanne les expires et publie un
-- event Redis `quarantine_expired` que le bot consume pour appeler
-- `guild.kick(...)`. Resilient aux redemarrages.
--
-- Note : le QuarantineManager garde toujours en RAM les `original_roles`
-- pour la restauration des roles a la verification du captcha (autre
-- flow non concerne par ce timer). C'est OK, les deux structures sont
-- decouplees : kick = juste (guild_id, user_id), restauration = roles.

CREATE TABLE IF NOT EXISTS security_quarantine_pending (
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_security_quarantine_expires
    ON security_quarantine_pending (expires_at);
