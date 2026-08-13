-- 005_paliers_de_roles.sql
--
-- Retablit l'attribution de roles par palier de niveau.
--
-- La fonctionnalite avait sa propre table (`level_rewards`), supprimee par une
-- migration ancienne. Elle revient sous forme de reglage, au meme format que
-- les multiplicateurs XP deja en place — le back-office genere ses formulaires
-- depuis `config_schema`, donc un reglage s'edite sans ecrire de front, la ou
-- une table demanderait des routes CRUD et un ecran dedie pour un contenu qui
-- tient en trois lignes.
--
-- Le role de DEPART reste `default_role_ids` : il est donne a l'arrivee, pas
-- a un niveau. Les paliers prennent le relais ensuite.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "level_role_rewards", "type": "text",
   "label": "Paliers de roles (niveau:role_id)",
   "required": false,
   "depends_on": {"key": "enabled", "equals": "true"},
   "description": "Un role attribue a partir d''un niveau. Format niveau:role_id, separes par des virgules ou des retours a la ligne. Ex : 5:111,10:222,20:333. Les entrees illisibles sont ignorees sans bloquer les autres."},

  {"key": "level_role_mode", "type": "enum",
   "label": "Que faire des paliers precedents",
   "default": "cumulatif",
   "required": false,
   "options": ["cumulatif", "remplacement"],
   "depends_on": {"key": "level_role_rewards", "equals": ""},
   "description": "Cumulatif : le membre garde tous les roles obtenus. Remplacement : il ne porte que le palier atteint, les autres sont retires — a choisir quand les roles sont des rangs qui se succedent.",
   "warning": "En mode remplacement, le bot RETIRE les roles des autres paliers. Ne mets dans les paliers que des roles dedies a la progression : un role liste ici sera enleve aux membres qui ne sont plus au bon niveau, meme s''il leur avait ete donne a la main."}
]'::jsonb
WHERE bot_name = 'progression-bot'
  AND NOT config_schema @> '[{"key": "level_role_rewards"}]'::jsonb;


-- La description de `max_level` promettait deja que « les role rewards
-- continuent » alors que la fonctionnalite n'existait plus. Elle redevient
-- vraie, mais autant la formuler dans les termes du nouveau reglage.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'key' = 'max_level'
             THEN elem || jsonb_build_object('description',
                  'Niveau max au-dela duquel les annonces level-up sont supprimees. 0 = illimite. Les paliers de roles continuent de s''appliquer.')
             ELSE elem END
        ORDER BY ord
    )
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
)
WHERE bot_name = 'progression-bot'
  AND config_schema @> '[{"key": "max_level"}]'::jsonb;
