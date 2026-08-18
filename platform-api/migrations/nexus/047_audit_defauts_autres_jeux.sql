-- 047_audit_defauts_autres_jeux.sql
--
-- Audit des schemas des autres jeux, verifies contre la documentation de
-- CHAQUE image Docker plutot que de memoire — comme cela vient d'etre fait
-- pour Palworld (migration 046).
--
-- Trois constats, deux natures de probleme.
--
-- ── Factorio : deux curseurs sans effet ──
--
-- `AUTOSAVE_INTERVAL` et `AUTOSAVE_SLOTS` ne sont PAS des variables
-- d'environnement de l'image `factoriotools/factorio` : elles n'apparaissent
-- nulle part dans sa documentation, et le modele ne pose aucun fichier
-- d'initialisation qui les traduirait. Ces reglages de Factorio vivent dans
-- `server-settings.json` ou en argument de ligne de commande.
--
-- Un administrateur pouvait donc regler la sauvegarde automatique sur 10
-- minutes et n'obtenir aucun changement. Un curseur qu'on deplace sans effet
-- est pire que son absence : il fait croire au probleme resolu, et on cherche
-- ailleurs. C'est exactement ce que la migration 016 avait nettoye.
--
-- ── Valheim : deux reglages remplaces par l'image ──
--
-- `BACKUPS_INTERVAL` et `UPDATE_INTERVAL` sont declarees « legacy » par
-- `lloesche/valheim-server-docker`, qui leur a substitue `BACKUPS_CRON` et
-- `UPDATE_CRON`. Or les deux CRON figurent DEJA dans notre schema : le meme
-- reglage y apparaissait donc deux fois, sous deux formes, dont une que
-- l'image n'honore plus. Les heritees partent, les cron restent.
--
-- ── 7 Days to Die : un libelle ambigu ──
--
-- `DayLightLength` n'est pas la duree d'une journee, mais le nombre d'HEURES
-- DE LUMIERE dans un cycle de 24 h (18 par defaut, donc 6 h de nuit). Lu
-- « Duree du jour », le reglage se comprend a l'envers.
--
-- Aucune valeur reglee par un serveur n'est touchee : seules les definitions
-- changent. Les valeurs orphelines des cles retirees restent en base sans
-- effet, comme le fait deja la migration 016.

-- ── Factorio ──

UPDATE game_templates SET config_schema = (
    SELECT COALESCE(jsonb_agg(elem ORDER BY ord), '[]'::jsonb)
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
    WHERE elem ->> 'key' NOT IN ('AUTOSAVE_INTERVAL', 'AUTOSAVE_SLOTS')
)
WHERE slug = 'factorio';

-- ── Valheim ──

UPDATE game_templates SET config_schema = (
    SELECT COALESCE(jsonb_agg(elem ORDER BY ord), '[]'::jsonb)
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
    WHERE elem ->> 'key' NOT IN ('BACKUPS_INTERVAL', 'UPDATE_INTERVAL')
)
WHERE slug = 'valheim';

-- ── 7 Days to Die ──

UPDATE game_templates SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'key' = 'SERVERCONFIG_DayLightLength'
             THEN elem || '{"label": "Heures de lumiere par jour", "description": "Sur un cycle de 24 h : 18 laisse 6 h de nuit."}'::jsonb
             ELSE elem END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE slug = '7dtd';
