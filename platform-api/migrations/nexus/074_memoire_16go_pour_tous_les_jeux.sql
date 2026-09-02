-- 074_memoire_16go_pour_tous_les_jeux.sql
--
-- La migration 018 avait porte le plafond memoire de chaque jeu a 16 Go.
-- Les jeux ajoutes ensuite (054, 055, 070) ont ete inseres avec leurs
-- propres valeurs, dont plusieurs a 8192 : le curseur de creation, qui lit
-- `max_memory_mb` du template, s'arretait donc a 8 Go sur ces jeux.
--
-- On repasse tout le catalogue a 16 Go, et on refait le meme geste sur le
-- quota de guilde pour que le plafond individuel serve a quelque chose.
--
-- Rappel : le CONTENEUR recoit un quart de plus que la valeur allouee
-- (cf. `container_memory_mb`). Un serveur a 16 Go occupe 20 Go machine.

UPDATE game_templates SET max_memory_mb = 16384 WHERE max_memory_mb < 16384;


-- Defaut du quota cumule par guilde : 8 Go interdisait a lui seul un unique
-- serveur a 16 Go. Aligne sur la valeur posee en 018.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'key' = 'max_memory_total_mb'
             THEN elem || '{"default": "32768"}'::jsonb
             ELSE elem END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE bot_name = 'game-portal'
  AND config_schema @> '[{"key": "max_memory_total_mb"}]'::jsonb;

-- Les guildes configurees avant 018 ont la valeur 8192 ecrite en base : le
-- defaut du schema ne les concerne plus. On ne remonte QUE cette valeur-la,
-- qui est l'ancien defaut recopie, jamais un plafond choisi volontairement.
UPDATE bot_guild_config
SET config_value = '32768'
WHERE bot_name = 'game-portal'
  AND config_key = 'max_memory_total_mb'
  AND config_value = '8192';
