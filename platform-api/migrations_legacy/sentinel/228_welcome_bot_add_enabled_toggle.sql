-- welcome-bot — ajoute le toggle top-level `enabled` (manquant) et la
-- cascade depends_on sur les sous-features.
--
-- Avant : 33 cles dans le schema, mais aucune cle `enabled` master.
-- Apres : prepend `enabled` au schema + cascade depends_on enabled=true
-- sur toutes les sous-cles qui n'ont pas deja un depends_on. Le code
-- (handler) checkera is_module_enabled en plus.

UPDATE bot_definitions SET
    config_schema = '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Master switch : si OFF, aucun message welcome / leave / rules / counter / anniversaire ne sera envoye."}
    ]'::jsonb || (
        SELECT jsonb_agg(
            CASE
                WHEN entry ? 'depends_on' THEN entry
                WHEN entry->>'key' = 'enabled' THEN entry
                ELSE jsonb_set(entry, '{depends_on}', '{"key":"enabled","equals":"true"}'::jsonb)
            END
        )
          FROM jsonb_array_elements(config_schema) AS entry
         WHERE entry->>'key' != 'enabled'
    )
WHERE bot_name = 'welcome-bot';
