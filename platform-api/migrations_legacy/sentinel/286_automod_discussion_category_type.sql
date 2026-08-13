-- Automod — corrige le type du champ `discussion_category_id`.
--
-- La migration 265 declarait ce champ en type "channel", ce qui affiche dans
-- le dashboard un selecteur de SALONS TEXTUELS — impossible d'y choisir une
-- categorie. Le salon de discussion etait donc cree a la racine du serveur.
-- On repasse le champ en type "category" (selecteur de categories) pour que
-- le salon soit bien ancre sous la categorie choisie.

-- Idempotent : on retire d'abord la cle si presente, puis on la (re)ajoute.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'discussion_category_id'
        UNION ALL SELECT '{
            "key": "discussion_category_id",
            "label": "Categorie des salons de discussion",
            "type": "category",
            "required": false,
            "description": "Categorie sous laquelle creer les salons de discussion. Vide = a la racine du serveur."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'automod-bot';
