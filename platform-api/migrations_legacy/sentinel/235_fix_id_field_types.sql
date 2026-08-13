-- Audit : detecte les cles de type "text" dont le nom suggere un ID Discord
-- (channel_id, role_id, category_id) et les passe au type approprie pour
-- afficher le bon selecteur visuel (au lieu d'une saisie ID brute).
--
-- Mapping :
--   *_channel_id   -> type "channel"  (selecteur salons)
--   *_role_id      -> type "role"     (selecteur roles)
--   *_category_id  -> type "channel"  (categorie = channel type 4 cote Discord)
--
-- Les *_id qui ne mappent pas a un selecteur Discord (guild_id, user_id,
-- message_id, panel_message_id, emoji_host_guild_id) restent en type "text".

UPDATE bot_definitions
   SET config_schema = (
       SELECT jsonb_agg(
           CASE
               -- channel_id ou category_id en text -> channel
               WHEN entry->>'type' = 'text'
                AND (entry->>'key' LIKE '%_channel_id'
                  OR entry->>'key' LIKE '%_category_id'
                  OR entry->>'key' = 'channel_id'
                  OR entry->>'key' = 'category_id')
                AND entry->>'key' NOT IN ('emoji_host_guild_id', 'panel_message_id')
                   THEN jsonb_set(entry, '{type}', '"channel"'::jsonb)
               -- role_id en text -> role
               WHEN entry->>'type' = 'text'
                AND (entry->>'key' LIKE '%_role_id' OR entry->>'key' = 'role_id')
                   THEN jsonb_set(entry, '{type}', '"role"'::jsonb)
               ELSE entry
           END
       )
         FROM jsonb_array_elements(config_schema) AS entry
   )
 WHERE config_schema IS NOT NULL
   AND EXISTS (
       SELECT 1 FROM jsonb_array_elements(config_schema) e
        WHERE e->>'type' = 'text'
          AND (e->>'key' LIKE '%_channel_id'
            OR e->>'key' LIKE '%_category_id'
            OR e->>'key' LIKE '%_role_id'
            OR e->>'key' IN ('channel_id', 'category_id', 'role_id'))
          AND e->>'key' NOT IN ('emoji_host_guild_id', 'panel_message_id')
   );
