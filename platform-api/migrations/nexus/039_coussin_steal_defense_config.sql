-- 039_coussin_steal_defense_config.sql
--
-- Reglages de la fenetre de defense de la fouille.
--
-- La fouille se jouait sur un pourcentage fixe : la cible n'avait aucune prise
-- dessus, et perdre sept fois sur dix ressemblait a une taxe plutot qu'a un
-- jeu. Elle se joue maintenant en deux jets opposes, avec une fenetre pendant
-- laquelle la victime peut serrer les coussins.
--
-- Les deux reglages qui decident de l'equilibre du systeme appartiennent donc
-- au serveur, pas au code : combien de temps on laisse pour reagir, et ce que
-- coute de n'avoir rien fait.
--
-- Les anciennes cles `steal_success_pct` et `steal_success_pct_piegeur` sont
-- retirees : elles ne sont plus lues par personne. Un curseur qui ne fait
-- plus rien est pire que son absence — on croit le probleme regle et on
-- cherche ailleurs (c'est exactement ce que la migration 016 avait nettoye).

UPDATE bot_definitions SET config_schema = (
    SELECT jsonb_agg(elem ORDER BY ord)
    FROM jsonb_array_elements(config_schema) WITH ORDINALITY AS t(elem, ord)
    WHERE elem ->> 'key' NOT IN ('steal_success_pct', 'steal_success_pct_piegeur')
)
WHERE bot_name = 'nexus-coussin';

DELETE FROM bot_guild_config
WHERE bot_name = 'nexus-coussin'
  AND config_key IN ('steal_success_pct', 'steal_success_pct_piegeur');

UPDATE bot_definitions SET config_schema = config_schema || '[
  {"key": "steal_defense_window_seconds", "type": "number", "label": "Temps pour reagir",
   "default": "60", "min": 10, "max": 600, "required": false, "group": "Fouille", "unit": "s",
   "description": "Delai laisse a la cible pour serrer les coussins avant que la fouille ne se resolve.",
   "warning": "Trop court, personne n''a le temps de voir la notification et la defense ne sert a rien."},

  {"key": "steal_absence_malus", "type": "number", "label": "Malus de vigilance",
   "default": "8", "min": 0, "max": 20, "required": false, "group": "Fouille",
   "description": "Retire a la defense de la cible qui n''a pas reagi. C''est ce reglage qui decide si etre attentif paie.",
   "warning": "A 0, reagir ne change plus rien et le bouton devient decoratif."}
]'::jsonb
WHERE bot_name = 'nexus-coussin';
