-- Passe `default_role_id` (single) a `default_role_ids` (multi, CSV) dans le
-- config_schema de progression-bot. Migre les valeurs existantes dans
-- bot_guild_config : l'ID unique devient une liste a un element (CSV sans
-- virgule), et la cle est renommee.

-- 1. Remplace l'entree single role par la version multi dans le schema.
UPDATE bot_definitions
SET config_schema = (
  SELECT jsonb_agg(
    CASE
      WHEN (elem->>'key') = 'default_role_id' THEN
        jsonb_build_object(
          'key', 'default_role_ids',
          'label', 'Roles par defaut (nouvel arrivant)',
          'type', 'role',
          'required', false,
          'default', '',
          'description', 'IDs des roles attribues automatiquement a chaque nouveau membre. Separez plusieurs IDs par une virgule. Laissez vide pour desactiver.'
        )
      ELSE elem
    END
  )
  FROM jsonb_array_elements(config_schema) elem
)
WHERE bot_name = 'progression-bot'
  AND config_schema @> '[{"key": "default_role_id"}]'::jsonb;

-- 2. Renomme la cle dans les valeurs deja configurees par les admins.
UPDATE bot_guild_config
SET config_key = 'default_role_ids'
WHERE bot_name = 'progression-bot'
  AND config_key = 'default_role_id';
