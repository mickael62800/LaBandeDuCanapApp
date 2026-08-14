-- 035_clarify_levelup_announce_toggle.sql
--
-- Clarifie, dans le schema de config du module `progression-bot`, que le seul
-- interrupteur qui coupe les messages de level-up (texte ET vocal) est
-- `levelup_announce_enabled`. Vider le salon d'annonce ne desactive PAS le
-- message : en texte, le bot retombe sur le salon courant (voir
-- sentinel-bot/src/modules/progression/mod.rs, fonction `announce_level_up`).
--
-- On ne touche qu'aux `description` de deux cles, sans changer les valeurs ni
-- la structure du schema. Idempotent : on reecrit chaque description a sa valeur
-- cible quelle que soit l'ancienne.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN elem ->> 'key' = 'levelup_announce_enabled' THEN
                elem || jsonb_build_object(
                    'description',
                    'INTERRUPTEUR PRINCIPAL des messages de level-up (texte ET vocal). '
                    || 'Si OFF, aucun message n''est poste, nulle part. '
                    || 'Note : vider le salon ci-dessus ne desactive PAS l''annonce '
                    || '(en texte, elle retombe sur le salon courant) — pour tout couper, mets CET interrupteur sur OFF.'
                )
            WHEN elem ->> 'key' = 'level_up_channel_id' THEN
                elem || jsonb_build_object(
                    'description',
                    'Salon ou poster l''annonce de level-up. Si vide, l''annonce est postee dans le salon courant. '
                    || 'Vider ce champ NE desactive PAS l''annonce : pour la couper, utilise l''interrupteur '
                    || '« Annonce level-up dans le salon ».'
                )
            ELSE elem
        END
    )
    FROM jsonb_array_elements(config_schema) AS elem
)
WHERE bot_name = 'progression-bot'
  AND config_schema @> '[{"key": "levelup_announce_enabled"}]'::jsonb;
