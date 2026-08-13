-- 006_jurons_distincts_des_insultes.sql
--
-- Les jurons d'exclamation deviennent un flag distinct des insultes ciblees.
--
-- Avant, « merde j'ai oublie » et « nique ta mere » levaient le MEME flag, au
-- meme poids (5.0), au-dessus du seuil de suppression (4.0) : le premier se
-- faisait supprimer comme le second. Aucun reglage de poids ne pouvait les
-- separer puisqu'ils partageaient le flag — d'ou un flag propre.
--
-- Poids par defaut 1.0, volontairement SOUS le seuil d'avertissement (2.0) :
-- un juron seul ne declenche rien. Combine a un autre signal, il pese quand
-- meme dans la balance.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "score_weight_profanity", "type": "number",
   "label": "Scoring — poids juron d''exclamation",
   "default": "1", "min": 0, "max": 10, "required": false,
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "Poids des jurons sans cible (putain, merde, bordel, zut). Distinct des insultes ciblees, qui gardent leur propre poids. A 1 avec un seuil d''avertissement a 2, un juron seul ne declenche rien."}
]'::jsonb
WHERE bot_name = 'automod-bot'
  AND NOT config_schema @> '[{"key": "score_weight_profanity"}]'::jsonb;

-- Precision de la description du poids « insulte », qui ne couvre plus les
-- jurons. La laisser telle quelle ferait croire qu'elle les inclut encore.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'key' = 'score_weight_insult'
             THEN elem || jsonb_build_object('description',
                  'Poids des insultes CIBLEES (connard, fdp, ta gueule…). Les jurons d''''exclamation ont leur propre poids, plus bas.')
             ELSE elem END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE bot_name = 'automod-bot'
  AND config_schema @> '[{"key": "score_weight_insult"}]'::jsonb;
