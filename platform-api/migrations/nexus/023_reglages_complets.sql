-- 023_reglages_complets.sql
--
-- Ouvre l'equilibrage des deux jeux au reglage par serveur.
--
-- Trois reglages MENTAIENT jusqu'ici : le prix de la garantie (ecrit en dur
-- dans la requete SQL), la duree de la garantie (INTERVAL '1 hour') et le
-- gain d'un pari (amount*2). Le premier etait affiche a l'ecran et ne
-- changeait rien ; les deux autres n'existaient meme pas. C'est repare cote
-- code, et les curseurs apparaissent ici.
--
-- Tous les defauts reproduisent EXACTEMENT le comportement actuel. Un serveur
-- qui ne touche a rien ne voit aucun changement — condition pour livrer un
-- tel lot sans surprise.
--
-- Ce qui n'est PAS ici, volontairement :
--   - les cases de la Roue : elles ont leur propre table et leur editeur
--     (migration 022), parce qu'une liste d'objets ne s'edite pas dans un
--     formulaire pilote par ce schema.
--   - le taux d'arnaque de la garantie (5 %) : c'est la blague de la
--     mecanique. A 0 l'achat n'a plus d'histoire, a 100 ce n'est plus une
--     garantie.

-- ── Coussin Piégé ──

UPDATE bot_definitions SET config_schema = config_schema || '[
  {"key": "combat_mise_min", "type": "number", "label": "Mise minimum", "default": "1",
   "min": 1, "required": false, "group": "Bagarres", "unit": "coins"},

  {"key": "combat_mise_max", "type": "number", "label": "Mise maximum", "default": "0",
   "min": 0, "required": false, "group": "Bagarres", "unit": "coins",
   "description": "0 = pas de plafond.",
   "warning": "Sans plafond, deux gros joueurs peuvent se transferer des fortunes en une bagarre."},

  {"key": "level_gap_max", "type": "number", "label": "Ecart de niveau tolere", "default": "9",
   "min": 0, "max": 200, "required": false, "group": "Bagarres",
   "description": "Au-dela, la bagarre est refusee. En dessous, le plus haut niveau est penalise progressivement.",
   "warning": "Un ecart large expose les debutants aux joueurs installes, et c''est ce qui les fait partir."},

  {"key": "combat_max_rounds", "type": "number", "label": "Nombre de manches", "default": "0",
   "min": 0, "max": 10, "required": false, "group": "Bagarres",
   "description": "0 = automatique (3, 5 ou 7 selon le Confort des deux joueurs)."},

  {"key": "bet_payout_pct", "type": "number", "label": "Gain d''un pari gagnant", "default": "200",
   "min": 100, "max": 1000, "required": false, "group": "Paris", "unit": "%",
   "description": "200 = la mise est doublee.",
   "warning": "Au-dela de 200 %, parier rapporte plus que se battre et le jeu se vide de ses bagarres."},

  {"key": "insurance_duration_minutes", "type": "number", "label": "Duree de la garantie", "default": "60",
   "min": 1, "max": 10080, "required": false, "group": "Coffre a coussins", "unit": "min"},

  {"key": "shop_price_pct", "type": "number", "label": "Prix des objets", "default": "100",
   "min": 10, "max": 1000, "required": false, "group": "Coffre a coussins", "unit": "%",
   "description": "Applique au tarif catalogue. 200 double tous les prix.",
   "warning": "En dessous de 50 %, les objets cessent d''etre un choix : on les prend tous."},

  {"key": "max_level", "type": "number", "label": "Niveau maximum", "default": "25",
   "min": 1, "max": 200, "required": false, "group": "Progression",
   "description": "Longueur de la course. Les titres sont repartis sur cette echelle."},

  {"key": "xp_winner", "type": "number", "label": "Experience du vainqueur", "default": "15",
   "min": 0, "max": 1000, "required": false, "group": "Progression"},

  {"key": "xp_loser", "type": "number", "label": "Experience du perdant", "default": "5",
   "min": 0, "max": 1000, "required": false, "group": "Progression",
   "description": "Un perdant qui gagne un peu d''experience continue de jouer. A zero, perdre devient punitif."},

  {"key": "stat_points_per_level", "type": "number", "label": "Points gagnes par niveau", "default": "3",
   "min": 0, "max": 20, "required": false, "group": "Progression"}
]'::jsonb
WHERE bot_name = 'nexus-coussin';

-- Le prix de la garantie etait affiche mais jamais preleve : la requete
-- retirait 50 coins quoi qu'il arrive. Le code lit desormais ce reglage ; on
-- aligne le defaut sur ce que les joueurs payaient reellement.
UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'key' = 'insurance_cost'
             THEN elem || '{"default": "50", "description": "Preleve a l''achat. Ce reglage etait sans effet avant cette version."}'::jsonb
             ELSE elem END
    )
    FROM jsonb_array_elements(config_schema) AS elem
)
WHERE bot_name = 'nexus-coussin'
  AND config_schema @> '[{"key": "insurance_cost"}]'::jsonb;

-- ── Roue du Destin ──
--
-- Les cases s'editent sur leur propre page ; ce qui reste ici, c'est ce qui
-- s'applique PAR-DESSUS elles.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE WHEN elem ->> 'key' = 'wheel_payout_multiplier'
             THEN elem || '{"description": "Applique aux gains ET aux pertes de chaque case, par-dessus la roue du serveur. 200 double les deux."}'::jsonb
             ELSE elem END
    )
    FROM jsonb_array_elements(config_schema) AS elem
)
WHERE bot_name = 'nexus-economy'
  AND config_schema @> '[{"key": "wheel_payout_multiplier"}]'::jsonb;
