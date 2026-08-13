-- UI : pour chaque bot_definitions ayant une cle 'enabled', ajoute
-- automatiquement `depends_on: {key: "enabled", equals: "true"}` sur
-- toutes les autres cles qui n'ont pas deja un depends_on.
--
-- Sans ca, les utilisateurs voyaient certains inputs rester
-- editables alors que le module etait OFF (cascade UI incomplete).

UPDATE bot_definitions
   SET config_schema = (
       SELECT jsonb_agg(
           CASE
               -- cle 'enabled' elle-meme : on la touche pas
               WHEN entry->>'key' = 'enabled' THEN entry
               -- depends_on deja present (autre dependance) : on touche pas
               WHEN entry ? 'depends_on' THEN entry
               -- sinon : ajout du depends_on enabled
               ELSE jsonb_set(entry, '{depends_on}',
                   '{"key":"enabled","equals":"true"}'::jsonb)
           END
       )
         FROM jsonb_array_elements(config_schema) AS entry
   )
 WHERE config_schema @> '[{"key": "enabled"}]'::jsonb;
