-- Migration 173 : Seed des templates flavor pour /refuser (cf.
-- bots/.../refuser.rs SHAME_MESSAGES) — finalisation Phase 3 #9 audit.
-- Cle : `combat_refused`. Aucun placeholder (le bot prefixe la mention
-- du joueur lui-meme).

INSERT INTO coude_flavor_templates (key, content) VALUES
('combat_refused', E'a fui comme un poulet sans tete !'),
('combat_refused', E'a prefere se cacher sous la table...'),
('combat_refused', E'a tremble de peur et s''est enfui !'),
('combat_refused', E'a fait pipi dans son pantalon !'),
('combat_refused', E'a pleure en appelant sa maman !'),
('combat_refused', E'a couru si vite qu''il a perdu ses chaussures !'),
('combat_refused', E'a fait semblant d''avoir un rendez-vous urgent...'),
('combat_refused', E's''est cache derriere un buisson !'),
('combat_refused', E'a invente une excuse bidon pour fuir !'),
('combat_refused', E'a declare forfait avant meme de commencer !');
