-- 014_economy_coude_config.sql
--
-- Configuration de l'economie et de Coup de Coude.
--
-- Ces deux domaines n'avaient AUCUN reglage : taux de vol, cooldowns, gains,
-- cout des objets — tout etait en dur dans le code. Regler l'equilibre d'un
-- jeu demandait donc de recompiler, ce qui revient a ne jamais le regler.
--
-- Le formulaire de la page Configuration est entierement pilote par ce
-- schema : ajouter une ligne ici la fait apparaitre a l'ecran, sans toucher
-- au front.
--
-- Les valeurs par defaut reproduisent EXACTEMENT le comportement actuel.
-- Une installation qui ne touche a rien ne voit aucun changement — c'est la
-- condition pour livrer un tel systeme sans surprise.

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema) VALUES
('nexus-economy', 'Economie', 'Porte-monnaie partage, Roue du Destin, transferts entre membres.', '[
  {"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false,
   "group": "General",
   "description": "Si desactive, aucun gain ni transfert n''est possible."},

  {"key": "starting_coins", "type": "number", "label": "Solde de depart", "default": "100",
   "min": 0, "max": 1000000, "required": false, "group": "General", "unit": "coins",
   "description": "Credite au premier passage d''un membre. Ne s''applique pas retroactivement."},

  {"key": "transfer_enabled", "type": "boolean", "label": "Transferts entre membres", "default": "true",
   "required": false, "group": "Transferts"},

  {"key": "transfer_min", "type": "number", "label": "Montant minimum d''un transfert", "default": "1",
   "min": 1, "required": false, "group": "Transferts", "unit": "coins"},

  {"key": "transfer_max", "type": "number", "label": "Montant maximum d''un transfert", "default": "0",
   "min": 0, "required": false, "group": "Transferts", "unit": "coins",
   "description": "0 = pas de plafond."},

  {"key": "transfer_fee_pct", "type": "number", "label": "Frais preleves sur un transfert", "default": "0",
   "min": 0, "max": 50, "required": false, "group": "Transferts", "unit": "%",
   "description": "Retire de la circulation. Un petit pourcentage freine l''inflation sur la duree.",
   "warning": "Au-dela de 10 %, les membres cessent simplement de s''echanger des coins."},

  {"key": "wheel_enabled", "type": "boolean", "label": "Roue du Destin active", "default": "true",
   "required": false, "group": "Roue du Destin"},

  {"key": "wheel_cooldown_hours", "type": "number", "label": "Delai entre deux tirages", "default": "24",
   "min": 1, "max": 168, "required": false, "group": "Roue du Destin", "unit": "h",
   "description": "24 = un tirage par jour."},

  {"key": "wheel_payout_multiplier", "type": "number", "label": "Multiplicateur des gains", "default": "100",
   "min": 10, "max": 1000, "required": false, "group": "Roue du Destin", "unit": "%",
   "description": "Applique aux gains ET aux pertes. 200 double les deux.",
   "warning": "Au-dela de 300, une seule licorne peut desequilibrer tout le classement."},

  {"key": "leaderboard_size", "type": "number", "label": "Taille du classement", "default": "10",
   "min": 3, "max": 50, "required": false, "group": "Affichage"}
]'::jsonb)
ON CONFLICT (bot_name) DO NOTHING;

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema) VALUES
('nexus-coude', 'Coup de Coude', 'Combats, vols, primes et paris entre membres.', '[
  {"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false,
   "group": "General"},

  {"key": "max_level", "type": "number", "label": "Niveau maximum", "default": "25",
   "min": 5, "max": 200, "required": false, "group": "General"},

  {"key": "stat_points_per_level", "type": "number", "label": "Points gagnes par niveau", "default": "1",
   "min": 1, "max": 10, "required": false, "group": "Progression"},

  {"key": "xp_winner", "type": "number", "label": "Experience du vainqueur", "default": "10",
   "min": 0, "max": 1000, "required": false, "group": "Progression"},

  {"key": "xp_loser", "type": "number", "label": "Experience du perdant", "default": "3",
   "min": 0, "max": 1000, "required": false, "group": "Progression",
   "description": "Un perdant qui gagne un peu d''experience continue de jouer. A zero, perdre devient punitif."},

  {"key": "xp_underdog_bonus", "type": "number", "label": "Bonus contre plus fort que soi", "default": "5",
   "min": 0, "max": 500, "required": false, "group": "Progression",
   "description": "Ajoute a l''experience quand on bat quelqu''un de plusieurs niveaux au-dessus."},

  {"key": "combat_cooldown_minutes", "type": "number", "label": "Delai entre deux combats", "default": "0",
   "min": 0, "max": 1440, "required": false, "group": "Combats", "unit": "min",
   "description": "0 = aucune limite."},

  {"key": "combat_mise_min", "type": "number", "label": "Mise minimum", "default": "10",
   "min": 1, "required": false, "group": "Combats", "unit": "coins"},

  {"key": "combat_mise_max", "type": "number", "label": "Mise maximum", "default": "0",
   "min": 0, "required": false, "group": "Combats", "unit": "coins",
   "description": "0 = pas de plafond.",
   "warning": "Sans plafond, deux gros joueurs peuvent se transferer des fortunes en un combat."},

  {"key": "steal_enabled", "type": "boolean", "label": "Vols autorises", "default": "true",
   "required": false, "group": "Vols",
   "warning": "Le vol est ce qui cree le plus de tensions dans une communaute. A desactiver au premier probleme."},

  {"key": "steal_success_pct", "type": "number", "label": "Chance de reussite d''un vol", "default": "30",
   "min": 1, "max": 99, "required": false, "group": "Vols", "unit": "%"},

  {"key": "steal_success_pct_fourbe", "type": "number", "label": "Chance de reussite — classe Fourbe", "default": "50",
   "min": 1, "max": 99, "required": false, "group": "Vols", "unit": "%",
   "description": "Le bonus qui donne son interet a la classe Fourbe."},

  {"key": "steal_gain_pct", "type": "number", "label": "Part du solde de la victime volee", "default": "20",
   "min": 1, "max": 100, "required": false, "group": "Vols", "unit": "%",
   "warning": "Au-dela de 30 %, un seul vol peut ruiner quelqu''un et le degouter du jeu."},

  {"key": "steal_penalty_pct", "type": "number", "label": "Part perdue en cas d''echec", "default": "15",
   "min": 0, "max": 100, "required": false, "group": "Vols", "unit": "%",
   "description": "Prelevee sur le VOLEUR et versee a sa cible. C''est ce qui rend le vol risque."},

  {"key": "steal_cooldown_minutes", "type": "number", "label": "Delai entre deux vols", "default": "30",
   "min": 0, "max": 1440, "required": false, "group": "Vols", "unit": "min"},

  {"key": "steal_min_victim_coins", "type": "number", "label": "Solde minimum d''une cible", "default": "10",
   "min": 0, "required": false, "group": "Vols", "unit": "coins",
   "description": "Protege les plus pauvres : en dessous, on ne peut pas etre vole."},

  {"key": "prime_enabled", "type": "boolean", "label": "Primes autorisees", "default": "true",
   "required": false, "group": "Primes"},

  {"key": "prime_min", "type": "number", "label": "Prime minimum", "default": "50",
   "min": 1, "required": false, "group": "Primes", "unit": "coins"},

  {"key": "prime_max", "type": "number", "label": "Prime maximum", "default": "0",
   "min": 0, "required": false, "group": "Primes", "unit": "coins",
   "description": "0 = pas de plafond."},

  {"key": "bet_enabled", "type": "boolean", "label": "Paris autorises", "default": "true",
   "required": false, "group": "Paris"},

  {"key": "bet_min", "type": "number", "label": "Pari minimum", "default": "10",
   "min": 1, "required": false, "group": "Paris", "unit": "coins"},

  {"key": "bet_payout_multiplier", "type": "number", "label": "Gain d''un pari gagnant", "default": "200",
   "min": 100, "max": 1000, "required": false, "group": "Paris", "unit": "%",
   "description": "200 = la mise est doublee.",
   "warning": "Au-dela de 200 %, parier rapporte plus que combattre et le jeu se vide de ses combats."},

  {"key": "insurance_enabled", "type": "boolean", "label": "Assurance disponible", "default": "true",
   "required": false, "group": "Boutique"},

  {"key": "insurance_cost", "type": "number", "label": "Prix de l''assurance", "default": "100",
   "min": 1, "required": false, "group": "Boutique", "unit": "coins"},

  {"key": "hp_regen_per_hour", "type": "number", "label": "Points de vie regeneres par heure", "default": "5",
   "min": 0, "max": 1000, "required": false, "group": "Progression",
   "description": "0 = aucune regeneration automatique."}
]'::jsonb)
ON CONFLICT (bot_name) DO NOTHING;
