-- Depart eclair : ne rien laisser dans le salon quand un membre arrive puis
-- repart aussitot.
--
-- Le cas visé : les comptes qui rejoignent, regardent, et quittent au bout de
-- quelques minutes. Chacun laissait deux cards (bienvenue + depart) dans le
-- salon d'accueil, pour un membre qui n'a jamais existe. Passe ce delai, le
-- membre a reellement fait partie du serveur et les deux cards sont legitimes.
--
-- 0 desactive le comportement (on garde les deux cards, comme avant).

UPDATE bot_definitions SET
    config_schema = config_schema || '[
        {"key": "welcome_ghost_minutes", "type": "number", "label": "Depart eclair : delai (minutes)", "required": false, "default": "30", "min": 0, "max": 1440, "description": "Si un nouveau membre quitte le serveur dans ce delai, sa card de bienvenue est supprimee et aucune card de depart n''est publiee. 0 pour desactiver."}
    ]'::jsonb
WHERE bot_name = 'welcome-bot'
  AND NOT (config_schema @> '[{"key": "welcome_ghost_minutes"}]'::jsonb);
