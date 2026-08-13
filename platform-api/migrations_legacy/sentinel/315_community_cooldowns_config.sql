-- ============================================================================
-- Community — cooldowns configurables par serveur.
-- ============================================================================
-- Deux valeurs etaient codees en dur cote community-bot :
--   - le cooldown anti-spam de /parrain (30s)
--   - le cooldown du bouton de toggle de role dans les panneaux de roles (2s)
-- On les expose au config_schema du module `community-bot` pour un reglage
-- par serveur. Le bot lit ces cles via bot_guild_config ; en leur absence il
-- retombe sur les valeurs par defaut (30 et 2) — aucun changement de
-- comportement pour les serveurs existants.
--
-- Idempotent : n'ajoute chaque cle que si elle est absente du schema.

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "sponsor_cooldown_secs", "label": "Parrainage : cooldown /parrain", "type": "number", "required": false, "default": "30", "min": 0, "max": 3600, "unit": "s", "description": "Delai minimum entre deux commandes /parrain pour un meme membre (anti-spam)."}
]'::jsonb
WHERE bot_name = 'community-bot'
  AND NOT (config_schema @> '[{"key": "sponsor_cooldown_secs"}]'::jsonb);

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "role_button_cooldown_secs", "label": "Panneaux de roles : cooldown bouton", "type": "number", "required": false, "default": "2", "min": 0, "max": 3600, "unit": "s", "description": "Delai minimum entre deux clics sur un bouton de role (anti-spam du toggle)."}
]'::jsonb
WHERE bot_name = 'community-bot'
  AND NOT (config_schema @> '[{"key": "role_button_cooldown_secs"}]'::jsonb);
