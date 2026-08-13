-- ============================================================================
-- Game Portal — switch image Terraria de ryshe/terraria a beardedio/terraria
-- ============================================================================
-- L'image ryshe/terraria a un bootstrap.sh fragile (bug "[: =: unexpected
-- operator" ligne 11) et un comportement inconsistent autour de
-- WORLD/AUTOCREATE/WORLD_FILENAME selon les versions.
--
-- beardedio/terraria :
--  - Auto-cree un monde si /world est vide (pas de flag necessaire)
--  - Bootstrap propre, pas de bugs shell
--  - Volume : /world (pas /root/.local/share/...)
--  - Env vars simples : WORLD_NAME, WORLD_DIFFICULTY, WORLD_SIZE, MAX_PLAYERS

UPDATE game_templates
SET
    image = 'beardedio/terraria:latest',
    volume_path = '/world',
    run_as_root = TRUE,
    default_env = '{
        "WORLD_NAME": "Sentinel",
        "WORLD_SIZE": "Medium",
        "WORLD_DIFFICULTY": "Normal",
        "MAX_PLAYERS": "8"
    }'::jsonb,
    config_schema = '[
        {"key": "WORLD_NAME", "label": "Nom du monde", "type": "text", "default": "Sentinel"},
        {"key": "WORLD_SIZE", "label": "Taille du monde", "type": "enum", "default": "Medium", "options": ["Small", "Medium", "Large"]},
        {"key": "WORLD_DIFFICULTY", "label": "Difficulte", "type": "enum", "default": "Normal", "options": ["Normal", "Expert", "Master", "Journey"]},
        {"key": "MAX_PLAYERS", "label": "Joueurs max", "type": "number", "default": 8, "min": 1, "max": 16},
        {"key": "MOTD", "label": "Message d''accueil", "type": "text", "default": "Welcome to Sentinel Terraria!"},
        {"key": "PASSWORD", "label": "Mot de passe (vide = libre)", "type": "text", "default": ""},
        {"key": "WORLD_SEED", "label": "Seed monde (vide = aleatoire)", "type": "text", "default": ""}
    ]'::jsonb,
    description = 'Bac a sable 2D, exploration et boss. Image beardedio/terraria (auto-create world).',
    updated_at = NOW()
WHERE slug = 'terraria';
