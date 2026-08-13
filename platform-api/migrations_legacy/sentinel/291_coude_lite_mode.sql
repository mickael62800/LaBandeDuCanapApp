-- Mode "Lite" (fun & simple) pour Coup de Coude : quand active, le bot ne
-- publie que le coeur du jeu + les commandes rigolotes et masque tout le
-- meta-jeu lourd (braquage/prison, anti-vol, guerre sociale, prestige/
-- ultimate, paris, saisons...). Reversible : repasser a false restaure le
-- jeu complet. Le filtrage des commandes est fait cote bot (command_registry).
--
-- Idempotent : on n'ajoute le champ que s'il n'est pas deja present.
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "lite_mode", "label": "Mode simplifie (Lite) — masque le meta-jeu complexe", "type": "boolean", "required": false, "default": "false", "description": "Active une version fun & simple : ne garde que combat, classes, niveaux, boutique, vol light et commandes rigolotes. Masque braquage/prison, assurances, vendetta/coalition, prestige/ultimate, paris, saisons. Reversible."}
]'::jsonb
WHERE bot_name = 'coude-bot'
  AND NOT (config_schema @> '[{"key": "lite_mode"}]'::jsonb);
