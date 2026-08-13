-- Retire le champ `lite_mode` du schema coude-bot : le meta-jeu lourd a ete
-- retire DIRECTEMENT des commandes (le jeu est desormais "fun & simple" en
-- permanence), donc le toggle Lite n'a plus de raison d'etre. Annule la
-- migration 291 cote schema de config.
UPDATE bot_definitions
SET config_schema = COALESCE(
    (SELECT jsonb_agg(e) FROM jsonb_array_elements(config_schema) e WHERE e->>'key' <> 'lite_mode'),
    '[]'::jsonb
)
WHERE bot_name = 'coude-bot'
  AND config_schema @> '[{"key": "lite_mode"}]'::jsonb;
