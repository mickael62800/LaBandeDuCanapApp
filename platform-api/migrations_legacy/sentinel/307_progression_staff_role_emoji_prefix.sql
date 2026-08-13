-- ============================================================================
-- Progression — prefixe emoji de role staff devant le pseudo.
-- ============================================================================
-- Nouvelle fonctionnalite : le bot peut prefixer automatiquement le pseudo
-- d'un membre avec un emoji correspondant a son role staff le plus eleve
-- (ex. 👑 fondateur, 🛡️ admin, ⚔️ mod). Se combine avec le prefixe de niveau
-- existant `[NN]` -> `👑[12]Alice`.
--
-- Deux cles ajoutees au config_schema du module `progression-bot` :
--   - staff_prefix_enabled (boolean, defaut false) : active la fonctionnalite.
--   - staff_role_emojis (text CSV role_id:emoji) : la table de correspondance.
--
-- Idempotent : n'ajoute chaque cle que si elle est absente du schema.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "staff_prefix_enabled", "label": "Emoji de role staff devant le pseudo", "type": "boolean", "required": false, "default": "false", "description": "Ajoute automatiquement un emoji devant le pseudo selon le role staff le plus eleve du membre (ex. 👑 admin). Se combine avec le prefixe de niveau [NN]. Emojis unicode uniquement."}
]'::jsonb
WHERE bot_name = 'progression-bot'
  AND NOT (config_schema @> '[{"key": "staff_prefix_enabled"}]'::jsonb);

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "staff_role_emojis", "label": "Emojis par role (CSV)", "type": "text", "required": false, "description": "Format role_id:emoji, separes par des virgules. Ex: 111:👑,222:🛡️,333:⚔️. L''emoji du role le plus haut du membre est utilise. Emojis unicode uniquement (les emojis custom ne s''affichent pas dans un pseudo).", "depends_on": {"key": "staff_prefix_enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'progression-bot'
  AND NOT (config_schema @> '[{"key": "staff_role_emojis"}]'::jsonb);
