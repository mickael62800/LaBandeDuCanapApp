-- Migration 156 : fusionne image-bot dans automod-bot.
--
-- Contexte : depuis la fusion des 15 anciens microbots dans sentinel-bot,
-- "image-bot" n'a plus de module dedie. L'analyse d'images est faite par
-- le module automod (vision_enabled, vision_threshold + appel ai-worker
-- pour POST /api/ai/jobs avec job_type = analyze_image).
--
-- Les cles image-bot dans bot_guild_config ne sont plus lues par personne.
-- Cette migration :
--   1. Copie les valeurs existantes vers automod-bot avec un prefixe vision_
--   2. Ajoute les nouvelles cles au schema d'automod-bot avec descriptions
--   3. Supprime image-bot (config + definition)

-- ══════════════════════════════════════════════════════════
-- 1. Copie des config_values existantes : image-bot -> automod-bot
--    Renomme les cles avec prefixe vision_ pour eviter les collisions.
--    ON CONFLICT DO NOTHING : si automod-bot a deja la cle, on garde la sienne.
-- ══════════════════════════════════════════════════════════
INSERT INTO bot_guild_config (id, guild_id, bot_name, config_key, config_value, updated_at)
SELECT
    gen_random_uuid(),
    guild_id,
    'automod-bot',
    CASE config_key
        WHEN 'confidence_threshold' THEN 'vision_threshold'  -- merge avec cle existante
        WHEN 'channel_thresholds'   THEN 'vision_channel_thresholds'
        WHEN 'hash_cache_enabled'   THEN 'vision_hash_cache_enabled'
        WHEN 'hash_cache_ttl_secs'  THEN 'vision_hash_cache_ttl_secs'
        WHEN 'max_image_size_mb'    THEN 'vision_max_image_size_mb'
        WHEN 'queue_enabled'        THEN 'vision_queue_enabled'
        WHEN 'queue_max_retries'    THEN 'vision_queue_max_retries'
        WHEN 'scan_embeds'          THEN 'vision_scan_embeds'
        WHEN 'auto_delete_nsfw'     THEN 'vision_auto_delete_nsfw'
        WHEN 'auto_delete_illicit'  THEN 'vision_auto_delete_illicit'
        ELSE config_key
    END,
    config_value,
    NOW()
FROM bot_guild_config
WHERE bot_name = 'image-bot'
  -- Skip les cles qui font doublon avec celles d'automod-bot et qui ne sont
  -- pas porteuses d info specifique vision (l admin a deja gere ca via
  -- automod-bot.enabled / log_channel_id / ignored_roles).
  AND config_key NOT IN ('enabled', 'log_channel_id', 'ignored_roles')
ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

-- ══════════════════════════════════════════════════════════
-- 2. Ajoute les nouvelles cles vision_* au schema automod-bot.
--    On utilise jsonb concat (||) pour append. Idempotent grace au filtre
--    NOT EXISTS sur chaque cle.
-- ══════════════════════════════════════════════════════════
DO $$
DECLARE
    v_new_keys JSONB;
    v_existing_keys TEXT[];
    v_to_append JSONB := '[]'::jsonb;
    v_entry JSONB;
BEGIN
    -- Cles candidates a ajouter (avec description, unit, min, max).
    v_new_keys := '[
      {"key": "vision_channel_thresholds", "label": "Seuils vision par salon",
       "type": "text", "required": false,
       "description": "Override du vision_threshold par salon. Format CSV : channel_id:threshold,channel_id:threshold."},

      {"key": "vision_hash_cache_enabled", "label": "Cache hash images",
       "type": "boolean", "required": false, "default": "true",
       "description": "Active le cache des hash d images analysees pour eviter de relancer l IA sur la meme image."},

      {"key": "vision_hash_cache_ttl_secs", "label": "TTL cache hash images",
       "type": "number", "required": false, "default": "86400",
       "unit": "secondes", "min": 60, "max": 2592000,
       "description": "Duree de validite d un hash en cache. Recommande : 86400 (1 jour)."},

      {"key": "vision_max_image_size_mb", "label": "Taille max images",
       "type": "number", "required": false, "default": "10",
       "unit": "Mo", "min": 1, "max": 25,
       "description": "Taille max d une image analysee. Au-dela, skip. Discord upload limite a 25Mo."},

      {"key": "vision_queue_enabled", "label": "File async (ai-worker)",
       "type": "boolean", "required": false, "default": "true",
       "description": "Si ON, l analyse d image est asynchrone via ai-worker (POST /api/ai/jobs). Sinon, synchrone bloquant."},

      {"key": "vision_queue_max_retries", "label": "Tentatives max queue",
       "type": "number", "required": false, "default": "3",
       "unit": "tentatives", "min": 0, "max": 10,
       "description": "Nombre max de retries sur un job IA en echec avant abandon."},

      {"key": "vision_scan_embeds", "label": "Analyser images dans embeds",
       "type": "boolean", "required": false, "default": "true",
       "description": "Analyse aussi les images presentes dans les embeds (liens preview), pas seulement les pieces jointes."},

      {"key": "vision_auto_delete_nsfw", "label": "Suppression auto NSFW",
       "type": "boolean", "required": false, "default": "false",
       "description": "Supprime automatiquement les images detectees NSFW au-dessus du seuil. Si OFF, le scoring decide via les seuils warn/delete/mute/ban."},

      {"key": "vision_auto_delete_illicit", "label": "Suppression auto illicite",
       "type": "boolean", "required": false, "default": "true",
       "description": "Supprime automatiquement les images detectees comme contenu illicite. Recommande ON (poids defaut 9.0 deja eleve)."}
    ]'::jsonb;

    -- Recupere les cles deja presentes dans automod-bot.
    SELECT array_agg(e->>'key') INTO v_existing_keys
    FROM bot_definitions, jsonb_array_elements(config_schema) e
    WHERE bot_name = 'automod-bot';

    -- Filtre les nouvelles cles : on n ajoute que celles qui n existent pas.
    FOR v_entry IN SELECT * FROM jsonb_array_elements(v_new_keys)
    LOOP
        IF NOT (v_entry->>'key' = ANY(v_existing_keys)) THEN
            v_to_append := v_to_append || jsonb_build_array(v_entry);
        END IF;
    END LOOP;

    -- Append en bloc.
    IF jsonb_array_length(v_to_append) > 0 THEN
        UPDATE bot_definitions
        SET config_schema = config_schema || v_to_append
        WHERE bot_name = 'automod-bot';
        RAISE NOTICE 'Migration 156 : % nouvelles cles vision_* ajoutees a automod-bot',
                     jsonb_array_length(v_to_append);
    ELSE
        RAISE NOTICE 'Migration 156 : toutes les cles vision_* existent deja dans automod-bot';
    END IF;
END $$;

-- ══════════════════════════════════════════════════════════
-- 3. Suppression d image-bot (config_values + definition).
-- ══════════════════════════════════════════════════════════
DELETE FROM bot_guild_config WHERE bot_name = 'image-bot';
DELETE FROM bot_definitions WHERE bot_name = 'image-bot';
