-- 021_coussin_piege.sql
--
-- « Coup de Coude » devient « Coussin Piégé ».
--
-- Le jeu ne change pas de regles : on renomme ce qui se lit. L'ancien titre
-- ne racontait rien de la bande ; le nouveau tient dans une image que tout le
-- monde comprend — un coussin planque sur le canape, et celui qui s'assoit
-- dessus.
--
-- RENOMMAGE, pas recreation : les tables sont renommees et les valeurs de
-- classe traduites en place. Personne ne perd son personnage, son inventaire
-- ni son palmares. C'est la seule maniere honnete de rebaptiser un jeu deja
-- joue.

-- ── Tables ──

ALTER TABLE IF EXISTS nexus_coude_players     RENAME TO nexus_coussin_players;
ALTER TABLE IF EXISTS nexus_coude_combats     RENAME TO nexus_coussin_combats;
ALTER TABLE IF EXISTS nexus_coude_inventory   RENAME TO nexus_coussin_inventory;
ALTER TABLE IF EXISTS nexus_coude_bets        RENAME TO nexus_coussin_bets;
ALTER TABLE IF EXISTS nexus_coude_primes      RENAME TO nexus_coussin_primes;
ALTER TABLE IF EXISTS nexus_coude_insurances  RENAME TO nexus_coussin_insurances;
ALTER TABLE IF EXISTS nexus_coude_cooldowns   RENAME TO nexus_coussin_cooldowns;
ALTER TABLE IF EXISTS nexus_coude_events      RENAME TO nexus_coussin_events;

ALTER INDEX IF EXISTS idx_nexus_coude_pending          RENAME TO idx_nexus_coussin_pending;
ALTER INDEX IF EXISTS idx_nexus_coude_players_level    RENAME TO idx_nexus_coussin_players_level;
ALTER INDEX IF EXISTS idx_nexus_coude_bets_combat      RENAME TO idx_nexus_coussin_bets_combat;
ALTER INDEX IF EXISTS idx_nexus_coude_primes_target    RENAME TO idx_nexus_coussin_primes_target;
ALTER INDEX IF EXISTS idx_nexus_coude_insurances_active RENAME TO idx_nexus_coussin_insurances_active;

-- ── Classes ──
--
-- Les quatre archetypes deviennent quatre manieres d'occuper un canape. Les
-- statistiques associees sont inchangees : c'est une traduction, pas un
-- reequilibrage.
--
-- La contrainte est retiree AVANT la traduction : sinon la premiere ligne
-- mise a jour violerait l'ancienne liste de valeurs.

ALTER TABLE nexus_coussin_players DROP CONSTRAINT IF EXISTS nexus_coude_players_class_check;

UPDATE nexus_coussin_players SET class = CASE class
    WHEN 'bourrin' THEN 'ecraseur'
    WHEN 'agile'   THEN 'ressort'
    WHEN 'fourbe'  THEN 'piegeur'
    WHEN 'tank'    THEN 'couette'
    ELSE class
END;

ALTER TABLE nexus_coussin_players ALTER COLUMN class SET DEFAULT 'ecraseur';
ALTER TABLE nexus_coussin_players ADD CONSTRAINT nexus_coussin_players_class_check
    CHECK (class IN ('ecraseur', 'ressort', 'piegeur', 'couette'));

-- ── Titres ──
--
-- Les grades militaires laissent place a la place occupee sur le canape. Le
-- titre est recalcule a chaque affichage par le domaine ; on met la colonne a
-- jour pour que les profils jamais rouverts n'affichent pas l'ancien mot.

UPDATE nexus_coussin_players SET title = CASE
    WHEN level BETWEEN 1  AND 4  THEN 'Bout d''Accoudoir'
    WHEN level BETWEEN 5  AND 9  THEN 'Squatteur'
    WHEN level BETWEEN 10 AND 14 THEN 'Poseur de Coussins'
    WHEN level BETWEEN 15 AND 19 THEN 'Gardien de la Telecommande'
    WHEN level BETWEEN 20 AND 24 THEN 'Roi du Canape'
    ELSE 'Le Canape, c''est Lui'
END;

ALTER TABLE nexus_coussin_players ALTER COLUMN title SET DEFAULT 'Bout d''Accoudoir';

-- ── Configuration ──
--
-- Le bot change de nom : les valeurs deja reglees par chaque serveur suivent,
-- sinon un administrateur retrouverait tous ses curseurs remis a zero.

UPDATE bot_definitions  SET bot_name = 'nexus-coussin' WHERE bot_name = 'nexus-coude';
UPDATE bot_guild_config SET bot_name = 'nexus-coussin' WHERE bot_name = 'nexus-coude';

UPDATE bot_guild_config SET config_key = 'steal_success_pct_piegeur'
WHERE bot_name = 'nexus-coussin' AND config_key = 'steal_success_pct_fourbe';

-- Le schema est reecrit en entier plutot que rustine par rustine : les cles
-- sont exactement celles que `CoussinConfig` sait lire, et les libelles
-- parlent enfin la langue du jeu. Toute cle absente ici serait un curseur
-- sans effet — c'est precisement ce que la migration 016 avait nettoye.
UPDATE bot_definitions SET
    display_name = 'Coussin Piégé',
    description  = 'Bagarres de coussins, fouille sous les coussins, contrats et paris entre membres.',
    config_schema = '[
  {"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false,
   "group": "General"},

  {"key": "steal_enabled", "type": "boolean", "label": "Fouille sous les coussins autorisee", "default": "true",
   "required": false, "group": "Fouille",
   "warning": "Se servir chez les autres est ce qui cree le plus de tensions. A desactiver au premier probleme."},

  {"key": "steal_success_pct", "type": "number", "label": "Chance de trouver quelque chose", "default": "30",
   "min": 1, "max": 99, "required": false, "group": "Fouille", "unit": "%"},

  {"key": "steal_success_pct_piegeur", "type": "number", "label": "Chance de reussite — classe Piegeur", "default": "50",
   "min": 1, "max": 99, "required": false, "group": "Fouille", "unit": "%",
   "description": "Le bonus qui donne son interet au Piegeur : il sait ou les autres planquent."},

  {"key": "steal_gain_pct", "type": "number", "label": "Part du solde trouvee chez la cible", "default": "20",
   "min": 1, "max": 100, "required": false, "group": "Fouille", "unit": "%",
   "warning": "Au-dela de 30 %, une seule fouille peut ruiner quelqu''un et le degouter du jeu."},

  {"key": "steal_penalty_pct", "type": "number", "label": "Part perdue si on se fait prendre", "default": "15",
   "min": 0, "max": 100, "required": false, "group": "Fouille", "unit": "%",
   "description": "Prelevee sur le fouilleur et versee a sa cible. C''est ce qui rend la fouille risquee."},

  {"key": "steal_cooldown_minutes", "type": "number", "label": "Delai entre deux fouilles", "default": "30",
   "min": 0, "max": 1440, "required": false, "group": "Fouille", "unit": "min"},

  {"key": "steal_min_victim_coins", "type": "number", "label": "Solde minimum d''une cible", "default": "10",
   "min": 0, "required": false, "group": "Fouille", "unit": "coins",
   "description": "Protege les plus pauvres : en dessous, on ne trouve rien sous leur coussin."},

  {"key": "prime_enabled", "type": "boolean", "label": "Contrats autorises", "default": "true",
   "required": false, "group": "Contrats",
   "description": "Promettre une recompense a qui fera lever quelqu''un du canape."},

  {"key": "prime_min", "type": "number", "label": "Contrat minimum", "default": "50",
   "min": 1, "required": false, "group": "Contrats", "unit": "coins"},

  {"key": "prime_max", "type": "number", "label": "Contrat maximum", "default": "0",
   "min": 0, "required": false, "group": "Contrats", "unit": "coins",
   "description": "0 = pas de plafond."},

  {"key": "bet_enabled", "type": "boolean", "label": "Paris autorises", "default": "true",
   "required": false, "group": "Paris"},

  {"key": "bet_min", "type": "number", "label": "Pari minimum", "default": "10",
   "min": 1, "required": false, "group": "Paris", "unit": "coins"},

  {"key": "insurance_enabled", "type": "boolean", "label": "Garantie anti-tache disponible", "default": "true",
   "required": false, "group": "Coffre a coussins"},

  {"key": "insurance_cost", "type": "number", "label": "Prix de la garantie", "default": "50",
   "min": 1, "required": false, "group": "Coffre a coussins", "unit": "coins"}
]'::jsonb
WHERE bot_name = 'nexus-coussin';
