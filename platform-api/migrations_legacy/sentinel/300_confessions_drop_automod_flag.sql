-- C1 — Retire le flag mort `automod_enabled` du schema de config du module
-- confessions.
--
-- Contexte : le schema (migration 185) annoncait une entree "Filtre AutoMod
-- actif (refuse contenu toxique)", mais AUCUN filtre de mots n'a jamais ete
-- cable cote API/bot. Le reglage etait donc trompeur (l'utilisateur croyait
-- proteger son salon alors que rien ne filtrait).
--
-- On retire chirurgicalement l'objet `automod_enabled` du tableau JSONB
-- `bot_definitions.config_schema` (mirroir des migrations 298 / 226 qui
-- editent le JSONB par REPLACE sur le texte). La colonne DB applicative
-- `confession_configs.automod_enabled` (si presente) est CONSERVEE pour
-- back-compat : on cesse simplement de l'exposer.
--
-- Idempotent : no-op si l'entree est deja absente.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem ->> 'key' <> 'automod_enabled'
)
WHERE bot_name = 'confessions'
  AND config_schema @> '[{"key": "automod_enabled"}]'::jsonb;
