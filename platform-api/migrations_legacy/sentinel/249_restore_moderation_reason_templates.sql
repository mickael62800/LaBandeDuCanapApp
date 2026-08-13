-- moderation-bot — restauration de reason_templates dans le schema.
--
-- Regression : la cle reason_templates (ajoutee mig 049) a disparu quand la
-- mig 212 (fusion worker moderation -> module) a reecrit entierement le
-- config_schema. Le code la LIT toujours (mod.rs:186 pour l'autocomplete des
-- raisons, et la commande /template la lit/ecrit via CONFIG_KEY).
--
-- Impact limite : /template gere deja cette valeur directement en DB ; la
-- restaurer au schema permet juste de la voir/editer aussi depuis la page
-- Composants. Type text (format : label|raison, une par ligne).

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "reason_templates", "label": "Templates de raisons", "type": "text", "required": false, "default": "", "description": "Raisons de sanction predefinies (autocomplete). Format : label|raison, une par ligne. Gere aussi via /template.", "depends_on": {"key": "enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'moderation-bot'
  AND NOT (config_schema @> '[{"key": "reason_templates"}]'::jsonb);
