-- ============================================
-- DiscordL VOTE : le meme bot DiscordL poste bump ET vote. On ajoute le
-- provider `discordl_vote` au dashboard (activation + cooldown propre, 12h).
-- La recompense (coins) reutilise les cles partagees bump_reward_*.
-- Idempotent via @>.
-- ============================================

UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "discordl_vote_enabled", "label": "Provider DiscordL Vote actif", "type": "boolean", "required": false, "default": "true", "description": "Recompense les votes DiscordL (discordl.org) — meme bot que le bump, action /vote.", "depends_on": {"key": "enabled", "equals": "true"}},
    {"key": "discordl_vote_cooldown_minutes", "label": "Cooldown DiscordL Vote (minutes)", "type": "number", "required": false, "default": "720", "description": "Delai DiscordL entre deux votes (defaut 720 = 12h).", "depends_on": {"key": "discordl_vote_enabled", "equals": "true"}}
]'::jsonb
WHERE bot_name = 'bump-bot'
  AND NOT (config_schema @> '[{"key": "discordl_vote_enabled"}]'::jsonb);
