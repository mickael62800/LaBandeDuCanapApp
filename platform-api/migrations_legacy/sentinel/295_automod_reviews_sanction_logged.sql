-- C1 : anti double-strike sur les détections sévères auto-protégées.
--
-- Quand l'auto-protection sévère (raid / phishing / pub Discord / gros flood)
-- mute un membre, elle journalise DÉJÀ une sanction (qui compte dans l'escalade
-- de strikes) AVANT de poster la carte de review. Si un admin finalise ensuite
-- cette carte, la sanction était re-journalisée -> un incident = deux strikes.
--
-- Ce flag, posé à la création de la carte lorsque l'auto-protection a déjà
-- tracé une sanction, permet à la finalisation de NE PAS re-journaliser.
ALTER TABLE automod_reviews
    ADD COLUMN IF NOT EXISTS sanction_logged BOOLEAN NOT NULL DEFAULT false;
