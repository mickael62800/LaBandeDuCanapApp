-- Fusion de la config IA dans le schema automod-bot.
--
-- Avant cette migration, les cles IA etaient reparties entre deux sources :
--   * table dediee `ia_config` (text_enabled, vision_enabled, text_threshold,
--     vision_threshold, context_dampening, context_format, context_max_messages,
--     context_max_chars) avec sa page web /ia-config.
--   * `bot_guild_config` (bot_name=automod-bot) qui contenait deja
--     text_enabled / vision_enabled (migration 131), ainsi que
--     context_max_messages / context_max_chars (migration 086).
--
-- Cette migration :
--   1. Ajoute au config_schema de `automod-bot` les cles manquantes :
--      text_threshold, vision_threshold, context_dampening, context_format
--      (text_enabled, vision_enabled sont deja present via migration 131 ;
--       context_max_messages / context_max_chars via migration 086).
--   2. Migre les valeurs existantes de `ia_config` vers `bot_guild_config`
--      (bot_name=automod-bot) sans ecraser les valeurs deja presentes
--      (ON CONFLICT DO NOTHING).
--
-- La table `ia_config` n'est PAS supprimee (rétrocompat / backup).
-- L'API ne la lit plus apres cette migration.

-- ══════════════════════════════════════════════════════════
-- 1. Ajout des cles manquantes dans le config_schema automod-bot
-- ══════════════════════════════════════════════════════════

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "text_threshold", "label": "Seuil confidence IA texte", "type": "number", "required": false, "default": "0.5", "description": "Seuil de confidence IA texte (0.0-1.0) au-dela duquel une classification est retenue."},
  {"key": "vision_threshold", "label": "Seuil confidence IA vision", "type": "number", "required": false, "default": "0.5", "description": "Seuil de confidence IA vision (0.0-1.0) au-dela duquel une classification image est retenue."},
  {"key": "context_dampening", "label": "Attenuation contexte conversationnel", "type": "number", "required": false, "default": "0.65", "description": "Attenuation du score IA quand du contexte conversationnel est fourni (0.0-1.0)."},
  {"key": "context_format", "label": "Format contexte IA", "type": "text", "required": false, "default": "natural", "description": "Format du contexte envoye a l IA : natural (conversation brute) ou tagged (balises [message]/[context])."}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT (config_schema @> '[{"key": "text_threshold"}]'::jsonb);

-- ══════════════════════════════════════════════════════════
-- 2. Migration des valeurs existantes ia_config → bot_guild_config
-- ══════════════════════════════════════════════════════════
-- Pour chaque row de `ia_config`, on copie les valeurs dans bot_guild_config
-- avec bot_name=automod-bot. On n ecrase PAS une cle deja presente (l admin
-- peut avoir deja configure via /component-config).

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ia_config') THEN
    INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
    SELECT guild_id, 'automod-bot', 'text_enabled', text_enabled::text FROM ia_config
    ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

    INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
    SELECT guild_id, 'automod-bot', 'vision_enabled', vision_enabled::text FROM ia_config
    ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

    INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
    SELECT guild_id, 'automod-bot', 'text_threshold', text_threshold::text FROM ia_config
    ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

    INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
    SELECT guild_id, 'automod-bot', 'vision_threshold', vision_threshold::text FROM ia_config
    ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

    INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
    SELECT guild_id, 'automod-bot', 'context_dampening', context_dampening::text FROM ia_config
    ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

    INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
    SELECT guild_id, 'automod-bot', 'context_format', context_format FROM ia_config
    ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

    INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
    SELECT guild_id, 'automod-bot', 'context_max_messages', context_max_messages::text FROM ia_config
    ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

    INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
    SELECT guild_id, 'automod-bot', 'context_max_chars', context_max_chars::text FROM ia_config
    ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;
  END IF;
END $$;
