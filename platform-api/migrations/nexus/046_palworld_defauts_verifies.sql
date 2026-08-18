-- 046_palworld_defauts_verifies.sql
--
-- Corrige trois defauts Palworld, verifies contre la documentation de l'image
-- `thijsvanloef/palworld-server-docker` (page « Game settings ») plutot que
-- de memoire.
--
--   1. PAL_EGG_DEFAULT_HATCHING_TIME : le defaut est 1 heure, pas 72.
--
--      Et surtout, cette duree est celle de l'oeuf MASSIF — le plus long du
--      jeu ; les autres eclosent proportionnellement plus vite. Le libelle
--      disait « eclosion des oeufs » tout court, ce qui laissait croire que la
--      valeur s'appliquait telle quelle a chaque oeuf. Regler « 1 heure » et
--      voir le jeu annoncer une duree differente n'avait alors aucun sens.
--
--   2. ALLOW_GLOBAL_PALBOX_IMPORT : le defaut de l'image est False, pas True.
--
--      La migration 044 l'avait pose a True. C'est precisement le reglage le
--      plus lourd de consequences (un Pal venu d'un serveur debride arrive
--      tel quel) : son defaut doit etre celui de l'image, pas l'inverse.
--
--   3. DEATH_PENALTY : le defaut de l'image est « Item », pas « All ».
--
--      « All » est le defaut du JEU, mais c'est l'image qui ecrit la
--      configuration : afficher « All » comme defaut faisait croire a un
--      reglage subi alors qu'il etait impose par nous.
--
-- Seules les valeurs par DEFAUT changent : un serveur ayant deja regle ces
-- champs garde sa valeur (elle vit dans `game_server_config`, pas ici).

UPDATE game_templates SET config_schema = (
    SELECT jsonb_agg(
        CASE elem ->> 'key'
            WHEN 'PAL_EGG_DEFAULT_HATCHING_TIME' THEN elem || jsonb_build_object(
                'default', 1,
                'label', 'Eclosion de l''oeuf massif (heures)',
                'description', 'Duree pour l''oeuf le plus long du jeu ; les autres eclosent proportionnellement plus vite.'
            )
            WHEN 'ALLOW_GLOBAL_PALBOX_IMPORT' THEN elem || '{"default": "false"}'::jsonb
            WHEN 'DEATH_PENALTY' THEN elem || '{"default": "Item"}'::jsonb
            ELSE elem
        END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE slug = 'palworld';
