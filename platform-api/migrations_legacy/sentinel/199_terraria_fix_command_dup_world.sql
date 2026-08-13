-- ============================================================================
-- Game Portal — Terraria : retire -world de command (duplique avec ryshe)
-- ============================================================================
-- Le bootstrap de ryshe/terraria lit l'env WORLD_FILENAME et ajoute lui-meme
-- "-world /root/.local/share/Terraria/Worlds/$WORLD_FILENAME" a la command.
-- Notre command_template re-passait "-world ..." -> double argument ->
-- TerrariaApi.Server.ServerApi crash :
--   System.ArgumentException: An item with the same key has already been
--   added. Key: -world
--
-- Fix : on retire "-world" de notre command (delegue a ryshe via env) et on
-- garde uniquement les flags qu'il n'injecte pas : -autocreate, -worldname,
-- -difficulty, -port, -maxplayers, -motd.

UPDATE game_templates
SET
    command_template = '["-autocreate", "{{WORLD_SIZE}}", "-worldname", "{{WORLD_NAME}}", "-difficulty", "{{DIFFICULTY}}", "-port", "7777", "-maxplayers", "{{MAX_PLAYERS}}", "-motd", "{{MOTD}}"]',
    updated_at = NOW()
WHERE slug = 'terraria';
