-- Depart eclair : retirer le mot d'accueil d'Atrium quand le membre repart
-- aussitot.
--
-- Pendant de `welcome_ghost_minutes` cote Sentinel
-- (sentinel-api/migrations/026_welcome_depart_eclair.sql) : un membre qui
-- rejoint, regarde et quitte dans la foulee laissait deux cards Sentinel ET le
-- message d'accueil d'Atrium, adresse a quelqu'un qui n'est plus la.
--
-- Le seuil est declare par plateforme parce que chaque plateforme a sa propre
-- base logique, donc sa propre table `bot_guild_config` : Atrium ne lit jamais
-- celle de Sentinel. Les deux valeurs sont a aligner a la main sur un serveur
-- donne.
--
-- 0 desactive le comportement (le message reste, comme avant).

UPDATE bot_definitions SET
    config_schema = config_schema || '[
      {"key": "welcome_ghost_minutes", "type": "number", "unit": "min", "min": 0, "max": 1440, "label": "Depart eclair : delai (minutes)", "default": "30", "required": false,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "Si un membre accueilli quitte le serveur dans ce delai, le message d accueil poste dans le general est supprime. 0 pour desactiver."}
    ]'::jsonb
WHERE bot_name = 'atrium-bot'
  AND NOT (config_schema @> '[{"key": "welcome_ghost_minutes"}]'::jsonb);
