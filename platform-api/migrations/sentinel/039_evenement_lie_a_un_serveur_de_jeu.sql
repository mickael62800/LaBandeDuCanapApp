-- 039_evenement_lie_a_un_serveur_de_jeu.sql
--
-- Rattache un evenement du calendrier au serveur de jeu qui l'a fait naitre.
--
-- LE PROBLEME. Creer un serveur de jeu inscrit une soiree au calendrier
-- communautaire. Mais rien ne reliait les deux : supprimer le serveur laissait
-- l'evenement en place, visible sur le site public pendant des semaines. Une
-- session Terraria supprimee le 21 aout s'annoncait encore « jusqu'au
-- 21 septembre », et aucune requete ne permettait de reperer l'orphelin
-- autrement qu'en lisant les titres a l'oeil.
--
-- POURQUOI PAS UNE CLE ETRANGERE. `game_servers` vit dans la base `nexus`,
-- `community_events` dans `discord_sentinel` : PostgreSQL ne peut pas poser de
-- contrainte entre deux bases. Le lien est donc declaratif — un identifiant
-- que l'on renseigne et sur lequel on sait requeter, pas une integrite garantie
-- par le moteur. C'est la limite de la separation par domaine, assumee.
--
-- Colonne NULLABLE : la plupart des evenements n'ont aucun rapport avec un
-- serveur de jeu (soirees, annonces, tournois sur un jeu tiers). Seuls ceux
-- crees depuis la page de creation Nexus la portent.

ALTER TABLE community_events
    ADD COLUMN IF NOT EXISTS source_server_id uuid;

COMMENT ON COLUMN community_events.source_server_id IS
    'Serveur de jeu Nexus a l''origine de cet evenement. Sans contrainte : la table game_servers est dans une autre base logique.';

-- Retrouver l'evenement d'un serveur donne : c'est la requete que fait la
-- suppression d'un serveur, et la seule qui justifie cette colonne.
CREATE INDEX IF NOT EXISTS idx_community_events_source_server
    ON community_events (source_server_id)
    WHERE source_server_id IS NOT NULL;
