-- 048_palworld_audit_exhaustif.sql
--
-- Audit ligne a ligne du schema Palworld, confronte au CODE de l'image
-- `thijsvanloef/palworld-server-docker` (scripts/compile-settings.sh,
-- start.sh, update.sh, backup.sh, auto_reboot.sh) — la source qui ecrit
-- reellement `PalWorldSettings.ini`, plutot qu'a une documentation.
--
-- Quatre corrections, dont une qui rendait six reglages inertes.
--
-- ── 1. DECREACE -> DECREASE (quatre reglages morts) ──
--
-- Le JEU ecrit `PlayerStomachDecreaceRate` dans son fichier INI — faute
-- d'orthographe d'origine. L'IMAGE, elle, attend `PLAYER_STOMACH_DECREASE_RATE`
-- et se charge de la traduire. Nous envoyions la graphie du jeu : l'image ne
-- reconnaissait pas la variable et gardait sa valeur par defaut.
--
-- Consequence : la faim et l'endurance, joueurs comme Pals, ne changeaient
-- JAMAIS, quoi qu'on regle a l'ecran. Quatre curseurs sans effet.
--
-- ── 2. ALLOW_CONNECT_PLATFORM est deprecie ──
--
-- `compile-settings.sh` le dit explicitement : « ALLOW_CONNECT_PLATFORM is
-- deprecated and will not be applied to the PalWorldSettings.ini. Please use
-- CROSSPLAY_PLATFORMS instead. » Le reglage etait donc affiche, modifiable, et
-- sans aucun effet. `CROSSPLAY_PLATFORMS` figure deja dans le schema.
--
-- ── 3. LOG_FORMAT_TYPE attend « Text », pas « text » ──
--
-- L'image pose `LOG_FORMAT_TYPE=${LOG_FORMAT_TYPE:-Text}` et recopie la valeur
-- telle quelle dans l'INI. Nos options minuscules partaient donc au serveur
-- sous une graphie qu'il ne reconnait pas.
--
-- ── 4. BAN_LIST_URL a change d'hote ──
--
-- L'image pointe sur `b.palworldgame.com`, nous sur `api.palworldgame.com`.
-- Une liste de bannis qui ne repond pas ne banni personne, en silence.
--
-- Ce qui reste volontairement different de l'image, et n'est donc PAS touche :
-- `PLAYERS` (16 contre 32), `RCON_ENABLED` (actif, l'administration en depend),
-- `SHOW_PLAYER_LIST`, `SERVER_NAME`. Ce sont des choix de communaute, pas des
-- erreurs.

-- ── 1. Les quatre cles mal orthographiees ──

UPDATE game_templates SET config_schema = (
    SELECT jsonb_agg(
        CASE elem ->> 'key'
            WHEN 'PLAYER_STOMACH_DECREACE_RATE' THEN elem || '{"key": "PLAYER_STOMACH_DECREASE_RATE"}'::jsonb
            WHEN 'PLAYER_STAMINA_DECREACE_RATE' THEN elem || '{"key": "PLAYER_STAMINA_DECREASE_RATE"}'::jsonb
            WHEN 'PAL_STOMACH_DECREACE_RATE'    THEN elem || '{"key": "PAL_STOMACH_DECREASE_RATE"}'::jsonb
            WHEN 'PAL_STAMINA_DECREACE_RATE'    THEN elem || '{"key": "PAL_STAMINA_DECREASE_RATE"}'::jsonb
            ELSE elem
        END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE slug = 'palworld';

-- Les valeurs deja reglees suivent leur cle : sans cela, un serveur ayant
-- baisse la faim retrouverait le reglage a son defaut sans comprendre pourquoi.
UPDATE game_server_configs c SET config_key = replace(c.config_key, 'DECREACE', 'DECREASE')
FROM game_servers s, game_templates t
WHERE c.server_id = s.id AND s.template_id = t.id AND t.slug = 'palworld'
  AND c.config_key LIKE '%DECREACE%'
  AND NOT EXISTS (
      SELECT 1 FROM game_server_configs d
      WHERE d.server_id = c.server_id
        AND d.config_key = replace(c.config_key, 'DECREACE', 'DECREASE')
  );

-- ── 2. Reglage deprecie par l'image ──

UPDATE game_templates SET config_schema = (
    SELECT COALESCE(jsonb_agg(elem ORDER BY ord), '[]'::jsonb)
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
    WHERE elem ->> 'key' <> 'ALLOW_CONNECT_PLATFORM'
)
WHERE slug = 'palworld';

-- ── 3 et 4. Graphies et defauts ──

UPDATE game_templates SET config_schema = (
    SELECT jsonb_agg(
        CASE elem ->> 'key'
            WHEN 'LOG_FORMAT_TYPE' THEN elem || '{"default": "Text", "options": ["Text", "Json"]}'::jsonb
            WHEN 'BAN_LIST_URL' THEN elem || '{"default": "https://b.palworldgame.com/api/banlist.txt"}'::jsonb
            ELSE elem
        END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE slug = 'palworld';

UPDATE game_server_configs c SET config_value = 'Text'
FROM game_servers s, game_templates t
WHERE c.server_id = s.id AND s.template_id = t.id AND t.slug = 'palworld'
  AND c.config_key = 'LOG_FORMAT_TYPE' AND c.config_value = 'text';

UPDATE game_server_configs c SET config_value = 'Json'
FROM game_servers s, game_templates t
WHERE c.server_id = s.id AND s.template_id = t.id AND t.slug = 'palworld'
  AND c.config_key = 'LOG_FORMAT_TYPE' AND c.config_value = 'json';
