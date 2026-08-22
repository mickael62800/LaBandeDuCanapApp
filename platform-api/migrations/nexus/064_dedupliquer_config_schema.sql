-- 064_dedupliquer_config_schema.sql
--
-- Le formulaire de configuration affichait plusieurs reglages EN DOUBLE :
-- « Generer les structures », « Type de monde », « Rayon max du monde » et
-- « Nether accessible » apparaissaient deux fois dans la section Monde de
-- Minecraft, avec deux libelles et deux descriptions pour la meme variable.
--
-- CAUSE. La migration 012 ajoute des reglages avec
--   `config_schema = config_schema || '[...]'::jsonb`
-- soit une CONCATENATION de tableaux, pas une fusion par cle. Or 009 avait
-- deja pose LEVEL_TYPE, GENERATE_STRUCTURES, MAX_WORLD_SIZE et ALLOW_NETHER.
-- Les deux jeux d'entrees coexistent depuis.
--
-- Consequence au-dela de l'affichage : les deux champs ecrivent la MEME cle
-- dans `game_server_configs`. Le dernier rendu gagne, en silence. Un
-- exploitant qui desactivait le Nether sur le premier interrupteur pouvait le
-- voir se rallumer sans que rien ne le lui explique.
--
-- CORRECTION. 012 est deja appliquee et n'est pas modifiee : on reconstruit
-- ici le schema en ne gardant, pour chaque cle, que la DERNIERE occurrence —
-- celle des migrations recentes, la mieux renseignee (`group`, `description`,
-- bornes), la ou les entrees de 009 n'avaient ni section ni description.
--
-- La place d'origine de la cle est conservee : reordonner le formulaire n'est
-- pas le sujet, et deplacer un reglage que les exploitants ont appris a
-- trouver serait un second changement, non demande.

WITH champs AS (
    SELECT
        gt.id,
        e.element,
        e.ord,
        -- Les entrees sans `key` sont inexploitables par le formulaire, mais on
        -- ne les supprime pas au passage : une cle de repli unique par position
        -- les empeche d'etre fusionnees entre elles, donc de disparaitre.
        COALESCE(e.element ->> 'key', 'sans-cle::' || e.ord::text) AS cle
    FROM game_templates gt
    CROSS JOIN LATERAL
        jsonb_array_elements(gt.config_schema) WITH ORDINALITY AS e(element, ord)
    WHERE gt.config_schema IS NOT NULL
      AND jsonb_typeof(gt.config_schema) = 'array'
),
classement AS (
    SELECT
        id,
        element,
        cle,
        -- 1 = derniere occurrence de la cle : c'est celle que l'on garde.
        row_number() OVER (PARTITION BY id, cle ORDER BY ord DESC) AS rang,
        -- ...mais a la place de la premiere, pour ne pas remuer le formulaire.
        min(ord) OVER (PARTITION BY id, cle) AS place
    FROM champs
),
schemas AS (
    SELECT id, jsonb_agg(element ORDER BY place) AS schema
    FROM classement
    WHERE rang = 1
    GROUP BY id
),
-- Ne toucher que les templates reellement affectes : sans ce garde-fou, la
-- migration reecrirait les quatorze schemas pour n'en corriger qu'un, et
-- ferait mentir `updated_at` sur les treize autres.
affectes AS (
    SELECT DISTINCT id
    FROM classement
    WHERE rang > 1
)
UPDATE game_templates t
SET config_schema = s.schema,
    updated_at = now()
FROM schemas s
WHERE t.id = s.id
  AND t.id IN (SELECT id FROM affectes);
