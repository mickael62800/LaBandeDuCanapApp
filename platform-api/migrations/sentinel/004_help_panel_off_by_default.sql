-- 004_help_panel_off_by_default.sql
--
-- Le panneau d'aide ne se deploie plus tout seul.
--
-- Il valait `enabled = true` par defaut : le bot creait donc ses salons et y
-- publiait le catalogue des commandes des sa premiere connexion, sans que
-- personne ne l'ait demande. Un module qui fabrique des salons doit etre
-- choisi, pas subi.
--
-- Le reste du fonctionnement ne change pas : on designe une categorie, le
-- bot y cree son salon. C'est le comportement voulu.

-- Le defaut vit dans le schema de configuration du bot, en JSON.
UPDATE bot_definitions
SET config_schema = jsonb_set(
    config_schema,
    -- Chemin vers le champ `default` de l'entree dont la cle est `enabled`.
    ARRAY[
        (
            SELECT (idx - 1)::text
            FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, idx)
            WHERE elem ->> 'key' = 'enabled'
            LIMIT 1
        ),
        'default'
    ],
    '"false"'::jsonb
)
WHERE bot_name = 'help-bot'
  AND config_schema @> '[{"key": "enabled"}]'::jsonb;

-- Les guildes qui n'ont jamais touche au reglage suivaient le defaut : elles
-- basculent donc naturellement. Celles qui l'ont active explicitement ont une
-- ligne en base et gardent leur choix — on ne desactive pas sous leurs pieds
-- un panneau qu'elles utilisent.
