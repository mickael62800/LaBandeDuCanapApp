-- Re-cle les bannissements de salon vocal sur (guild_id, owner_id, banned_user_id)
-- au lieu de l'UUID ephemere du salon. Objectif : un ban doit survivre a la
-- suppression/recreation du salon temporaire du proprietaire (issue #2).
--
-- Avant : voice_channel_id avait une FK ON DELETE CASCADE -> les bans etaient
-- supprimes avec le salon vide, donc trivialement contournables. On retire la
-- cascade et on garde voice_channel_id comme reference historique (best-effort).

-- 1. Nouvelles colonnes stables par proprietaire.
ALTER TABLE voice_channel_bans ADD COLUMN IF NOT EXISTS guild_id TEXT;
ALTER TABLE voice_channel_bans ADD COLUMN IF NOT EXISTS owner_id TEXT;

-- 2. Backfill best-effort depuis les salons encore presents.
UPDATE voice_channel_bans b
SET guild_id = c.guild_id,
    owner_id = c.owner_id
FROM voice_channels c
WHERE b.voice_channel_id = c.id
  AND (b.guild_id IS NULL OR b.owner_id IS NULL);

-- 3. Supprimer la FK cascade : les bans ne doivent plus disparaitre avec le
--    salon. voice_channel_id devient une simple reference historique nullable.
ALTER TABLE voice_channel_bans DROP CONSTRAINT IF EXISTS voice_channel_bans_voice_channel_id_fkey;
ALTER TABLE voice_channel_bans ALTER COLUMN voice_channel_id DROP NOT NULL;

-- 4. Remplacer l'unicite par (guild_id, owner_id, user_id).
--    Les lignes legacy sans owner (NULL) restent tolerees : en Postgres les
--    NULL sont distincts, elles n'entrent donc pas en conflit. Elles ne seront
--    simplement jamais retrouvees par les requetes par proprietaire.
ALTER TABLE voice_channel_bans DROP CONSTRAINT IF EXISTS voice_channel_bans_voice_channel_id_user_id_key;
ALTER TABLE voice_channel_bans
    ADD CONSTRAINT voice_channel_bans_owner_user_key UNIQUE (guild_id, owner_id, user_id);

-- 5. Index pour la lecture par proprietaire (re-application a la creation).
CREATE INDEX IF NOT EXISTS idx_voice_bans_owner ON voice_channel_bans (guild_id, owner_id);
