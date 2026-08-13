-- Automod — bouton "Ouvrir une discussion" sur les cartes de vote.
--
-- Un modo peut, depuis une carte, creer un salon textuel prive (membre
-- concerne + role modo) avec un message de contexte epingle, pour discuter
-- avant de trancher. Deux cles de config (page web automod) :
--   discussion_channel_enabled : affiche le bouton sur les cartes
--   discussion_category_id     : categorie ou creer le salon (optionnel)

-- Idempotent : on retire d'abord les cles si presentes, puis on les (re)ajoute.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM (
        SELECT elem FROM jsonb_array_elements(config_schema) AS elem
            WHERE elem->>'key' NOT IN ('discussion_channel_enabled', 'discussion_category_id')
        UNION ALL SELECT '{
            "key": "discussion_channel_enabled",
            "label": "Bouton ''Ouvrir une discussion'' sur les cartes",
            "type": "boolean",
            "required": false,
            "default": "false",
            "description": "Si ON, chaque carte de vote affiche un bouton qui cree un salon textuel prive (membre concerne + role moderateur) avec un message de contexte epingle, pour discuter avant decision."
        }'::jsonb
        UNION ALL SELECT '{
            "key": "discussion_category_id",
            "label": "Categorie des salons de discussion",
            "type": "channel",
            "required": false,
            "description": "Categorie sous laquelle creer les salons de discussion. Vide = a la racine du serveur."
        }'::jsonb
    ) sub
)
WHERE bot_name = 'automod-bot';
