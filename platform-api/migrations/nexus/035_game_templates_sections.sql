-- 035_game_templates_sections.sql
--
-- Range dans une section les reglages qui n'en ont aucune.
--
-- Les sept jeux crees par la migration 007 ont ete ecrits sans champ `group`.
-- Les reglages ajoutes ensuite (012 a 024) en ont tous un : Monde, Mods,
-- Sauvegardes, Acces... Resultat a l'ecran, un jeu comme Minecraft affiche de
-- vraies sections ET un fourre-tout d'une quinzaine de reglages ou le PvP
-- cotoie la distance d'affichage.
--
-- Le front ne peut pas deviner la section : `web/src/composables/
-- useTemplateFieldGroups.ts` regroupe sur `group` et rassemble le reste sous
-- « Reglages generaux ». La correspondance appartient donc aux donnees.
--
-- Les noms de section reprennent EXACTEMENT ceux deja utilises, accents
-- compris (« Regles du jeu », « Acces ») : une variante d'orthographe
-- creerait une section jumelle a cote de l'originale.
--
-- 007 garde volontairement ses champs sans `group` : sqlx enregistre une
-- empreinte de chaque migration appliquee et refuse de demarrer si elle
-- change (voir 017). Une migration appliquee ne se modifie jamais, elle se
-- repare par une suivante.
--
-- Idempotente et non destructive : un champ qui porte deja un `group` n'est
-- jamais touche, y compris si un serveur a personnalise son modele.

UPDATE game_templates AS t
SET config_schema = (
    SELECT jsonb_agg(
        -- Une cle absente de la correspondance reste SANS `group` : le front
        -- la range alors sous « Reglages generaux », en fin de formulaire.
        -- Inventer une section ici la ferait disparaitre dans un intitule que
        -- personne n'a choisi.
        CASE
            WHEN elem ? 'group' OR sections.section IS NULL THEN elem
            ELSE elem || jsonb_build_object('group', sections.section)
        END
        ORDER BY ord
    )
    FROM jsonb_array_elements(t.config_schema) WITH ORDINALITY AS champs(elem, ord)
    LEFT JOIN (
        VALUES
            -- Identite et capacite du serveur.
            ('SERVER_NAME',                  'Serveur'),
            ('SERVER_DESCRIPTION',           'Serveur'),
            ('SESSION_NAME',                 'Serveur'),
            ('GAME_NAME',                    'Serveur'),
            ('MOTD',                         'Serveur'),
            ('VERSION',                      'Serveur'),
            ('SERVER_PUBLIC',                'Serveur'),
            ('MAX_PLAYERS',                  'Joueurs'),
            ('PLAYERS',                      'Joueurs'),

            -- Qui entre, et avec quel pouvoir.
            ('ADMIN_PASSWORD',               'Acces'),
            ('SERVER_PASSWORD',              'Acces'),
            ('SERVER_PASS',                  'Acces'),
            ('PASSWORD',                     'Acces'),
            ('WHITE_LIST',                   'Acces'),
            ('ONLINE_MODE',                  'Acces'),

            -- La carte et ce qui y vit.
            ('WORLD_NAME',                   'Monde'),
            ('WORLD_SIZE',                   'Monde'),
            ('WORLD_GEN_SEED',               'Monde'),
            ('SERVER_MAP',                   'Monde'),
            ('SAVE_NAME',                    'Monde'),
            ('GENERATE_NEW_SAVE',            'Monde'),
            ('LOAD_LATEST_SAVE',             'Monde'),
            ('ALLOW_NETHER',                 'Monde'),
            ('SPAWN_ANIMALS',                'Monde'),
            ('SPAWN_MONSTERS',               'Monde'),
            ('SPAWN_NPCS',                   'Monde'),
            ('DAY_NIGHT_LENGTH',             'Monde'),

            -- Ce qui change la partie elle-meme.
            ('DIFFICULTY',                   'Regles du jeu'),
            ('GAME_DIFFICULTY',              'Regles du jeu'),
            ('DIFFICULTY_OFFSET',            'Regles du jeu'),
            ('MODE',                         'Regles du jeu'),
            ('PVP',                          'Regles du jeu'),
            ('DEATH_PENALTY',                'Regles du jeu'),
            ('ZOMBIES_RUN',                  'Regles du jeu'),
            ('ENABLE_COMMAND_BLOCK',         'Regles du jeu'),
            ('ANNOUNCE_PLAYER_ACHIEVEMENTS', 'Regles du jeu'),
            ('TAMING_SPEED',                 'Regles du jeu'),
            ('XP_MULTIPLIER',                'Regles du jeu'),

            ('BACKUPS',                      'Sauvegardes'),
            ('BACKUPS_INTERVAL',             'Sauvegardes'),

            -- Ce qui se paie en CPU et en RAM.
            ('VIEW_DISTANCE',                'Performance'),
            ('SIMULATION_DISTANCE',          'Performance'),
            ('MULTITHREADING',               'Performance'),

            ('UPDATE_MODS_ON_START',         'Mods')
    ) AS sections(cle, section) ON sections.cle = elem ->> 'key'
)
-- `group` est entre guillemets dans le chemin : c'est un mot reserve du
-- langage jsonpath. La garde sur le type evite `jsonb_array_elements` sur un
-- schema qui ne serait pas un tableau.
WHERE jsonb_typeof(config_schema) = 'array'
  AND jsonb_path_exists(config_schema, '$[*] ? (!exists(@."group"))');
