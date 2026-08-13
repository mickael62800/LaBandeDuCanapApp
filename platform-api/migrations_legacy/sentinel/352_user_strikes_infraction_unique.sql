-- F4 : idempotence de l'ecriture d'un strike. Un strike issu d'une action de
-- moderation est lie a cette action (infraction_id). On garantit "un seul
-- strike par action" via un index unique partiel -> combine a un ON CONFLICT
-- DO NOTHING, l'ajout d'un strike devient idempotent (pas de double strike /
-- double escalade pour une meme action, meme en cas de re-appel).
--
-- Deduplication defensive prealable (garde le plus ancien) pour ne pas faire
-- echouer la creation de l'index si d'anciens doublons existent.
DELETE FROM user_strikes a
USING user_strikes b
WHERE a.infraction_id IS NOT NULL
  AND a.infraction_id = b.infraction_id
  AND a.ctid > b.ctid;

CREATE UNIQUE INDEX IF NOT EXISTS ux_user_strikes_infraction
ON user_strikes (infraction_id)
WHERE infraction_id IS NOT NULL;
