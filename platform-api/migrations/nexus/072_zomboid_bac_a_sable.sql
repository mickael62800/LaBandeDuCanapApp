-- 072_zomboid_bac_a_sable.sql
--
-- Le bac a sable de Project Zomboid devient reglable depuis l'interface.
--
-- CE QUI CHANGE PAR RAPPORT AU RESTE DU FORMULAIRE. Toutes les autres cles de
-- configuration deviennent des variables d'environnement du conteneur.
-- Celles-ci, prefixees `SANDBOX_`, n'y vont PAS : elles sont ecartees de
-- l'environnement et composent un fichier `SandboxVars.lua` depose dans la
-- sauvegarde avant le premier demarrage.
--
-- POURQUOI CE DETOUR. Population de zombies, butin, duree du jour, vitesse des
-- morts : aucune de ces valeurs n'est une variable d'environnement, ni dans
-- cette image ni dans aucune autre. Le jeu ne les lit que dans ce fichier.
--
-- LE FICHIER N'EST LU QU'AU DEMARRAGE. Project Zomboid le charge une fois, au
-- lancement, et n'y revient jamais. Chaque avertissement ci-dessous le
-- rappelle, sinon on croit le reglage perdu.
--
-- VINGT-SIX REGLAGES SUR PLUS DE QUATRE-VINGTS. Ceux retenus sont ceux qu'une
-- communaute discute avant de lancer une soiree. Les autres restent
-- accessibles en jeu, par le menu d'administration du compte `superuser` —
-- et chaque reglage de plus dans ce formulaire est une occasion de casser une
-- partie en cours.
--
-- Les cles absentes prennent la valeur par defaut du jeu : un champ laisse
-- vide ne fige rien.

UPDATE game_templates SET
    config_schema = config_schema || '[
      {"key": "SANDBOX_Zombies", "type": "enum", "label": "Population de zombies",
       "group": "Bac a sable — Monde", "default": "3",
       "options": ["1", "2", "3", "4", "5"],
       "description": "1 = insensee, 2 = elevee, 3 = normale, 4 = faible, 5 = tres faible.",
       "warning": "Ne prend effet qu au prochain demarrage du serveur."},

      {"key": "SANDBOX_Distribution", "type": "enum", "label": "Repartition des zombies",
       "group": "Bac a sable — Monde", "default": "1",
       "options": ["1", "2"],
       "description": "1 = urbaine (concentres en ville), 2 = uniforme."},

      {"key": "SANDBOX_DayLength", "type": "enum", "label": "Duree d une journee",
       "group": "Bac a sable — Monde", "default": "3",
       "options": ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"],
       "description": "1 = quinze minutes, 3 = une heure, 11 = temps reel. Le choix le plus structurant pour une soiree."},

      {"key": "SANDBOX_StartYear", "type": "number", "label": "Annee de depart",
       "group": "Bac a sable — Monde", "default": 1, "min": 1, "max": 100,
       "description": "1 = premiere annee de l epidemie."},

      {"key": "SANDBOX_StartMonth", "type": "number", "label": "Mois de depart",
       "group": "Bac a sable — Monde", "default": 7, "min": 1, "max": 12,
       "description": "L hiver rend la survie nettement plus dure."},

      {"key": "SANDBOX_StartDay", "type": "number", "label": "Jour de depart",
       "group": "Bac a sable — Monde", "default": 9, "min": 1, "max": 31},

      {"key": "SANDBOX_StartTime", "type": "enum", "label": "Heure de depart",
       "group": "Bac a sable — Monde", "default": "2",
       "options": ["1", "2", "3", "4", "5", "6", "7", "8", "9"],
       "description": "1 = 7h, 2 = 9h, 5 = midi, 9 = 2h du matin."},

      {"key": "SANDBOX_WaterShut", "type": "enum", "label": "Coupure de l eau",
       "group": "Bac a sable — Monde", "default": "2",
       "options": ["1", "2", "3", "4", "5", "6", "7", "8"],
       "description": "1 = immediate, 2 = 0 a 30 jours, 8 = jamais."},

      {"key": "SANDBOX_ElecShut", "type": "enum", "label": "Coupure de l electricite",
       "group": "Bac a sable — Monde", "default": "2",
       "options": ["1", "2", "3", "4", "5", "6", "7", "8"],
       "description": "Meme echelle que l eau. Coupe le chauffage et les congelateurs."},

      {"key": "SANDBOX_NightDarkness", "type": "enum", "label": "Obscurite nocturne",
       "group": "Bac a sable — Monde", "default": "3",
       "options": ["1", "2", "3", "4"],
       "description": "1 = penombre, 4 = nuit noire."},

      {"key": "SANDBOX_FoodLoot", "type": "enum", "label": "Nourriture trouvee",
       "group": "Bac a sable — Butin", "default": "4",
       "options": ["1", "2", "3", "4", "5", "6"],
       "description": "1 = tres rare, 4 = normal, 6 = abondant."},

      {"key": "SANDBOX_WeaponLoot", "type": "enum", "label": "Armes trouvees",
       "group": "Bac a sable — Butin", "default": "4",
       "options": ["1", "2", "3", "4", "5", "6"]},

      {"key": "SANDBOX_MedicalLoot", "type": "enum", "label": "Materiel medical trouve",
       "group": "Bac a sable — Butin", "default": "4",
       "options": ["1", "2", "3", "4", "5", "6"]},

      {"key": "SANDBOX_OtherLoot", "type": "enum", "label": "Autres objets trouves",
       "group": "Bac a sable — Butin", "default": "4",
       "options": ["1", "2", "3", "4", "5", "6"]},

      {"key": "SANDBOX_XpMultiplier", "type": "number", "label": "Multiplicateur d experience",
       "group": "Bac a sable — Personnage", "default": 1, "min": 0, "max": 1000,
       "description": "1 = normal. Les decimales sont acceptees (1.5)."},

      {"key": "SANDBOX_CharacterFreePoints", "type": "number", "label": "Points de creation offerts",
       "group": "Bac a sable — Personnage", "default": 0, "min": 0, "max": 100,
       "description": "Points supplementaires a la creation du personnage."},

      {"key": "SANDBOX_ZombieAttractionMultiplier", "type": "number", "label": "Attirance des zombies au bruit",
       "group": "Bac a sable — Personnage", "default": 1, "min": 0, "max": 100,
       "description": "1 = normal. Plus haut, un coup de feu ameute toute la ville."},

      {"key": "SANDBOX_Speed", "type": "enum", "label": "Vitesse des zombies",
       "group": "Bac a sable — Les morts", "default": "2",
       "options": ["1", "2", "3", "4"],
       "description": "1 = sprinteurs, 2 = rapides, 3 = lents, 4 = tres lents.",
       "warning": "Le reglage qui change le plus la difficulte reelle."},

      {"key": "SANDBOX_Strength", "type": "enum", "label": "Force des zombies",
       "group": "Bac a sable — Les morts", "default": "2",
       "options": ["1", "2", "3"],
       "description": "1 = surhumaine, 2 = normale, 3 = faible."},

      {"key": "SANDBOX_Toughness", "type": "enum", "label": "Resistance des zombies",
       "group": "Bac a sable — Les morts", "default": "2",
       "options": ["1", "2", "3"],
       "description": "1 = coriaces, 2 = normaux, 3 = fragiles."},

      {"key": "SANDBOX_Cognition", "type": "enum", "label": "Intelligence des zombies",
       "group": "Bac a sable — Les morts", "default": "3",
       "options": ["1", "2", "3"],
       "description": "1 = savent ouvrir les portes, 3 = basiques."},

      {"key": "SANDBOX_Memory", "type": "enum", "label": "Memoire des zombies",
       "group": "Bac a sable — Les morts", "default": "2",
       "options": ["1", "2", "3", "4"],
       "description": "1 = longue traque, 4 = oublient vite."},

      {"key": "SANDBOX_Sight", "type": "enum", "label": "Vue des zombies",
       "group": "Bac a sable — Les morts", "default": "2",
       "options": ["1", "2", "3"],
       "description": "1 = percante, 3 = mediocre."},

      {"key": "SANDBOX_Hearing", "type": "enum", "label": "Ouie des zombies",
       "group": "Bac a sable — Les morts", "default": "2",
       "options": ["1", "2", "3"],
       "description": "1 = fine, 3 = mediocre."},

      {"key": "SANDBOX_Smell", "type": "enum", "label": "Odorat des zombies",
       "group": "Bac a sable — Les morts", "default": "2",
       "options": ["1", "2", "3"]},

      {"key": "SANDBOX_ActiveOnly", "type": "enum", "label": "Periode d activite",
       "group": "Bac a sable — Les morts", "default": "1",
       "options": ["1", "2", "3"],
       "description": "1 = jour et nuit, 2 = actifs la nuit, 3 = actifs le jour."}
    ]'::jsonb,
    updated_at = now()
WHERE slug = 'project-zomboid'
  AND NOT (config_schema @> '[{"key": "SANDBOX_Zombies"}]'::jsonb);
