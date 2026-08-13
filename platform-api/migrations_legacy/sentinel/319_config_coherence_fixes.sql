-- ============================================================================
-- Coherence config : de-duplication de `starting_coins` (coude-bot) +
-- alignement du plafond `max_rows_per_export` (export).
-- ============================================================================
-- BUG #3 : la cle `starting_coins` a ete seedee deux fois dans le config_schema
-- de coude-bot — une fois par la migration 061 (default "200") puis de nouveau,
-- append inconditionnel, par la migration 285 (default "100"). Le dashboard
-- affichait donc le champ EN DOUBLE. On reconstruit le tableau JSONB en filtrant
-- TOUTES les entrees `starting_coins`, puis on en re-append UNE SEULE, canonique
-- (default 100 = le grant reel du wallet `DEFAULT_STARTING_COINS`, aligne sur le
-- fallback code `guild_config.rs`). Idempotent : re-executer filtre puis
-- re-append -> toujours exactement une entree.
--
-- BUG #5 : le schema autorisait jusqu a 10 000 000 lignes pour l export alors
-- que l export-service (sentinel-core) re-clampe a 50 000. Une valeur dashboard
-- entre 50k et 10M etait silencieusement tronquee ~200x. Plafond canonique
-- retenu : 50 000 (worker clamp + export-service clamp deja a 50k). On abaisse
-- le `max` du schema pour que l UI ne promette pas ce que le code ne livre pas.
-- Idempotent : re-fixer max=50000 est un no-op.

-- BUG #3 — de-duplication de starting_coins (coude-bot), canonique default 100.
UPDATE bot_definitions
SET config_schema = (
    SELECT COALESCE(jsonb_agg(elem), '[]'::jsonb)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem->>'key' <> 'starting_coins'
) || '[
    {"key": "starting_coins", "label": "Solde de depart", "type": "number", "required": false, "default": "100", "min": 0, "max": 1000000000, "unit": "coins", "description": "Coins offerts a la creation du portefeuille d un nouveau joueur."}
]'::jsonb
WHERE bot_name = 'coude-bot';

-- BUG #5 — abaisse le plafond max_rows_per_export a 50 000 (coherent code).
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN elem->>'key' = 'max_rows_per_export'
                THEN jsonb_set(elem, '{max}', '50000'::jsonb)
            ELSE elem
        END
    )
    FROM jsonb_array_elements(config_schema) AS elem
)
WHERE bot_name = 'export'
  AND config_schema @> '[{"key": "max_rows_per_export"}]'::jsonb;
