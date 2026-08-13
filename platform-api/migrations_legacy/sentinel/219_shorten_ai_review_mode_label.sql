-- UI : raccourcit le label "Mode review IA (insultes, spam, liens,
-- phishing)" en "Mode review IA..." pour ne pas faire deborder la
-- carte dans la grille AutoMod. Le detail reste dans la description
-- (tooltip).

UPDATE bot_definitions
   SET config_schema = (
       SELECT jsonb_agg(
           CASE
               WHEN entry->>'key' = 'ai_review_mode'
                   THEN jsonb_set(entry, '{label}', '"Mode review IA..."'::jsonb)
               ELSE entry
           END
       )
         FROM jsonb_array_elements(config_schema) AS entry
   )
 WHERE bot_name = 'automod-bot'
   AND config_schema @> '[{"key": "ai_review_mode"}]'::jsonb;
