-- UI : pour les cles qui referencent une CATEGORIE Discord (type 4),
-- on passe au nouveau type "category" pour afficher le selecteur dedie
-- (filtre sur kind=category, evite de melanger avec les salons texte/voice).
--
-- Cles cibles :
--   - blackjack-bot.category_blackjack
--   - ticket-bot.ticket_category_id
--   - audit-bot.archive_category_id (s'il existe)
--   - voice-bot.voice_anchor_category_id (mig 234)
--   - tout *_category_id en general

UPDATE bot_definitions
   SET config_schema = (
       SELECT jsonb_agg(
           CASE
               WHEN entry->>'key' LIKE '%category%'
                AND entry->>'type' IN ('channel', 'text')
                   THEN jsonb_set(entry, '{type}', '"category"'::jsonb)
               WHEN entry->>'key' = 'category_blackjack'
                AND entry->>'type' IN ('channel', 'text')
                   THEN jsonb_set(entry, '{type}', '"category"'::jsonb)
               ELSE entry
           END
       )
         FROM jsonb_array_elements(config_schema) AS entry
   )
 WHERE config_schema IS NOT NULL
   AND EXISTS (
       SELECT 1 FROM jsonb_array_elements(config_schema) e
        WHERE (e->>'key' LIKE '%category%' OR e->>'key' = 'category_blackjack')
          AND e->>'type' IN ('channel', 'text')
   );
