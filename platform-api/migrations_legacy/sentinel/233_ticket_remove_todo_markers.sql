-- ticket-bot : retire les markers TODO de transcript_format et
-- sla_first_response_minutes (maintenant cables).

UPDATE bot_definitions
   SET config_schema = (
       SELECT jsonb_agg(
           CASE
               WHEN entry->>'key' = 'transcript_format' THEN
                   jsonb_set(entry, '{description}',
                       '"text = plain (mail/sms), markdown = format Discord avec **bold** et > quote (default), html = document HTML envoye en attachment .html."'::jsonb)
               WHEN entry->>'key' = 'sla_first_response_minutes' THEN
                   jsonb_set(entry, '{description}',
                       '"Apres N min sans premiere reponse, le bot poste un rappel dans le ticket (avant escalation). 0 = desactive."'::jsonb)
               ELSE entry
           END
       )
         FROM jsonb_array_elements(config_schema) AS entry
   )
 WHERE bot_name = 'ticket-bot';
