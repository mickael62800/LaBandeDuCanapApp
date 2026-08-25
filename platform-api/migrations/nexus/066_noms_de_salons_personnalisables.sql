-- 066_noms_de_salons_personnalisables.sql
--
-- Les trois salons d'une session de jeu portaient des noms figes dans le code
-- du bot : « inscription-{slug} », « salon-{slug} » et « Vocal {serveur} ».
-- Aucun moyen de les adapter au vocabulaire d'une communaute.
--
-- DEUX NIVEAUX, VOULUS. Un MODELE par guilde couvre le cas courant — on decide
-- une fois, tous les jeux suivent, y compris ceux qui n'existent pas encore.
-- Un NOM LIBRE par serveur couvre l'exception, quand un jeu merite son propre
-- nom. Le serveur l'emporte sur la guilde, la guilde sur le defaut.
--
-- COLONNES NULLABLES, ET NON CHAINES VIDES. `NULL` dit « rien de choisi ici,
-- demande au niveau au-dessus » ; une chaine vide dirait « ce salon s'appelle
-- rien du tout », ce que Discord refuse. La distinction porte tout le
-- mecanisme de repli, elle ne doit pas se perdre en base.

ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS channel_name_registration TEXT,
    ADD COLUMN IF NOT EXISTS channel_name_private      TEXT,
    ADD COLUMN IF NOT EXISTS channel_name_voice        TEXT;

COMMENT ON COLUMN game_servers.channel_name_registration IS
    'Nom libre du salon d''inscription. NULL = utiliser le modele de la guilde.';
COMMENT ON COLUMN game_servers.channel_name_private IS
    'Nom libre du salon prive des inscrits. NULL = utiliser le modele de la guilde.';
COMMENT ON COLUMN game_servers.channel_name_voice IS
    'Nom libre du salon vocal. NULL = utiliser le modele de la guilde.';

-- Les trois modeles de guilde, dans la configuration du module game-portal.
--
-- La concatenation de tableaux ne fusionne pas par cle (c'est ce qui avait
-- produit les doublons corriges par la migration 064) : on ne concatene donc
-- que les cles reellement absentes, et la migration reste sans effet si elle
-- est rejouee.
UPDATE bot_definitions
SET config_schema = config_schema || jsonb_build_array(
    jsonb_build_object(
        'key', 'channel_name_registration_template',
        'type', 'text',
        'label', 'Modele du salon d''inscription',
        'default', 'inscription-{jeu}',
        'required', false,
        'depends_on', jsonb_build_object('key', 'enabled', 'equals', 'true'),
        'description', 'Reperes disponibles : {jeu} et {serveur}. Discord met les salons ecrits en minuscules et remplace les espaces par des tirets.'
    ),
    jsonb_build_object(
        'key', 'channel_name_private_template',
        'type', 'text',
        'label', 'Modele du salon prive des inscrits',
        'default', 'salon-{jeu}',
        'required', false,
        'depends_on', jsonb_build_object('key', 'enabled', 'equals', 'true'),
        'description', 'Reperes disponibles : {jeu} et {serveur}.'
    ),
    jsonb_build_object(
        'key', 'channel_name_voice_template',
        'type', 'text',
        'label', 'Modele du salon vocal',
        'default', 'Vocal {serveur}',
        'required', false,
        'depends_on', jsonb_build_object('key', 'enabled', 'equals', 'true'),
        'description', 'Reperes disponibles : {jeu} et {serveur}. Un salon vocal accepte majuscules, espaces et emoji.'
    )
)
WHERE bot_name = 'game-portal'
  AND NOT EXISTS (
      SELECT 1
      FROM jsonb_array_elements(config_schema) AS champ
      WHERE champ ->> 'key' = 'channel_name_registration_template'
  );
