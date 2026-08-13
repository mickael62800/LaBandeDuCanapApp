-- progression-bot : retire les markers "TODO" des cles maintenant cablees.
--
-- max_level / levelup_message / levelup_dm_enabled etaient marques TODO
-- dans la mig 226. Ces 3 cles sont maintenant cablees via le helper
-- announce_level_up. Mise a jour des descriptions UI.

UPDATE bot_definitions
   SET config_schema = (
       SELECT jsonb_agg(
           CASE
               WHEN entry->>'key' = 'max_level' THEN
                   jsonb_set(entry, '{description}',
                       '"Niveau max au-dela duquel les annonces level-up sont supprimees. 0 = illimite. Les role rewards continuent par contre."'::jsonb)
               WHEN entry->>'key' = 'levelup_message' THEN
                   jsonb_set(entry, '{description}',
                       '"Template du message level-up. Variables : {user}, {level}, {kind}. Si vide, message par defaut. S applique a la fois a l annonce salon et au DM."'::jsonb)
               WHEN entry->>'key' = 'levelup_dm_enabled' THEN
                   jsonb_set(entry, '{description}',
                       '"Si ON, envoie aussi un DM au membre lors du level-up (en plus de l annonce dans le salon)."'::jsonb)
               ELSE entry
           END
       )
         FROM jsonb_array_elements(config_schema) AS entry
   )
 WHERE bot_name = 'progression-bot';
