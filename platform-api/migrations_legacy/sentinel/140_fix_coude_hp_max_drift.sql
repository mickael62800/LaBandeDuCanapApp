-- Migration 140 — Correctif drift hp_max sur coude_players.
--
-- Bug : jusqu'au fix combat.rs + progression.rs du 20 avril 2026, les
-- appels `/train stat:defense` et les level-ups ne mettaient pas a jour
-- `hp_max`. Resultat : la valeur DB diverge de la formule canonique
-- `100 + effective_def * 2` appliquee par le moteur de combat. Visible
-- pour le joueur par des HP capes silencieusement apres un `/repos`
-- (symptome « /repos pas pris en compte »).
--
-- Cette migration recompute `hp_max` pour tous les joueurs existants a
-- partir de leur classe + niveau + points de stat investis, puis clamp
-- `hp_current` au nouveau plafond. Les joueurs sans classe (nouveaux /
-- legacy) conservent `hp_max = 100`.
--
-- Valeurs de `base_def` et `def_growth` hardcodees car elles proviennent
-- du domain `coude_combat_engine::classes.rs` :
--   Bourrin : base_def = 8,  def_growth = 1
--   Agile   : base_def = 18, def_growth = 3
--   Fourbe  : base_def = 14, def_growth = 2
--   Tank    : base_def = 25, def_growth = 4
--
-- Idempotence : replayable, l'UPDATE pose directement les valeurs
-- cibles — relancer la migration n'a pas d'effet de bord.

UPDATE coude_players
SET
    hp_max = 100 + (
        (CASE class
            WHEN 'bourrin' THEN 8  + (level - 1) * 1
            WHEN 'agile'   THEN 18 + (level - 1) * 3
            WHEN 'fourbe'  THEN 14 + (level - 1) * 2
            WHEN 'tank'    THEN 25 + (level - 1) * 4
            ELSE 0
        END) + def
    ) * 2,
    hp_current = LEAST(
        hp_current,
        100 + (
            (CASE class
                WHEN 'bourrin' THEN 8  + (level - 1) * 1
                WHEN 'agile'   THEN 18 + (level - 1) * 3
                WHEN 'fourbe'  THEN 14 + (level - 1) * 2
                WHEN 'tank'    THEN 25 + (level - 1) * 4
                ELSE 0
            END) + def
        ) * 2
    ),
    updated_at = NOW()
WHERE class IS NOT NULL;

-- Pour les joueurs sans classe (jamais joue), on force hp_max = 100 +
-- 2 * def pour rester coherent avec le fallback `bourrin` du moteur.
UPDATE coude_players
SET
    hp_max = 100 + def * 2,
    hp_current = LEAST(hp_current, 100 + def * 2),
    updated_at = NOW()
WHERE class IS NULL;
