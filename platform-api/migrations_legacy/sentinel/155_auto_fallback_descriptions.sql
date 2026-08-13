-- Migration 155 : auto-fallback de description sur TOUTES les cles sans desc.
--
-- Au lieu d'enumerer manuellement chaque cle de chaque bot (143 dans la 154,
-- et il en manque encore...), on ajoute automatiquement une description
-- generee depuis le `label` pour toute cle qui n en a pas. Garantit 100%
-- de couverture en une passe.
--
-- Logique : si l entree a deja `description`, on ne touche pas. Sinon on
-- ajoute "description = label" comme fallback. Les descriptions manuelles
-- (152, 153, 154) sont preservees.
--
-- Pour ameliorer une description specifique plus tard, il suffit de faire
-- une migration qui appelle enrich_schema_keys avec la cle en question.

UPDATE bot_definitions
SET config_schema = (
  SELECT jsonb_agg(
    CASE
      -- Deja une description -> on garde tel quel
      WHEN entry ? 'description' AND (entry->>'description') IS NOT NULL AND length(entry->>'description') > 0
        THEN entry
      -- Pas de description : fallback = label
      ELSE entry || jsonb_build_object('description', COALESCE(entry->>'label', entry->>'key'))
    END
  )
  FROM jsonb_array_elements(config_schema) AS entry
);

-- Verification : aucune cle ne doit etre sans description apres cette migration.
DO $$
DECLARE
  v_count INTEGER;
BEGIN
  SELECT COUNT(*) INTO v_count
  FROM bot_definitions, jsonb_array_elements(config_schema) AS e
  WHERE NOT (e ? 'description');

  IF v_count > 0 THEN
    RAISE EXCEPTION 'Migration 155 : il reste % cles sans description apres la passe auto-fallback', v_count;
  END IF;

  RAISE NOTICE 'Migration 155 : 100%% des cles ont une description.';
END $$;
