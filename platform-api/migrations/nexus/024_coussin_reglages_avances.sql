-- 024_coussin_reglages_avances.sql
--
-- Le reste de l'equilibrage de Coussin Piégé : prix par objet, statistiques
-- des quatre classes, bonus de classe, formules de Confort et de degats,
-- faces du de, et delais entre actions.
--
-- Avec ceci, plus AUCUN chiffre du jeu n'est en dur, a une exception assumee :
-- les 5 % d'arnaque de la garantie, qui sont la blague de la mecanique.
--
-- Tous les defauts reproduisent le comportement actuel. Les delais valent 0 —
-- il n'y en avait aucun, et en imposer un par defaut changerait le jeu sous
-- les pieds des serveurs existants.

UPDATE bot_definitions SET config_schema = config_schema || '[
  {"key": "shop_price_rage", "type": "number", "label": "Prix — Coussin Plombe", "default": "0",
   "min": 0, "required": false, "group": "Prix des objets", "unit": "coins",
   "description": "0 = tarif catalogue (100). Le multiplicateur global s''applique par-dessus."},

  {"key": "shop_price_mindgame", "type": "number", "label": "Prix — Oeil sous le Plaid", "default": "0",
   "min": 0, "required": false, "group": "Prix des objets", "unit": "coins",
   "description": "0 = tarif catalogue (150)."},

  {"key": "shop_price_explosion", "type": "number", "label": "Prix — Renversement de Chips", "default": "0",
   "min": 0, "required": false, "group": "Prix des objets", "unit": "coins",
   "description": "0 = tarif catalogue (200)."},

  {"key": "shop_price_double_coup", "type": "number", "label": "Prix — Double Coussin", "default": "0",
   "min": 0, "required": false, "group": "Prix des objets", "unit": "coins",
   "description": "0 = tarif catalogue (250)."},

  {"key": "shop_price_surprise", "type": "number", "label": "Prix — Bataille d''Oreillers", "default": "0",
   "min": 0, "required": false, "group": "Prix des objets", "unit": "coins",
   "description": "0 = tarif catalogue (300)."},

  {"key": "shop_price_coup_traitre", "type": "number", "label": "Prix — Punaise dans le Coussin", "default": "0",
   "min": 0, "required": false, "group": "Prix des objets", "unit": "coins",
   "description": "0 = tarif catalogue (350)."},

  {"key": "shop_price_inversion", "type": "number", "label": "Prix — Retourne le Canape", "default": "0",
   "min": 0, "required": false, "group": "Prix des objets", "unit": "coins",
   "description": "0 = tarif catalogue (500)."},

  {"key": "combat_cooldown_minutes", "type": "number", "label": "Delai entre deux bagarres", "default": "0",
   "min": 0, "max": 10080, "required": false, "group": "Delais", "unit": "min",
   "description": "0 = aucune limite. Compte pour celui qui LANCE le defi."},

  {"key": "bet_cooldown_minutes", "type": "number", "label": "Delai entre deux paris", "default": "0",
   "min": 0, "max": 10080, "required": false, "group": "Delais", "unit": "min",
   "description": "0 = aucune limite."},

  {"key": "prime_cooldown_minutes", "type": "number", "label": "Delai entre deux contrats", "default": "0",
   "min": 0, "max": 10080, "required": false, "group": "Delais", "unit": "min",
   "description": "0 = aucune limite."},

  {"key": "class_change_cooldown_minutes", "type": "number", "label": "Delai entre deux changements de classe", "default": "0",
   "min": 0, "max": 10080, "required": false, "group": "Delais", "unit": "min",
   "description": "0 = aucune limite. Changer de classe remet les statistiques a celles de la nouvelle classe : sans delai, on peut en changer avant chaque bagarre.",
   "warning": "A zero, la classe cesse d''etre un choix et devient un bouton qu''on tourne selon l''adversaire."},

  {"key": "dice_faces", "type": "number", "label": "Faces du de", "default": "6",
   "min": 2, "max": 100, "required": false, "group": "Bagarres",
   "description": "Chaque manche multiplie les degats par le jet. Plus de faces = plus de hasard, et des ecarts plus violents."},

  {"key": "hp_base", "type": "number", "label": "Confort de base", "default": "100",
   "min": 1, "max": 10000, "required": false, "group": "Formules",
   "description": "Avant tout moelleux."},

  {"key": "hp_per_def", "type": "number", "label": "Confort par point de moelleux", "default": "10",
   "min": 0, "max": 1000, "required": false, "group": "Formules"},

  {"key": "damage_base", "type": "number", "label": "Degats de base", "default": "10",
   "min": 0, "max": 1000, "required": false, "group": "Formules",
   "description": "Degats = base + impact x facteur - moelleux adverse x facteur."},

  {"key": "damage_per_atk", "type": "number", "label": "Degats par point d''impact", "default": "4",
   "min": 0, "max": 100, "required": false, "group": "Formules"},

  {"key": "damage_per_def", "type": "number", "label": "Degats absorbes par point de moelleux", "default": "2",
   "min": 0, "max": 100, "required": false, "group": "Formules",
   "warning": "Au-dessus des degats par point d''impact, les bagarres n''avancent plus : tout le monde encaisse le minimum."},

  {"key": "ecraseur_atk", "type": "number", "label": "Ecraseur — impact de depart", "default": "25",
   "min": 0, "max": 1000, "required": false, "group": "Classe Ecraseur"},
  {"key": "ecraseur_def", "type": "number", "label": "Ecraseur — moelleux de depart", "default": "8",
   "min": 0, "max": 1000, "required": false, "group": "Classe Ecraseur"},
  {"key": "ecraseur_atk_growth", "type": "number", "label": "Ecraseur — impact par niveau", "default": "4",
   "min": 0, "max": 100, "required": false, "group": "Classe Ecraseur"},
  {"key": "ecraseur_def_growth", "type": "number", "label": "Ecraseur — moelleux par niveau", "default": "1",
   "min": 0, "max": 100, "required": false, "group": "Classe Ecraseur"},
  {"key": "ecraseur_damage_pct", "type": "number", "label": "Ecraseur — degats", "default": "125",
   "min": 10, "max": 500, "required": false, "group": "Classe Ecraseur", "unit": "%",
   "description": "125 = +25 % sur chaque coup."},
  {"key": "ecraseur_rage_threshold_pct", "type": "number", "label": "Ecraseur — seuil de rage", "default": "30",
   "min": 0, "max": 100, "required": false, "group": "Classe Ecraseur", "unit": "%",
   "description": "Part de Confort en dessous de laquelle il frappe plus fort. 0 = jamais."},
  {"key": "ecraseur_rage_bonus_pct", "type": "number", "label": "Ecraseur — degats enrage", "default": "125",
   "min": 100, "max": 500, "required": false, "group": "Classe Ecraseur", "unit": "%"},

  {"key": "ressort_atk", "type": "number", "label": "Ressort — impact de depart", "default": "12",
   "min": 0, "max": 1000, "required": false, "group": "Classe Ressort"},
  {"key": "ressort_def", "type": "number", "label": "Ressort — moelleux de depart", "default": "18",
   "min": 0, "max": 1000, "required": false, "group": "Classe Ressort"},
  {"key": "ressort_atk_growth", "type": "number", "label": "Ressort — impact par niveau", "default": "2",
   "min": 0, "max": 100, "required": false, "group": "Classe Ressort"},
  {"key": "ressort_def_growth", "type": "number", "label": "Ressort — moelleux par niveau", "default": "3",
   "min": 0, "max": 100, "required": false, "group": "Classe Ressort"},

  {"key": "piegeur_atk", "type": "number", "label": "Piegeur — impact de depart", "default": "18",
   "min": 0, "max": 1000, "required": false, "group": "Classe Piegeur"},
  {"key": "piegeur_def", "type": "number", "label": "Piegeur — moelleux de depart", "default": "14",
   "min": 0, "max": 1000, "required": false, "group": "Classe Piegeur"},
  {"key": "piegeur_atk_growth", "type": "number", "label": "Piegeur — impact par niveau", "default": "3",
   "min": 0, "max": 100, "required": false, "group": "Classe Piegeur"},
  {"key": "piegeur_def_growth", "type": "number", "label": "Piegeur — moelleux par niveau", "default": "2",
   "min": 0, "max": 100, "required": false, "group": "Classe Piegeur",
   "description": "Le bonus qui fait le Piegeur est sa chance de fouille, reglee dans le groupe Fouille."},

  {"key": "couette_atk", "type": "number", "label": "Couette — impact de depart", "default": "8",
   "min": 0, "max": 1000, "required": false, "group": "Classe Couette"},
  {"key": "couette_def", "type": "number", "label": "Couette — moelleux de depart", "default": "25",
   "min": 0, "max": 1000, "required": false, "group": "Classe Couette"},
  {"key": "couette_atk_growth", "type": "number", "label": "Couette — impact par niveau", "default": "1",
   "min": 0, "max": 100, "required": false, "group": "Classe Couette"},
  {"key": "couette_def_growth", "type": "number", "label": "Couette — moelleux par niveau", "default": "4",
   "min": 0, "max": 100, "required": false, "group": "Classe Couette"},
  {"key": "couette_hp_pct", "type": "number", "label": "Couette — Confort maximum", "default": "130",
   "min": 50, "max": 500, "required": false, "group": "Classe Couette", "unit": "%",
   "description": "130 = +30 % de Confort par rapport aux autres classes."},
  {"key": "couette_damage_taken_pct", "type": "number", "label": "Couette — degats subis", "default": "80",
   "min": 10, "max": 200, "required": false, "group": "Classe Couette", "unit": "%",
   "description": "80 = encaisse 20 % de moins.",
   "warning": "En dessous de 50 %, plus rien ne la fait lever du canape."},
  {"key": "couette_flat_reduction", "type": "number", "label": "Couette — degats retires par coup", "default": "5",
   "min": 0, "max": 100, "required": false, "group": "Classe Couette",
   "description": "Retire apres le pourcentage. Un coup fait toujours au moins 1."}
]'::jsonb
WHERE bot_name = 'nexus-coussin';
