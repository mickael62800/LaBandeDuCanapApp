-- 053_ports_additionnels_catalogue.sql
--
-- Sort les ports additionnels des jeux hors du code Rust.
--
-- Valheim n'ecoute pas sur un port mais sur trois : 2456 (jeu), 2457 (requete
-- Steam) et 2458 (communication). Cette particularite etait ecrite DEUX FOIS
-- dans le code, sous la forme `if template.slug == "valheim"` : une fois pour
-- reserver un bloc de trois ports hote, une fois pour publier les mappings.
-- Ajouter V Rising, Project Zomboid ou Vintage Story — qui ont tous le meme
-- besoin — imposait donc un deploiement applicatif pour une propriete qui
-- appartient a l'IMAGE, c'est-a-dire au catalogue.
--
-- Le decalage (`offset`) est relatif au port principal et vaut des deux cotes :
-- l'allocateur reserve un bloc contigu, donc `host_port + offset` est libre par
-- construction. Un decalage nul est licite et sert a doubler le port principal
-- dans l'autre protocole (Vintage Story ecoute en TCP ET en UDP sur 42420).
--
-- Defaut vide : un jeu qui ne declare rien garde exactement le comportement
-- actuel, un seul port publie.

ALTER TABLE game_templates
    ADD COLUMN IF NOT EXISTS extra_ports JSONB NOT NULL DEFAULT '[]'::jsonb;

COMMENT ON COLUMN game_templates.extra_ports IS
    'Ports additionnels exiges par l''image, en decalage du port principal : [{"offset": 1, "protocol": "udp"}]. Le decalage vaut cote conteneur ET cote hote ; l''allocateur reserve un bloc contigu de cette largeur.';

-- Valheim quitte le code pour les donnees. Les valeurs reproduisent a
-- l'identique ce que faisait `provisioning.rs` : +1 et +2 en UDP. Aucun
-- serveur existant ne change de port — les ports deja alloues sont persistes
-- sur `game_servers` et reutilises tels quels au redemarrage.
UPDATE game_templates
SET extra_ports = '[{"offset": 1, "protocol": "udp"}, {"offset": 2, "protocol": "udp"}]'::jsonb,
    updated_at = now()
WHERE slug = 'valheim';
