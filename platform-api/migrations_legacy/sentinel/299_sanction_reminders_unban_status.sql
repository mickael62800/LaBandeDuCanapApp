-- BUG #1/#2 : auto-unban des bans temporaires a l'expiration.
--
-- Jusqu'ici le seul signal d'expiration etait un DM "1h avant" au moderateur
-- (status 'pending' -> 'sent'). Aucun unban Discord n'etait jamais emis, donc
-- les bans temporaires restaient permanents.
--
-- On ajoute une machine a etats SEPAREE pour l'enforcement de l'unban, qui doit
-- se declencher exactement a `expires_at`, independamment du DM "early" (et donc
-- aussi pour les bans courts <= remind_before qui n'ont jamais de DM).
--
-- `unban_status` :
--   'pending' -> a traiter (claim worker FOR UPDATE SKIP LOCKED a expires_at)
--   'done'    -> event `sanction_expired_unban` deja emis (fire-once)
--
-- Les mutes (mute_temp) utilisent le timeout natif Discord qui auto-expire :
-- le job worker filtre `action_type LIKE 'ban%'`, donc leurs lignes restent
-- 'pending' sans effet (jamais reclamees).
ALTER TABLE sanction_reminders
    ADD COLUMN IF NOT EXISTS unban_status TEXT NOT NULL DEFAULT 'pending';

CREATE INDEX IF NOT EXISTS idx_reminders_unban_pending
    ON sanction_reminders(expires_at)
    WHERE unban_status = 'pending';
