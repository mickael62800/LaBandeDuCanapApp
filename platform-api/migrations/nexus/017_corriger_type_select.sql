-- 017_corriger_type_select.sql
--
-- Repare les reglages ecrits avec `"type": "select"`.
--
-- Le type attendu est `enum` : c'est ce que reconnait `ConfigFieldType` cote
-- Rust. J'ai ecrit `select` dans les migrations 012 et 013, par analogie avec
-- le HTML.
--
-- Consequence : la desserialisation du schema echouait, et TOUT l'endpoint
-- des modeles de jeu repondait 500 — pas seulement les cinq champs fautifs.
-- Un seul champ invalide empeche de lire le tableau entier.
--
-- 012 et 013 gardent volontairement leur `select` fautif : sqlx enregistre
-- une EMPREINTE de chaque migration appliquee et refuse de demarrer si elle
-- change. Les corriger apres coup a immobilise nexus-api sur
-- « migration 12 was previously applied but has been modified ».
--
-- Une migration appliquee ne se modifie jamais. On la repare par une
-- suivante. Une installation neuve passera donc par l'erreur puis par sa
-- correction — sans consequence, 017 s'executant dans la foulee.
--
-- Idempotente : sans occurrence de `select`, elle ne modifie rien.

UPDATE game_templates
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'type' = 'select'
             THEN elem || '{"type": "enum"}'::jsonb
             ELSE elem END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE config_schema @> '[{"type": "select"}]'::jsonb;

-- Meme correction sur les definitions de bots : la migration 014 n'utilisait
-- que des types valides, mais le controle coute moins cher que la certitude.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'type' = 'select'
             THEN elem || '{"type": "enum"}'::jsonb
             ELSE elem END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE config_schema @> '[{"type": "select"}]'::jsonb;
