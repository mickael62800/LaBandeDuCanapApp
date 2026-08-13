-- ============================================================================
-- Game Portal — fix env vars Terraria pour beardedio/terraria
-- ============================================================================
-- L'image attend des valeurs NUMERIQUES et noms specifiques :
--   WORLD_FILENAME         : nom du fichier monde (sans .wld)
--   WORLD_SIZE_NUM         : 1=small, 2=medium, 3=large
--   WORLD_DIFFICULTY_NUM   : 0=classic, 1=expert, 2=master, 3=journey
--   MAX_PLAYERS            : nb de joueurs
--   MOTD                   : message d'accueil
-- L'entrypoint genere /config/serverconfig.txt avec ces valeurs et
-- l'argument -autocreate $WORLD_SIZE_NUM.

UPDATE game_templates
SET
    default_env = '{
        "WORLD_FILENAME": "Sentinel",
        "WORLD_SIZE_NUM": "2",
        "WORLD_DIFFICULTY_NUM": "0",
        "MAX_PLAYERS": "8",
        "MOTD": "Welcome to Sentinel Terraria!"
    }'::jsonb,
    config_schema = '[
        {"key": "WORLD_FILENAME", "label": "Nom du fichier monde", "type": "text", "default": "Sentinel"},
        {"key": "WORLD_SIZE_NUM", "label": "Taille du monde", "type": "enum", "default": "2", "options": ["1", "2", "3"]},
        {"key": "WORLD_DIFFICULTY_NUM", "label": "Difficulte", "type": "enum", "default": "0", "options": ["0", "1", "2", "3"]},
        {"key": "MAX_PLAYERS", "label": "Joueurs max", "type": "number", "default": 8, "min": 1, "max": 16},
        {"key": "MOTD", "label": "Message d''accueil", "type": "text", "default": "Welcome to Sentinel Terraria!"},
        {"key": "PASSWORD", "label": "Mot de passe (vide = libre)", "type": "text", "default": ""}
    ]'::jsonb,
    updated_at = NOW()
WHERE slug = 'terraria';
