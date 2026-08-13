-- ============================================================================
-- Game Portal — init_files + command_template + revert Terraria sur ryshe
-- ============================================================================
-- Beaucoup d'images de serveur de jeu (Terraria/ryshe, ARK, 7DTD, Astroneer,
-- etc.) attendent qu'on pose un fichier de configuration sur leur volume
-- AVANT le premier demarrage, sinon elles plantent ou tombent en menu
-- interactif. Aucune ne sait generer ce fichier depuis des env vars.
--
-- On generalise le besoin avec deux nouvelles colonnes sur game_templates :
--   - init_files JSONB     : liste de {path, content} a uploader dans le
--                            container apres create et avant start. Le
--                            content peut contenir des {{KEY}} qui seront
--                            substitues par les valeurs des env (defaults
--                            + overrides utilisateur). Source de verite =
--                            DB, pas le volume Docker -> modifier la config
--                            depuis l'UI regenere le fichier au prochain
--                            start.
--   - command_template TEXT: command Docker a passer (override CMD de
--                            l'image). Egalement templated avec {{KEY}}.
--                            Stocke en JSON array (["bin", "arg1", ...]).
--                            NULL = utilise l'ENTRYPOINT/CMD de l'image.
--
-- Pour Terraria : retour sur ryshe/terraria:tshock-1.4.5.6-6.1.0 (tshock,
-- RCON natif, plugins). Init :
--   - /tshock/config.json minimal (sinon bug bootstrap.sh:11 jq)
-- Command override avec -autocreate pour forcer l'auto-create du monde
-- (pas de menu interactif).

ALTER TABLE game_templates
    ADD COLUMN IF NOT EXISTS init_files       JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS command_template TEXT;

COMMENT ON COLUMN game_templates.init_files IS
    'Fichiers a uploader dans le container avant start. Format : [{"path": "/abs/path", "content": "templatable {{KEY}}"}]. Les {{KEY}} sont remplaces par les env vars (defaults + overrides) au moment du start.';
COMMENT ON COLUMN game_templates.command_template IS
    'Command Docker (JSON array string : [\"bin\", \"arg1\", \"arg2\"]). Override le CMD de l''image. Templatable via {{KEY}}. NULL = laisse l''image decider.';

-- ─── Terraria : retour sur ryshe/terraria + init_files + command ──────────
-- /tshock/config.json minimal pour eviter le crash de bootstrap.sh ligne 11.
-- Command : -autocreate <SIZE> -worldname <NAME> -world <PATH> -port -maxplayers
-- WORLD_FILENAME garde l'extension .wld (le binaire Terraria attend ca).
UPDATE game_templates
SET
    image            = 'ryshe/terraria:tshock-1.4.5.6-6.1.0',
    volume_path      = '/root/.local/share/Terraria/Worlds',
    run_as_root      = TRUE,
    description      = 'Bac a sable 2D, exploration et boss. Image ryshe (tshock + RCON + plugins).',
    default_env      = '{
        "WORLD_FILENAME": "world1.wld",
        "WORLD_NAME": "Sentinel",
        "WORLD_SIZE": "2",
        "DIFFICULTY": "0",
        "MAX_PLAYERS": "8",
        "MOTD": "Welcome to Sentinel Terraria!",
        "PASSWORD": "",
        "CONFIGPATH": "/tshock",
        "LOGPATH": "/tshock/logs"
    }'::jsonb,
    config_schema    = '[
        {"key": "WORLD_NAME", "label": "Nom du monde", "type": "text", "default": "Sentinel"},
        {"key": "WORLD_SIZE", "label": "Taille du monde", "type": "enum", "default": "2", "options": ["1", "2", "3"]},
        {"key": "DIFFICULTY", "label": "Difficulte", "type": "enum", "default": "0", "options": ["0", "1", "2", "3"]},
        {"key": "MAX_PLAYERS", "label": "Joueurs max", "type": "number", "default": 8, "min": 1, "max": 16},
        {"key": "MOTD", "label": "Message d''accueil", "type": "text", "default": "Welcome to Sentinel Terraria!"},
        {"key": "PASSWORD", "label": "Mot de passe (vide = libre)", "type": "text", "default": ""}
    ]'::jsonb,
    init_files       = '[
        {
            "path": "/tshock/config.json",
            "content": "{\n  \"Settings\": {\n    \"StorageType\": \"sqlite\",\n    \"ServerPort\": 7777,\n    \"MaxSlots\": {{MAX_PLAYERS}},\n    \"ServerPassword\": \"{{PASSWORD}}\",\n    \"ServerName\": \"{{WORLD_NAME}}\",\n    \"AutoSave\": true\n  }\n}\n"
        }
    ]'::jsonb,
    command_template = '["-world", "/root/.local/share/Terraria/Worlds/{{WORLD_FILENAME}}", "-autocreate", "{{WORLD_SIZE}}", "-worldname", "{{WORLD_NAME}}", "-difficulty", "{{DIFFICULTY}}", "-port", "7777", "-maxplayers", "{{MAX_PLAYERS}}", "-motd", "{{MOTD}}"]',
    updated_at       = NOW()
WHERE slug = 'terraria';
