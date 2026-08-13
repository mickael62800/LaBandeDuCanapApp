-- ============================================================================
-- Game Portal — fix env vars Terraria (ryshe/terraria)
-- ============================================================================
-- Migration 193 utilisait AUTOCREATE qui ne fonctionne pas avec ryshe/terraria.
-- L'image attend WORLD=N (1=small, 2=medium, 3=large) pour declencher
-- l'auto-creation d'un monde. WORLD_FILENAME ne doit PAS etre defini
-- au premier lancement (sinon l'image cherche un fichier inexistant
-- au lieu de generer le monde).
--
-- Apres premier boot, le bootstrap.sh genere world1.wld dans
-- /root/.local/share/Terraria/Worlds, et les redemarrages suivants
-- chargent ce fichier automatiquement.

UPDATE game_templates
SET
    default_env = '{
        "WORLD": "2",
        "DIFFICULTY": "0",
        "MAXPLAYERS": "8",
        "MOTD": "Welcome to Sentinel Terraria!"
    }'::jsonb,
    config_schema = '[
        {"key": "WORLD", "label": "Taille du monde (auto-create premier boot)", "type": "enum", "default": "2", "options": ["1", "2", "3"]},
        {"key": "DIFFICULTY", "label": "Difficulte (0=Classic, 1=Expert, 2=Master, 3=Journey)", "type": "enum", "default": "0", "options": ["0", "1", "2", "3"]},
        {"key": "MAXPLAYERS", "label": "Joueurs max", "type": "number", "default": 8, "min": 1, "max": 16},
        {"key": "MOTD", "label": "Message d''accueil", "type": "text", "default": "Welcome to Sentinel Terraria!"},
        {"key": "PASSWORD", "label": "Mot de passe (vide = pas de mdp)", "type": "text", "default": ""}
    ]'::jsonb,
    updated_at = NOW()
WHERE slug = 'terraria';
