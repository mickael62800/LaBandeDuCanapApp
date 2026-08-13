-- Met a jour le label de voice_anchor_category_id pour refleter le nouveau
-- comportement : les salons temporaires sont desormais crees DANS la
-- categorie selectionnee (en bas) au lieu d etre juste positionnes en dessous.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN elem->>'key' = 'voice_anchor_category_id'
                THEN elem
                    || '{"label": "Categorie des salons temporaires"}'::jsonb
                    || '{"description": "Les salons vocaux temporaires seront crees dans cette categorie (en bas). Vide = racine du serveur."}'::jsonb
            ELSE elem
        END
    )
    FROM jsonb_array_elements(config_schema) AS elem
)
WHERE bot_name = 'voice-bot';
