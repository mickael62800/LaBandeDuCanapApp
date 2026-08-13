-- Ajoute deux toggles dedies :
-- - rename_enabled : active le rename auto des pseudos sur seuil de streak
-- - messages_enabled : active le post des embeds de raillerie dans le salon
--
-- Permet de mixer : uniquement messages, uniquement rename, les deux, ou rien
-- (le toggle global `enabled` reste le kill-switch principal).
--
-- Default TRUE pour conserver le comportement existant.

ALTER TABLE coude_taunts_config
    ADD COLUMN IF NOT EXISTS rename_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS messages_enabled BOOLEAN NOT NULL DEFAULT TRUE;
