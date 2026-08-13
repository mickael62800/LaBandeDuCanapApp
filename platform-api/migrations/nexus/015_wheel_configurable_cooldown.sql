-- 015_wheel_configurable_cooldown.sql
--
-- Rendre le delai entre deux tirages reellement configurable.
--
-- La table etait verrouillee par une cle primaire (guild_id, user_id, day) :
-- au plus une ligne par jour et par joueur. Un delai inferieur a 24 heures
-- etait donc IMPOSSIBLE — le deuxieme tirage entrait en conflit sur la cle
-- et etait silencieusement rejete, exactement comme un « deja tire ».
--
-- Le reglage `wheel_cooldown_hours` aurait donc ete affiche sans effet en
-- dessous de 24. On passe a un journal de tirages : une ligne par tirage,
-- et le delai se verifie sur la date.
--
-- La colonne `day` est conservee : elle ne sert plus de contrainte mais
-- reste lisible, et la supprimer casserait toute requete existante.

-- Chaque tirage devient une ligne a part entiere.
ALTER TABLE nexus_wheel_daily_claims
    DROP CONSTRAINT IF EXISTS nexus_wheel_daily_claims_pkey;

ALTER TABLE nexus_wheel_daily_claims
    ADD COLUMN IF NOT EXISTS id uuid DEFAULT gen_random_uuid();

-- Les lignes anterieures n'ont pas d'identifiant : on leur en donne un avant
-- de le rendre obligatoire.
UPDATE nexus_wheel_daily_claims SET id = gen_random_uuid() WHERE id IS NULL;

ALTER TABLE nexus_wheel_daily_claims
    ALTER COLUMN id SET NOT NULL;

ALTER TABLE nexus_wheel_daily_claims
    ADD CONSTRAINT nexus_wheel_daily_claims_pkey PRIMARY KEY (id);

-- Requete dominante : « ce joueur a-t-il tire depuis N heures ». Le tri
-- descendant sur la date evite un balayage quand un joueur accumule les
-- tirages sur la duree.
CREATE INDEX IF NOT EXISTS idx_nexus_wheel_claims_recent
    ON nexus_wheel_daily_claims USING btree (guild_id, user_id, claimed_at DESC);
