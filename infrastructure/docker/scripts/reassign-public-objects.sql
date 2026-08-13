\set ON_ERROR_STOP on

-- Transfere uniquement les objets applicatifs du schema public. Un
-- `REASSIGN OWNED` global touche aussi les objets fournis par PostgreSQL ou
-- par des extensions et echoue notamment quand l'ancien proprietaire est le
-- superuser d'initialisation du cluster.

SELECT format(
    'ALTER %s %I.%I OWNER TO %I',
    CASE c.relkind
        WHEN 'r' THEN 'TABLE'
        WHEN 'p' THEN 'TABLE'
        WHEN 'S' THEN 'SEQUENCE'
        WHEN 'v' THEN 'VIEW'
        WHEN 'm' THEN 'MATERIALIZED VIEW'
        WHEN 'f' THEN 'FOREIGN TABLE'
    END,
    n.nspname,
    c.relname,
    :'target_role'
)
FROM pg_catalog.pg_class AS c
JOIN pg_catalog.pg_namespace AS n ON n.oid = c.relnamespace
WHERE n.nspname = 'public'
  AND c.relkind IN ('r', 'p', 'S', 'v', 'm', 'f')
  -- Une sequence SERIAL/IDENTITY est liee a sa colonne par une dependance
  -- AUTO/INTERNAL. PostgreSQL interdit de changer son proprietaire
  -- directement ; ALTER TABLE ... OWNER la transfere avec sa table.
  -- Les sequences autonomes, elles, doivent toujours etre traitees ici.
  AND NOT (
      c.relkind = 'S'
      AND EXISTS (
          SELECT 1
          FROM pg_catalog.pg_depend AS owned
          WHERE owned.classid = 'pg_catalog.pg_class'::regclass
            AND owned.objid = c.oid
            AND owned.refclassid = 'pg_catalog.pg_class'::regclass
            AND owned.deptype IN ('a', 'i')
      )
  )
  AND NOT EXISTS (
      SELECT 1
      FROM pg_catalog.pg_depend AS d
      WHERE d.classid = 'pg_catalog.pg_class'::regclass
        AND d.objid = c.oid
        AND d.deptype = 'e'
  )
ORDER BY c.relkind, c.relname
\gexec

SELECT format(
    'ALTER %s %I.%I(%s) OWNER TO %I',
    CASE p.prokind WHEN 'p' THEN 'PROCEDURE' ELSE 'FUNCTION' END,
    n.nspname,
    p.proname,
    pg_catalog.pg_get_function_identity_arguments(p.oid),
    :'target_role'
)
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
\gexec

SELECT format(
    'ALTER %s %I.%I OWNER TO %I',
    CASE t.typtype WHEN 'd' THEN 'DOMAIN' ELSE 'TYPE' END,
    n.nspname,
    t.typname,
    :'target_role'
)
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
\gexec
