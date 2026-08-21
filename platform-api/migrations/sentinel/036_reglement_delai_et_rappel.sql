-- 036_reglement_delai_et_rappel.sql
--
-- Le delai laisse a un nouveau membre pour accepter le reglement devient un
-- reglage du SERVEUR, et un rappel en message prive part avant l'expulsion.
--
-- ETAT PRECEDENT. Le delai valait `CAPTCHA_TIMEOUT_SECS`, une variable
-- d'environnement globale a 300 secondes par defaut. Cinq minutes pour
-- quelqu'un qui rejoint depuis son telephone, ou dont les messages prives sont
-- fermes et qui doit d'abord les rouvrir : l'expulsion tombait avant que la
-- personne ait vu le message. La charte du depot veut d'ailleurs qu'un reglage
-- lie a une guilde vive ici, pas dans l'environnement.
--
-- Le delai continue d'etre calcule a l'ARRIVEE du membre et fige dans
-- `expires_at` : changer le reglage ne raccourcit donc jamais le sursis de
-- quelqu'un deja en attente. Il ne vaut que pour les arrivees suivantes.
--
-- LE RAPPEL. Une seule ligne suffit a le suivre : `reminded_at`. Nulle tant
-- qu'aucun rappel n'est parti, elle rend le job idempotent — sans elle, un
-- balayage toutes les quinze secondes enverrait un message prive toutes les
-- quinze secondes.

ALTER TABLE security_quarantine_pending
    ADD COLUMN IF NOT EXISTS reminded_at TIMESTAMPTZ;

COMMENT ON COLUMN security_quarantine_pending.reminded_at IS
    'Date d''envoi du rappel avant expulsion. NULL = pas encore rappele. Sert de garde d''idempotence au job remind-quarantine-rules.';

-- Retrouver les rappels a envoyer, c'est chercher les lignes non rappelees
-- dont l'echeance approche. L'index existant porte sur `expires_at` seul et
-- ramenait donc aussi toutes les lignes deja rappelees.
CREATE INDEX IF NOT EXISTS idx_security_quarantine_a_rappeler
    ON security_quarantine_pending (expires_at)
    WHERE reminded_at IS NULL;


-- ─────────────────────────────────────────────────────────────────────
-- Les reglages, cote serveur
-- ─────────────────────────────────────────────────────────────────────
--
-- Ajoutes a la fin du schema du module de securite, sans toucher aux reglages
-- existants ni a leur ordre : le formulaire du tableau de bord les affiche
-- dans l'ordre du tableau.
--
-- `jsonb_path_exists` protege le rejeu : une cle deja presente n'est pas
-- ajoutee une seconde fois.

UPDATE bot_definitions
SET config_schema = config_schema || '[
  {"key": "quarantine_timeout_secs", "type": "number", "unit": "secondes",
   "label": "Delai pour accepter le reglement (secondes)",
   "default": "86400", "min": 60, "max": 2592000, "required": false,
   "depends_on": {"key": "quarantine_enabled", "equals": "true"},
   "description": "Temps laisse a un nouveau membre pour se verifier avant expulsion. 86400 = 24 heures, 604800 = 7 jours.",
   "warning": "Pendant tout ce delai, la personne reste en acces restreint. Un delai tres court expulse des membres legitimes qui n''ont pas eu le temps de voir le message prive."},

  {"key": "quarantine_kick_enabled", "type": "boolean",
   "label": "Expulser a l''expiration du delai",
   "default": "true", "required": false,
   "depends_on": {"key": "quarantine_enabled", "equals": "true"},
   "description": "Si desactive, le membre reste en quarantaine indefiniment et attend une decision humaine.",
   "warning": "Desactiver laisse s''accumuler des comptes en attente, qui gardent l''acces restreint sans limite de temps."},

  {"key": "quarantine_reminder_secs", "type": "number", "unit": "secondes",
   "label": "Rappel avant expulsion (secondes avant l''echeance)",
   "default": "3600", "min": 0, "max": 2592000, "required": false,
   "depends_on": {"key": "quarantine_enabled", "equals": "true"},
   "description": "Envoie un message prive rappelant d''accepter le reglement, ce nombre de secondes AVANT l''expulsion. 3600 = une heure avant. 0 desactive le rappel.",
   "warning": "Une valeur superieure au delai d''acceptation ferait partir le rappel immediatement, en meme temps que le premier message."},

  {"key": "rules_channel_id", "type": "channel",
   "label": "Salon du reglement",
   "required": false,
   "depends_on": {"key": "quarantine_enabled", "equals": "true"},
   "description": "Cite dans le message prive de rappel pour indiquer ou lire le reglement. Vide : le message reste general."}
]'::jsonb
WHERE bot_name = 'security-bot'
  AND NOT jsonb_path_exists(config_schema, '$[*] ? (@.key == "quarantine_timeout_secs")');
