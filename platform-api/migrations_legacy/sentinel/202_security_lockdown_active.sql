-- Phase 5G — Persistance des lockdowns actifs.
--
-- Le bot sauvegarde les permission_overwrites originaux par salon en RAM
-- pour pouvoir les restaurer a la fin du lockdown. Si le bot redemarre,
-- l'etat est perdu et le lockdown reste indefiniment actif.
--
-- Maintenant : a chaque activation, le bot serialise les overwrites
-- originaux en JSON et persiste ici. Le worker scanne les expires et
-- publie un event `lockdown_expired` avec le JSON. Le bot consume,
-- desserialise et restaure les permissions.
--
-- saved_states : JSON array de
--   {channel_id, allow, deny, kind, target_id, had_original}
--
-- - had_original = true  -> il y avait un overwrite avant, restaure-le
-- - had_original = false -> on a cree l'overwrite, supprime-le

CREATE TABLE IF NOT EXISTS security_lockdown_active (
    guild_id     TEXT PRIMARY KEY,
    saved_states JSONB NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_security_lockdown_expires
    ON security_lockdown_active (expires_at);
