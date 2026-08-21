-- 061_compteurs_salon_vocal_et_en_vocal.sql
--
-- Deux choses.
--
-- 1. Les compteurs demandaient un salon TEXTE. Ils veulent un salon VOCAL.
-- 2. Un quatrieme compteur : les membres qui jouent ET sont en vocal.
--
--
-- 1. LE BON SELECTEUR
--
-- Les trois compteurs de la migration 058 declaraient `"type": "channel"`,
-- ce qui affiche dans le tableau de bord la liste des salons TEXTE : le salon
-- vocal cree pour accueillir le compteur n'y figurait pas, et le reglage
-- restait donc impossible a remplir.
--
-- Ce n'est pas qu'une affaire de liste. Discord n'autorise ni espace, ni
-- majuscule, ni deux-points dans le nom d'un salon textuel : « 🎮 En jeu : 7 »
-- y deviendrait « 🎮-en-jeu-7 ». Seul un salon vocal garde le nom tel qu'il
-- est ecrit — c'est la raison pour laquelle les compteurs de membres et de
-- connectes en vocal du module Accueil visent eux aussi un salon vocal.
--
-- `"type": "voice"` est le type deja compris par le formulaire
-- (`ConfigFieldRow`), qui bascule alors sur le selecteur de salons vocaux.
--
--
-- 2. EN JEU **ET** EN VOCAL
--
-- Les autres compteurs disent combien de personnes jouent, ou combien de
-- serveurs tournent. Celui-ci dit combien jouent ENSEMBLE : ceux qui ont une
-- partie en cours et sont dans un salon vocal du serveur au meme moment.
--
-- C'est le seul des quatre qui mesure la vie de la communaute plutot que
-- l'occupation des machines : deux personnes qui jouent chacune dans leur coin
-- comptent pour deux dans « En partie », et pour zero ici.

-- ─────────────────────────────────────────────────────────────────────
-- 1. Les trois compteurs existants passent au selecteur vocal
-- ─────────────────────────────────────────────────────────────────────
--
-- On ne touche QUE le type, et uniquement pour ces trois cles : le reste du
-- schema (libelles, descriptions, avertissements, ordre) est reconduit tel
-- quel. Un salon deja choisi reste choisi — c'est un identifiant, pas un type.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN elem ->> 'key' IN (
                'players_counter_channel_id',
                'servers_counter_channel_id',
                'activity_counter_channel_id'
            )
            THEN elem || '{"type": "voice"}'::jsonb
            ELSE elem
        END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE bot_name = 'game-portal'
  AND jsonb_path_exists(
      config_schema,
      '$[*] ? (@.key == "players_counter_channel_id" && @.type == "channel")'
  );


-- ─────────────────────────────────────────────────────────────────────
-- 2. Le compteur « en jeu et en vocal »
-- ─────────────────────────────────────────────────────────────────────

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "ingame_voice_counter_channel_id", "type": "voice",
   "label": "Salon compteur : en jeu ET en vocal",
   "required": false,
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "Salon vocal dont le nom affiche le nombre de membres qui jouent tout en etant connectes en vocal ici. Vide : compteur eteint.",
   "warning": "Necessite le droit de lire les presences (Presence Intent cote portail Discord, puis NEXUS_PRESENCE_INTENT=true), comme le compteur « en partie »."},

  {"key": "ingame_voice_counter_format", "type": "text",
   "label": "Format du compteur en jeu et en vocal",
   "default": "🎧 Ensemble : {count}", "max_length": 90, "required": false,
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "{count} est remplace par le nombre de membres qui jouent et parlent en meme temps."}
]'::jsonb
WHERE bot_name = 'game-portal'
  AND NOT jsonb_path_exists(config_schema, '$[*] ? (@.key == "ingame_voice_counter_channel_id")');
