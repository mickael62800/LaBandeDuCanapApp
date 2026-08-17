-- 044_palworld_palbox_global.sql
--
-- Palbox global : import et export de Pals entre serveurs.
--
-- Deux reglages manquaient au schema Palworld, alors qu'ils decident de
-- quelque chose de sensible : `bAllowGlobalPalboxExport` et
-- `bAllowGlobalPalboxImport`, c'est-a-dire la possibilite pour un joueur
-- d'emporter ses Pals vers un autre serveur, ou d'en ramener depuis ailleurs.
--
-- L'import est le plus lourd de consequences : un Pal venu d'un serveur ou
-- les taux sont debrides arrive tel quel, et rend sans interet la progression
-- de ceux qui ont joue ici. C'est un choix de communaute, il doit donc etre
-- visible et reglable, pas subi.
--
-- Les cles reprennent les variables d'environnement de l'image
-- `thijsvanloef/palworld-server-docker`, comme tout le reste du schema
-- (`CROSSPLAY_PLATFORMS`, `BAN_LIST_URL`...), et non les noms INI du jeu.
--
-- Idempotente : les cles sont retirees avant d'etre reecrites.

UPDATE game_templates SET config_schema = (
    SELECT COALESCE(jsonb_agg(elem ORDER BY ord), '[]'::jsonb)
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
    WHERE elem ->> 'key' NOT IN (
        'ALLOW_GLOBAL_PALBOX_EXPORT', 'ALLOW_GLOBAL_PALBOX_IMPORT'
    )
)
WHERE slug = 'palworld';

UPDATE game_templates SET config_schema = config_schema || '[
  {"key": "ALLOW_GLOBAL_PALBOX_EXPORT", "type": "boolean", "label": "Autoriser l''export de Pals",
   "group": "Regles du jeu", "default": "true", "required": false,
   "description": "Permet aux joueurs d''emporter leurs Pals vers un autre serveur."},

  {"key": "ALLOW_GLOBAL_PALBOX_IMPORT", "type": "boolean", "label": "Autoriser l''import de Pals",
   "group": "Regles du jeu", "default": "true", "required": false,
   "description": "Permet aux joueurs de ramener des Pals venus d''un autre serveur.",
   "warning": "Un Pal eleve sur un serveur aux taux debrides arrive ici tel quel, et rend sans interet la progression de ceux qui ont joue sur ce serveur."}
]'::jsonb
WHERE slug = 'palworld';
