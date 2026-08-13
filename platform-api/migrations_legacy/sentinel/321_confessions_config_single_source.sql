-- ============================================================================
-- CONFESSIONS : consolidation de la config sur UNE seule source de verite.
-- ============================================================================
-- Avant cette migration, la config des confessions etait SPLIT entre deux
-- stores :
--   - `bot_guild_config` (composant `confessions`) : lu par le bot pour
--     l'affichage (min/max_chars des modales, couleur, archivage thread...).
--   - la table `confession_config` : lue par le domaine (ManageConfessionsService)
--     pour l'ENFORCEMENT (enabled, min/max_chars, cooldown_secs, max_per_day,
--     quota_window_hours, channel_id) + la liste des bannis.
--
-- Consequence : les cles `cooldown_secs` / `max_per_day` / `channel_id` du
-- schema generique etaient MORTES (jamais appliquees), et `min_chars`/`max_chars`
-- existaient en double (divergence silencieuse possible).
--
-- Desormais `bot_guild_config` (composant `confessions`) est la SOURCE UNIQUE
-- des reglages. La table `confession_config` ne sert plus qu'a stocker la
-- DONNEE `banned_user_ids` (les autres colonnes sont conservees pour eviter
-- tout risque, mais ne sont plus lues).
--
-- Idempotent : cle ajoutee seulement si absente ; copie via ON CONFLICT DO
-- NOTHING (ne clobbe jamais une valeur deja definie sur bot_guild_config).

-- 1. SCHEMA -------------------------------------------------------------------
-- Les cles enabled / channel_id / cooldown_secs / max_per_day / min_chars /
-- max_chars sont deja presentes dans le schema `confessions` (migration 185).
-- Seule `quota_window_hours` (jusqu'ici une colonne de `confession_config`,
-- migration 314) manque : on l'ajoute.
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "quota_window_hours", "label": "Quota — fenetre glissante (heures)", "type": "number", "required": false, "default": "24", "min": 1, "max": 168, "unit": "heures", "description": "Fenetre glissante (en heures) sur laquelle le nombre max de confessions par jour et par utilisateur est compte. Defaut 24h (bornee a >= 1h a l usage)."}
]'::jsonb
WHERE bot_name = 'confessions'
  AND NOT (config_schema @> '[{"key": "quota_window_hours"}]'::jsonb);

-- 2. COPIE DES DONNEES --------------------------------------------------------
-- Pour chaque serveur ayant deja configure `confession_config`, on recopie ses
-- reglages vers `bot_guild_config` (composant `confessions`) afin qu'aucun
-- serveur ne perde son parametrage. On n'ecrit QUE si la cle n'existe pas deja
-- cote bot_guild_config (ON CONFLICT DO NOTHING) : les valeurs deja definies
-- via le dashboard restent prioritaires.
--
-- Les valeurs NULL (channel_id / panel_message_id non renseignes) sont
-- filtrees car `bot_guild_config.config_value` est NOT NULL.
INSERT INTO bot_guild_config (id, guild_id, bot_name, config_key, config_value, updated_at)
SELECT gen_random_uuid(), cc.guild_id, 'confessions', kv.key, kv.value, NOW()
FROM confession_config cc
CROSS JOIN LATERAL (VALUES
    ('enabled', CASE WHEN cc.enabled THEN 'true' ELSE 'false' END),
    ('cooldown_secs', cc.cooldown_secs::text),
    ('max_per_day', cc.max_per_day::text),
    ('quota_window_hours', cc.quota_window_hours::text),
    ('min_chars', cc.min_chars::text),
    ('max_chars', cc.max_chars::text),
    ('channel_id', cc.channel_id),
    ('panel_message_id', cc.panel_message_id)
) AS kv(key, value)
WHERE kv.value IS NOT NULL
ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

-- Note : les colonnes de reglage de `confession_config` (enabled, channel_id,
-- panel_message_id, cooldown_secs, max_per_day, quota_window_hours, min_chars,
-- max_chars, automod_enabled) ne sont PLUS lues par le code apres cette
-- migration. Elles sont laissees en place (pas de DROP) pour eviter tout
-- risque ; seule `banned_user_ids` reste activement utilisee.
