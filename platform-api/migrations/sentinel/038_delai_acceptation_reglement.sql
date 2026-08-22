-- 038_delai_acceptation_reglement.sql
--
-- Delai d'acceptation du reglement pour les arrivants ORDINAIRES.
--
-- ── Ce qui manquait ──
--
-- Le module Accueil posait le reglement et son bouton, puis n'attendait rien :
-- un membre pouvait rester indefiniment sans avoir clique, sans relance et sans
-- fin. Le seul mecanisme d'expulsion apres delai vivait dans la QUARANTAINE,
-- qui ne se declenche que sur suspicion (raid, compte trop jeune, alt d'un
-- banni) et ne voit donc jamais un arrivant normal. Ses libelles parlaient
-- pourtant d'« accepter le reglement » (corrige en migration 037), ce qui
-- laissait croire que le systeme existait.
--
-- ── Pourquoi une table separee de la quarantaine ──
--
-- Les deux files n'ont ni le meme rythme, ni la meme population, ni la meme
-- issue. Un raid se traite en secondes et se solde par une expulsion massive ;
-- quelqu'un qui tarde a cliquer merite des jours et une relance. Les melanger
-- aurait fait qu'un reglage de securite deplace l'echeance d'un membre
-- legitime, ou l'inverse.
--
-- ── L'echeance est FIGEE a l'arrivee ──
--
-- `expires_at` est une date, pas une duree recalculee a chaque passage.
-- Rallonger le delai depuis le tableau de bord ne doit pas raccourcir le
-- sursis de quelqu'un qui attend deja, et le raccourcir ne doit pas expulser
-- d'un coup toute la file.

CREATE TABLE IF NOT EXISTS welcome_rules_pending (
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    -- Nulle tant qu'aucune relance n'est partie. Posee AVANT la publication de
    -- l'evenement, sous garde `IS NULL` : un balayage regulier n'envoie pas un
    -- message prive a chaque passage, et deux instances ne relancent pas deux
    -- fois le meme membre.
    reminded_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);

-- Les deux jobs cherchent « ce qui est du », jamais un membre precis.
CREATE INDEX IF NOT EXISTS idx_welcome_rules_pending_expires
    ON welcome_rules_pending (expires_at);
CREATE INDEX IF NOT EXISTS idx_welcome_rules_pending_relance
    ON welcome_rules_pending (expires_at)
    WHERE reminded_at IS NULL;

COMMENT ON TABLE welcome_rules_pending IS
    'Arrivants ORDINAIRES qui n''ont pas encore accepte le reglement. Distinct de security_quarantine_pending, qui ne suit que les comptes juges suspects.';
COMMENT ON COLUMN welcome_rules_pending.expires_at IS
    'Echeance figee a l''arrivee : changer le delai ne bouge pas le sursis de ceux qui attendent deja.';

-- ── Reglages, dans le module Accueil et non dans la Securite ──
--
-- `jsonb_path_exists` protege le rejeu : une cle deja presente n'est pas
-- ajoutee une seconde fois.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "rules_deadline_enabled", "type": "boolean",
   "label": "Delai pour accepter le reglement",
   "default": "false", "required": false,
   "depends_on": {"key": "rules_enabled", "equals": "true"},
   "description": "Donne un delai aux nouveaux arrivants pour cliquer sur le bouton du reglement, avec relance puis expulsion. Concerne TOUS les arrivants, pas seulement les comptes suspects.",
   "warning": "Desactive, rien ne se passe : les membres qui ne cliquent jamais restent indefiniment."},

  {"key": "rules_deadline_secs", "type": "number", "unit": "secondes",
   "label": "Delai laisse pour accepter (secondes)",
   "default": "259200", "min": 3600, "max": 2592000, "required": false,
   "depends_on": {"key": "rules_deadline_enabled", "equals": "true"},
   "description": "259200 = 3 jours, 604800 = 7 jours. L''echeance est figee a l''arrivee : la changer ne touche pas ceux qui attendent deja.",
   "warning": "Ce n''est pas un dispositif anti-raid : rien ne presse. Un delai de quelques heures expulse ceux qui rejoignent le soir et ne rouvrent Discord que le lendemain."},

  {"key": "rules_reminder_secs", "type": "number", "unit": "secondes",
   "label": "Relance avant expulsion (secondes avant l''echeance)",
   "default": "86400", "min": 0, "max": 2592000, "required": false,
   "depends_on": {"key": "rules_deadline_enabled", "equals": "true"},
   "description": "Message prive rappelant d''accepter le reglement, envoye ce nombre de secondes AVANT l''echeance. 86400 = un jour avant. 0 desactive la relance.",
   "warning": "Une valeur superieure au delai ferait partir la relance en meme temps que le message d''accueil : deux messages dans la meme seconde, dont un qui menace d''expulsion."},

  {"key": "rules_kick_enabled", "type": "boolean",
   "label": "Expulser a l''expiration du delai",
   "default": "true", "required": false,
   "depends_on": {"key": "rules_deadline_enabled", "equals": "true"},
   "description": "Si desactive, le membre reste sans avoir accepte : le delai ne sert plus qu''a relancer.",
   "warning": "Une expulsion n''est pas un bannissement : la personne peut revenir avec une nouvelle invitation."}
]'::jsonb
WHERE bot_name = 'welcome-bot'
  AND NOT jsonb_path_exists(config_schema, '$[*] ? (@.key == "rules_deadline_enabled")');
