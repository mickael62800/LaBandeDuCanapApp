-- ============================================================================
-- Game Portal — fix Terraria + support volumes/users heterogenes par jeu
-- ============================================================================
-- Chaque image Docker a sa propre convention :
--  - itzg/minecraft-server : /data, user 1000:1000
--  - ryshe/terraria        : /root/.local/share/Terraria/Worlds, user root
--  - lloesche/valheim      : /config et /opt/valheim, user root
--  - factoriotools/factorio: /factorio, user 845
--  - palworld              : /palworld
--
-- On ajoute 2 colonnes au schema pour gerer ces specificites :
--  - volume_path  : point de montage du volume nomme (defaut /data)
--  - run_as_root  : si TRUE, l'API ne passe PAS --user 1000:1000

ALTER TABLE game_templates
    ADD COLUMN IF NOT EXISTS volume_path VARCHAR(255) NOT NULL DEFAULT '/data';

ALTER TABLE game_templates
    ADD COLUMN IF NOT EXISTS run_as_root BOOLEAN NOT NULL DEFAULT FALSE;

-- ── Fix Terraria ──────────────────────────────────────────────────────
-- L'image ryshe/terraria utilise tshock + bootstrap.sh qui ecrit dans
-- /root/.local/share/Terraria/Worlds et /tshock. Doit tourner en root.
-- Variable AUTOCREATE est lue par le bootstrap pour generer un monde si
-- inexistant (1 = small, 2 = medium, 3 = large).
UPDATE game_templates
SET
    volume_path = '/root/.local/share/Terraria/Worlds',
    run_as_root = TRUE,
    default_env = '{
        "WORLD_FILENAME": "world1.wld",
        "AUTOCREATE": "2",
        "DIFFICULTY": "0",
        "MAXPLAYERS": "8"
    }'::jsonb,
    config_schema = '[
        {"key": "WORLD_FILENAME", "label": "Nom du fichier monde", "type": "text", "default": "world1.wld"},
        {"key": "AUTOCREATE", "label": "Taille (autocreate si absent)", "type": "enum", "default": "2", "options": ["1", "2", "3"]},
        {"key": "DIFFICULTY", "label": "Difficulte", "type": "enum", "default": "0", "options": ["0", "1", "2", "3"]},
        {"key": "MAXPLAYERS", "label": "Joueurs max", "type": "number", "default": 8, "min": 1, "max": 16},
        {"key": "MOTD", "label": "Message d''accueil", "type": "text", "default": "Welcome to Sentinel Terraria!"},
        {"key": "PASSWORD", "label": "Mot de passe (vide = pas de mdp)", "type": "text", "default": ""}
    ]'::jsonb,
    updated_at = NOW()
WHERE slug = 'terraria';

-- ── Valheim ──────────────────────────────────────────────────────────
-- L'image lloesche/valheim utilise /config (ses configs) + /opt/valheim
-- (le world). On expose seulement /config. Doit tourner en root.
UPDATE game_templates
SET
    volume_path = '/config',
    run_as_root = TRUE,
    updated_at = NOW()
WHERE slug = 'valheim';

-- ── Factorio ─────────────────────────────────────────────────────────
-- factoriotools/factorio tourne en user 845 (factorio) par defaut, ce
-- qui differe de notre 1000:1000. On laisse l'image gerer son user.
UPDATE game_templates
SET
    volume_path = '/factorio',
    run_as_root = TRUE,
    updated_at = NOW()
WHERE slug = 'factorio';

-- ── Palworld ─────────────────────────────────────────────────────────
-- thijsvanloef/palworld-server-docker utilise /palworld, user steam
-- (1000:1000) - reste sur run_as_root=false (defaut).
UPDATE game_templates
SET
    volume_path = '/palworld',
    updated_at = NOW()
WHERE slug = 'palworld';

-- Minecraft reste en /data + user 1000:1000 (defauts), pas de changement.
