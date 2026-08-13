-- Modules orphelins : 4 modules existent dans le bot mais n'avaient pas
-- d'entree bot_definitions, donc invisibles dans la page Composants.
--
-- - confessions : confessions anonymes (panel + admin)
-- - community-bot : parrainage + role prerequisites
-- - slot-bot : machine a sous
-- - wheel-bot : roue de la fortune
--
-- Pour chacun : entree avec `enabled` + cles consommees par le code.

-- ── confessions ──────────────────────────────────────────────────
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'confessions',
    'Confessions anonymes',
    'Permet aux membres de poster des confessions anonymes via un panel. La configuration (salon de publication, panel_message_id) est automatiquement persistee par /confess-admin deploy-panel.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active les commandes /confess + /confess-admin. Le salon de publication est defini via /confess-admin deploy-panel."}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

-- ── community-bot ────────────────────────────────────────────────
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'community-bot',
    'Communaute (parrainage + roles)',
    'Systeme de parrainage entre membres + verification de roles prerequis (ex: avoir le role Verifie pour poster). Les groupes exclusifs empechent la coexistence de roles incompatibles.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true"},
        {"key": "max_sponsorships", "label": "Max parrainages par membre", "type": "number", "required": false, "default": "5", "min": 1, "max": 100, "description": "Combien de filleuls un meme parrain peut avoir simultanement.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "sponsor_min_parrain_days", "label": "Anciennete min parrain", "type": "number", "required": false, "default": "30", "min": 0, "max": 365, "unit": "j", "description": "Le parrain doit avoir au moins N jours d anciennete sur le serveur pour parrainer.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "sponsor_max_filleul_days", "label": "Anciennete max filleul", "type": "number", "required": false, "default": "7", "min": 0, "max": 365, "unit": "j", "description": "Le filleul doit avoir moins de N jours sur le serveur (recent join).", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "exclusive_groups", "label": "Groupes exclusifs", "type": "text", "required": false, "default": "", "description": "Groupes de roles incompatibles. Format CSV multilignes : group_name|role_id,role_id (un par ligne). Si un membre a deja un role d un groupe et reussit en obtient un autre du meme groupe, l ancien est retire.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "role_prerequisites", "label": "Prerequis de roles", "type": "text", "required": false, "default": "", "description": "Format CSV : target_role_id:requires_role_id (un par ligne). Pour obtenir target, il faut deja avoir requires.", "depends_on": {"key": "enabled", "equals": "true"}},
        {"key": "temp_roles", "label": "Roles temporaires (assign manuel)", "type": "text", "required": false, "default": "", "description": "Format CSV : role_id:duree_secs (un par ligne). Roles attribues via /role temp qui seront retires apres N secondes par le worker temp_roles.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

-- ── slot-bot ─────────────────────────────────────────────────────
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'slot-bot',
    'Machine a sous',
    'Mini-jeu machine a sous (3 rouleaux) en utilisant les coins du module Coude. Limite de mise par defaut configurable.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active la commande /slot."},
        {"key": "default_bet", "label": "Mise par defaut", "type": "number", "required": false, "default": "10", "min": 1, "max": 1000000, "unit": "coins", "description": "Mise utilisee si /slot est appele sans argument.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

-- ── wheel-bot ────────────────────────────────────────────────────
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'wheel-bot',
    'Roue de la fortune',
    'Mini-jeu roue de la fortune en utilisant les coins du module Coude. Multiplicateurs aleatoires (perte totale a x5).',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "true", "description": "Active la commande /wheel."}
    ]'::jsonb
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
