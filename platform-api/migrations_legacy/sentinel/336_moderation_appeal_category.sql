-- Moderation-bot — appel de sanction par SALON dedie sous une CATEGORIE.
--
-- Au clic sur « Contester cette sanction » (ou /appeal), le bot cree un salon
-- prive sous cette categorie, visible seulement par l'appelant et le role modo.
-- Si vide, on retombe sur une simple notification dans appeal_channel_id.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' <> 'appeal_category_id'
        UNION ALL SELECT '{
            "key": "appeal_category_id",
            "label": "Catégorie des salons d appel",
            "type": "category",
            "required": false,
            "default": "",
            "description": "Catégorie sous laquelle un salon privé est créé automatiquement quand un membre conteste sa sanction. Vide = simple notification dans le salon d appels."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'moderation-bot';
