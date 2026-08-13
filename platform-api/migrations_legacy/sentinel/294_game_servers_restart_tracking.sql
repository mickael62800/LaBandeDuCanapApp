-- Suivi des redemarrages auto (auto-restart with backoff) des serveurs
-- Game Portal. `restart_attempts` compte les redemarrages auto consecutifs
-- apres crash (remis a 0 a la recuperation) ; `last_restart_at` horodate le
-- dernier essai pour le calcul du backoff exponentiel. Borne stricte cote
-- code (MAX_RESTART_ATTEMPTS) -> pas de crash loop.
ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS restart_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS last_restart_at TIMESTAMPTZ;
