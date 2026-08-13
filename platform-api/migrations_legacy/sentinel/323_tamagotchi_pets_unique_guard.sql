-- ============================================
-- Integrite des donnees Tamagotchi : un seul compagnon par (guild_id, owner_id).
--
-- La migration 255 a cree la table `pets` avec `CREATE TABLE IF NOT EXISTS ...
-- UNIQUE (guild_id, owner_id)`. Si une table `pets` preexistait, la contrainte
-- UNIQUE a ete silencieusement ignoree : des doublons (guild_id, owner_id) ont
-- alors pu apparaitre. Cette migration :
--   1. deduplique les eventuels doublons existants (en gardant le compagnon
--      vivant le plus recent) ;
--   2. garantit la presence de la contrainte UNIQUE si elle manque.
--
-- Idempotente : rejouable sans effet de bord.
-- ============================================

-- 1) Dedup : pour chaque (guild_id, owner_id) on garde UNE seule ligne.
--    Preference : compagnons vivants d'abord (status <> 'dead'), puis le plus
--    recemment mis a jour. On ne supprime jamais l'unique compagnon d'un
--    joueur (rang 1 toujours conserve).
DELETE FROM pets p
USING (
    SELECT id
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                   PARTITION BY guild_id, owner_id
                   ORDER BY (status = 'dead'), updated_at DESC, created_at DESC
               ) AS rn
        FROM pets
    ) ranked
    WHERE rn > 1
) dups
WHERE p.id = dups.id;

-- 2) Ajoute la contrainte UNIQUE si aucun index/contrainte unique ne couvre
--    deja EXACTEMENT (guild_id, owner_id). On verifie par ensemble de colonnes
--    (la migration 255 a pu creer une contrainte au nom auto-genere), pas par
--    nom, pour ne pas dupliquer une contrainte existante.
DO $$
DECLARE
    guild_col_attnum smallint;
    owner_col_attnum smallint;
    has_unique boolean;
BEGIN
    SELECT attnum INTO guild_col_attnum
    FROM pg_attribute
    WHERE attrelid = 'pets'::regclass AND attname = 'guild_id';

    SELECT attnum INTO owner_col_attnum
    FROM pg_attribute
    WHERE attrelid = 'pets'::regclass AND attname = 'owner_id';

    SELECT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'pets'::regclass
          AND contype IN ('u', 'p')
          AND conkey @> ARRAY[guild_col_attnum, owner_col_attnum]
          AND conkey <@ ARRAY[guild_col_attnum, owner_col_attnum]
    ) INTO has_unique;

    IF NOT has_unique THEN
        ALTER TABLE pets
            ADD CONSTRAINT pets_guild_owner_unique UNIQUE (guild_id, owner_id);
    END IF;
END $$;
