-- 041_palworld_commandes_completes.sql
--
-- Complete le catalogue Palworld : les quatre commandes qui manquaient.
--
-- La migration 040 en posait sept ; le serveur dedie Palworld en expose onze,
-- pas une de plus. Ce n'est pas la page qui est pauvre, c'est le protocole du
-- jeu : `PalServer` ne sait ni changer la meteo, ni donner un objet, ni
-- teleporter un joueur ailleurs qu'aupres de l'administrateur. Tout le reste
-- de l'administration passe par la configuration du serveur (onglet
-- Configuration, une centaine de reglages) et par le cycle de vie du
-- conteneur, pas par RCON.
--
-- Mieux vaut le dire que laisser chercher : le catalogue est desormais
-- COMPLET pour ce jeu.
--
-- Idempotente : les cles deja presentes sont retirees avant d'etre reecrites,
-- ce qui permet aussi de corriger un libelle sans creer de doublon.

UPDATE game_templates SET command_schema = (
    SELECT COALESCE(jsonb_agg(elem ORDER BY ord), '[]'::jsonb)
    FROM jsonb_array_elements(command_schema) WITH ORDINALITY AS t(elem, ord)
    WHERE elem ->> 'key' NOT IN (
        'unban_player', 'do_exit', 'teleport_to_me', 'teleport_to_player'
    )
)
WHERE slug = 'palworld';

UPDATE game_templates SET command_schema = command_schema || '[
  {"key": "unban_player", "label": "Lever un bannissement", "group": "Joueurs",
   "template": "UnBanPlayer {steamid}",
   "confirm": true,
   "description": "Rend l''acces au serveur a un joueur banni.",
   "params": [
     {"key": "steamid", "label": "Identifiant Steam", "type": "text", "required": true,
      "max_length": 32,
      "description": "A saisir a la main : un joueur banni ne figure evidemment pas dans la liste des connectes."}
   ]},

  {"key": "teleport_to_me", "label": "Faire venir un joueur", "group": "Joueurs",
   "template": "TeleportToMe {steamid}",
   "confirm": true,
   "description": "Teleporte le joueur aupres de l''administrateur.",
   "warning": "N''a d''effet que si tu es toi-meme connecte en jeu.",
   "params": [
     {"key": "steamid", "label": "Joueur", "type": "player", "required": true}
   ]},

  {"key": "teleport_to_player", "label": "Rejoindre un joueur", "group": "Joueurs",
   "template": "TeleportToPlayer {steamid}",
   "confirm": true,
   "description": "Teleporte l''administrateur aupres du joueur.",
   "warning": "N''a d''effet que si tu es toi-meme connecte en jeu.",
   "params": [
     {"key": "steamid", "label": "Joueur", "type": "player", "required": true}
   ]},

  {"key": "do_exit", "label": "Arreter immediatement", "group": "Maintenance",
   "template": "DoExit",
   "confirm": true, "danger": true,
   "description": "Coupe le serveur sur-le-champ, sans preavis pour les joueurs.",
   "warning": "Tout ce qui n''a pas ete sauvegarde est perdu. Sauvegarde d''abord, ou prefere un arret avec delai."}
]'::jsonb
WHERE slug = 'palworld';
