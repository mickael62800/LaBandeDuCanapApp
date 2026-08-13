-- Verification d'age au reglement.
--
-- A l'arrivee, le membre recoit le role "Membre temporaire" (unverified_role_id)
-- qui ne voit que le salon reglement. En cliquant sur "J'accepte", un formulaire
-- demande son age :
--   - age >= age_minimum  -> retire le role temporaire, donne le role Membre
--                            (rules_role_id existant).
--   - age <  age_minimum  -> ban temporaire jusqu'a ses age_minimum ans
--                            (duree = (age_minimum - age) annees).
--
-- Le deban est gere par un job worker mensuel qui scanne age_verification_bans.

-- 1) Nouveaux reglages sur welcome_config (parametrables via le dashboard).
ALTER TABLE welcome_config
    ADD COLUMN IF NOT EXISTS age_check_enabled   BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS age_minimum         INT     NOT NULL DEFAULT 20,
    ADD COLUMN IF NOT EXISTS unverified_role_id  TEXT,
    ADD COLUMN IF NOT EXISTS age_modal_question  TEXT NOT NULL DEFAULT 'Quel age as-tu ? (en chiffres)',
    ADD COLUMN IF NOT EXISTS age_ban_message     TEXT NOT NULL DEFAULT 'Tu dois avoir au moins {min} ans pour rejoindre ce serveur. Tu pourras revenir dans {annees} an(s).';

-- 2) Table de suivi des bans d'age (source de verite du job d'unban).
CREATE TABLE IF NOT EXISTS age_verification_bans (
    id            UUID PRIMARY KEY,
    guild_id      TEXT NOT NULL,
    user_id       TEXT NOT NULL,
    declared_age  INT  NOT NULL,
    banned_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    unban_at      TIMESTAMPTZ NOT NULL,          -- date a partir de laquelle on debannit
    status        TEXT NOT NULL DEFAULT 'pending', -- 'pending' | 'lifted'
    lifted_at     TIMESTAMPTZ
);

-- Scan du job : WHERE status = 'pending' AND unban_at <= NOW()
CREATE INDEX IF NOT EXISTS idx_age_bans_pending_unban
    ON age_verification_bans (unban_at)
    WHERE status = 'pending';

-- Recherche par membre (verifier si deja banni / historique).
CREATE INDEX IF NOT EXISTS idx_age_bans_guild_user
    ON age_verification_bans (guild_id, user_id);
