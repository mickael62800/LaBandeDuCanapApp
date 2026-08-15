\set ON_ERROR_STOP on

-- Transfere uniquement les objets applicatifs du schema public. Un
-- `REASSIGN OWNED` global touche aussi les objets fournis par PostgreSQL ou
-- par des extensions et echoue notamment quand l'ancien proprietaire est le
-- superuser d'initialisation du cluster.

DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT format(
            'ALTER %s %I.%I OWNER TO %I',
            CASE c.relkind
                WHEN 'r' THEN 'TABLE'
                WHEN 'p' THEN 'TABLE'
                WHEN 'v' THEN 'VIEW'
                WHEN 'm' THEN 'MATERIALIZED VIEW'
                WHEN 'f' THEN 'FOREIGN TABLE'
            END,
            n.nspname,
            c.relname,
            :'target_role'
        ) AS stmt
        FROM pg_catalog.pg_class AS c
        JOIN pg_catalog.pg_namespace AS n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public'
          AND c.relkind IN ('r', 'p', 'v', 'm', 'f')
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_depend AS d
              WHERE d.classid = 'pg_catalog.pg_class'::regclass
                AND d.objid = c.oid
                AND d.deptype = 'e'
          )
        ORDER BY c.relkind, c.relname
    LOOP
        BEGIN
            EXECUTE r.stmt;
        EXCEPTION WHEN concurrent_transaction_relationship_object_reorganization OR others THEN
            RAISE NOTICE 'Erreur ignorée sur stmt: % (%)', r.stmt, SQLERRM;
        END;
    END LOOP;
END $$;

DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT format(
            'ALTER %s %I.%I(%s) OWNER TO %I',
            CASE p.prokind WHEN 'p' THEN 'PROCEDURE' ELSE 'FUNCTION' END,
            n.nspname,
            p.proname,
            pg_catalog.pg_get_function_identity_arguments(p.oid),
            :'target_role'
        ) AS stmt
        FROM pg_catalog.pg_proc AS p
        JOIN pg_catalog.pg_namespace AS n ON n.oid = p.pronamespace
        WHERE n.nspname = 'public'
          AND p.prokind IN ('f', 'p', 'w')
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_depend AS d
              WHERE d.classid = 'pg_catalog.pg_proc'::regclass
                AND d.objid = p.oid
                AND d.deptype = 'e'
          )
        ORDER BY p.proname, p.oid
    LOOP
        BEGIN
            EXECUTE r.stmt;
        EXCEPTION WHEN others THEN
            RAISE NOTICE 'Erreur ignorée sur stmt: % (%)', r.stmt, SQLERRM;
        END;
    END LOOP;
END $$;

DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT format(
            'ALTER %s %I.%I OWNER TO %I',
            CASE t.typtype WHEN 'd' THEN 'DOMAIN' ELSE 'TYPE' END,
            n.nspname,
            t.typname,
            :'target_role'
        ) AS stmt
        FROM pg_catalog.pg_type AS t
        JOIN pg_catalog.pg_namespace AS n ON n.oid = t.typnamespace
        WHERE n.nspname = 'public'
          AND t.typtype IN ('d', 'e')
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_depend AS d
              WHERE d.classid = 'pg_catalog.pg_type'::regclass
                AND d.objid = t.oid
                AND d.deptype = 'e'
          )
        ORDER BY t.typname
    LOOP
        BEGIN
            EXECUTE r.stmt;
        EXCEPTION WHEN others THEN
            RAISE NOTICE 'Erreur ignorée sur stmt: % (%)', r.stmt, SQLERRM;
        END;
    END LOOP;
END $$;
