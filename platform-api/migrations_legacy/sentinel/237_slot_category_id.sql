-- slot-bot : ajout `slot_category_id` pour regrouper les salons slot
-- temporaires sous une categorie Discord (UX : eviter le bordel a la
-- racine quand plusieurs users jouent en meme temps).

UPDATE bot_definitions SET
    config_schema = config_schema || '[
        {"key": "slot_category_id", "label": "Categorie pour les salons slot", "type": "category", "required": false, "description": "Categorie Discord ou les salons slot temporaires (slot-{user}) sont crees. Si vide, places a la racine.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
WHERE bot_name = 'slot-bot'
  AND NOT (config_schema @> '[{"key": "slot_category_id"}]'::jsonb);
