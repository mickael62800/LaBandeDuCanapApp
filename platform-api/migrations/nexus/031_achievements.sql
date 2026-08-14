-- 031_achievements.sql
--
-- Hauts faits Nexus (cf. DOC/Nexus/haut-faits.md). Premier adaptateur : Palworld.
--
-- Trois tables, conformes au modele de donnees du document :
--
--   achievements       definitions (catalogue), propres a un jeu ou globales ;
--   game_player_links  liaison VERIFIEE membre Discord <-> identite de jeu ;
--   user_achievements  attributions, uniques par (guilde, membre, haut fait).
--
-- Regles structurelles portees par le schema lui-meme :
--
--   * un membre ne peut pas recevoir deux fois le meme haut fait dans la meme
--     guilde -> UNIQUE (guild_id, discord_user_id, achievement_id) ;
--   * la consommation d'evenements est idempotente -> source_event_id unique ;
--   * une identite de jeu appartient a UN SEUL membre par guilde, et un membre
--     n'a qu'une identite par jeu -> deux contraintes d'unicite croisees, qui
--     ferment l'usurpation par homonymie ;
--   * les hauts faits sont propres a une guilde : `guild_id` est porte par
--     l'attribution et la liaison, jamais deduit.

-- ── Catalogue ────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS achievements (
    id UUID PRIMARY KEY,
    -- NULL = haut fait transverse (Discord / Nexus), sinon slug du jeu.
    game TEXT,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    -- Image du haut fait, choisie par l'administrateur depuis le dashboard.
    icon_url TEXT,
    -- Parametres du critere (seuils, durees, nombres). Jamais codes en dur
    -- dans le bot : le document impose que ces valeurs soient configurables.
    criteria JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- 'auto'   : attribuable par un evenement verifie ;
    -- 'manual' : exige une validation d'administrateur (auditee). Les hauts
    --            faits dont le critere n'est pas verifiable automatiquement
    --            restent en 'manual' tant qu'un adaptateur n'est pas valide.
    verification TEXT NOT NULL DEFAULT 'manual'
        CHECK (verification IN ('auto', 'manual')),
    -- Masque tant qu'il n'est pas debloque (hauts faits secrets).
    hidden BOOLEAN NOT NULL DEFAULT FALSE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- `game` est nullable (haut fait transverse) : NULLS NOT DISTINCT rend deux
-- lignes (NULL, 'code') en conflit, ce qu'un UNIQUE classique laisserait passer.
CREATE UNIQUE INDEX IF NOT EXISTS achievements_game_code_key
    ON achievements (game, code) NULLS NOT DISTINCT;

CREATE INDEX IF NOT EXISTS achievements_game_enabled_idx
    ON achievements (game) WHERE enabled;

-- ── Liaison membre Discord <-> identite de jeu ───────────────────────────

CREATE TABLE IF NOT EXISTS game_player_links (
    id UUID PRIMARY KEY,
    guild_id TEXT NOT NULL,
    discord_user_id TEXT NOT NULL,
    game TEXT NOT NULL,
    -- Palworld : SteamID64 (17 chiffres). Le format est valide par le domaine.
    game_player_id TEXT NOT NULL,
    -- NULL tant que la liaison n'est pas confirmee : sans date de verification
    -- aucun haut fait ne doit etre attribue (fail closed).
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Un membre = une identite par jeu et par guilde.
    UNIQUE (guild_id, game, discord_user_id),
    -- Une identite de jeu = un seul membre. Ferme l'usurpation : deux membres
    -- ne peuvent pas revendiquer le meme SteamID.
    UNIQUE (guild_id, game, game_player_id)
);

CREATE INDEX IF NOT EXISTS game_player_links_lookup_idx
    ON game_player_links (guild_id, game, game_player_id)
    WHERE verified_at IS NOT NULL;

-- ── Attributions ─────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_achievements (
    id UUID PRIMARY KEY,
    guild_id TEXT NOT NULL,
    discord_user_id TEXT NOT NULL,
    achievement_id UUID NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
    -- Identite de jeu au moment de l'attribution (trace), NULL pour un haut
    -- fait transverse.
    game_player_id TEXT,
    -- Identifiant de l'evenement source. Unique -> rejouer l'evenement
    -- n'attribue pas deux fois (idempotence de la consommation Redis).
    source_event_id TEXT,
    -- Acteur pour une attribution manuelle (audit). NULL si automatique.
    granted_by TEXT,
    unlocked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (guild_id, discord_user_id, achievement_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS user_achievements_source_event_key
    ON user_achievements (source_event_id) WHERE source_event_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS user_achievements_member_idx
    ON user_achievements (guild_id, discord_user_id, unlocked_at DESC);

-- ── Catalogue Palworld ───────────────────────────────────────────────────
--
-- Repris de DOC/Nexus/haut-faits.md. Tous en `manual` sauf le premier
-- lancement : aucun adaptateur d'evenements Palworld n'est encore valide sur
-- le conteneur reel, et le document interdit de declarer le contraire. Les
-- basculer en 'auto' se fera quand la source d'evenements sera en place.
--
-- `criteria` porte les seuils par defaut, ajustables par serveur.
-- ON CONFLICT DO NOTHING : la migration est rejouable et ne PIETINE PAS les
-- images (`icon_url`) ni les seuils deja choisis par l'administrateur.

INSERT INTO achievements (id, game, code, name, description, category, criteria, verification, enabled)
VALUES
  -- Premier contact : verifiable via la session du Game Portal.
  (gen_random_uuid(), 'palworld', 'first_launch_palworld', 'Premier lancement', 'Rejoindre une session Palworld du portail.', 'decouverte', '{}'::jsonb, 'auto', TRUE),

  -- Progression extreme
  (gen_random_uuid(), 'palworld', 'palworld_full_paldeck', 'Paldeck presque complet', 'Capturer toutes les especes prevues par la saison du serveur.', 'progression', '{"species_required": 0}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_all_towers', 'Maitre des tours', 'Vaincre tous les boss de tour.', 'progression', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_all_legendaries', 'Chasseur de legendes', 'Vaincre tous les boss legendaires configures.', 'progression', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_all_alpha_bosses', 'Dompteur d''Alphas', 'Vaincre tous les Alphas suivis par le serveur.', 'progression', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_max_level', 'Niveau maximum', 'Atteindre le niveau maximal du serveur.', 'progression', '{"level_required": 60}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_technology_complete', 'Technologie ultime', 'Debloquer toutes les technologies prevues.', 'progression', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_endgame', 'Fin de parcours', 'Atteindre simultanement les objectifs de progression de fin de jeu.', 'progression', '{}'::jsonb, 'manual', TRUE),

  -- Defis sans marge d'erreur
  (gen_random_uuid(), 'palworld', 'palworld_boss_no_down', 'Invaincu', 'Vaincre un boss sans etre mis K.O.', 'defis', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_boss_no_death', 'Aucun sacrifice', 'Vaincre un boss sans perte de Pal dans l''equipe.', 'defis', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_boss_under_time', 'Course contre le temps', 'Vaincre un boss avant une duree limite.', 'defis', '{"time_limit_secs": 300}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_boss_under_level', 'Contre toute attente', 'Vaincre un boss avec un niveau inferieur au niveau recommande.', 'defis', '{"level_gap": 5}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_boss_single_element', 'Specialiste elementaire', 'Vaincre un boss avec une equipe d''un seul element.', 'defis', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_boss_single_pal', 'Un seul compagnon', 'Vaincre un boss avec un seul Pal actif.', 'defis', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_no_fast_travel', 'Marcheur infatigable', 'Terminer une expedition ou un objectif sans teleportation.', 'defis', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_no_death_run', 'Sans seconde chance', 'Atteindre un objectif majeur sans mourir.', 'defis', '{}'::jsonb, 'manual', TRUE),

  -- Elevage et maitrise des Pals
  (gen_random_uuid(), 'palworld', 'palworld_perfect_breed', 'Elevage parfait', 'Obtenir un Pal avec les criteres de reproduction definis.', 'elevage', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_passive_master', 'Maitre des passifs', 'Obtenir un Pal avec une combinaison de passifs rare.', 'elevage', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_breed_chain', 'Lignee exceptionnelle', 'Realiser une chaine d''elevage de plusieurs generations.', 'elevage', '{"generations": 5}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_one_species_team', 'Equipe specialisee', 'Vaincre un objectif avec une equipe d''une meme espece.', 'elevage', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_full_team_bred', 'Equipe issue de l''elevage', 'Utiliser une equipe complete issue de reproductions.', 'elevage', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_pal_workforce', 'Main-d''oeuvre parfaite', 'Faire fonctionner une base avec des Pals ayant les aptitudes requises.', 'elevage', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_partner_loyalty', 'Partenaire fidele', 'Utiliser le meme Pal sur une longue progression.', 'elevage', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_rare_collection', 'Collection rare', 'Obtenir plusieurs variantes ou Pals rares suivis par le serveur.', 'elevage', '{}'::jsonb, 'manual', TRUE),

  -- Base et production
  (gen_random_uuid(), 'palworld', 'palworld_automated_base', 'Base autonome', 'Maintenir une production complete pendant une duree definie.', 'base', '{"hours": 24}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_three_bases', 'Triple implantation', 'Maintenir trois bases operationnelles.', 'base', '{"bases": 3}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_raid_proof', 'Forteresse imprenable', 'Resister a plusieurs raids sans batiment critique detruit.', 'base', '{"raids": 3}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_mass_production', 'Production industrielle', 'Produire une quantite elevee d''objets ou de ressources.', 'base', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_logistics_master', 'Maitre logistique', 'Maintenir une base sans rupture de ressources critiques.', 'base', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_base_specialist', 'Base specialisee', 'Atteindre le rendement cible d''une base specialisee.', 'base', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_rebuild', 'Reconstruction heroique', 'Restaurer une base apres un raid ou un incident majeur.', 'base', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_server_supplier', 'Fournisseur du serveur', 'Produire et partager des ressources avec plusieurs joueurs.', 'base', '{}'::jsonb, 'manual', TRUE),

  -- Exploration longue duree
  (gen_random_uuid(), 'palworld', 'palworld_world_explorer', 'Explorateur du monde', 'Decouvrir toutes les zones suivies par le serveur.', 'exploration', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_dungeon_chain', 'Maitre des donjons', 'Terminer plusieurs donjons consecutivement.', 'exploration', '{"dungeons": 5}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_all_fast_travel', 'Reseau complet', 'Decouvrir tous les points de voyage rapide.', 'exploration', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_extreme_expedition', 'Expedition extreme', 'Revenir vivant d''une zone de tres haut niveau.', 'exploration', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_map_without_death', 'Cartographe prudent', 'Explorer une grande partie de la carte sans mourir.', 'exploration', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_night_explorer', 'Enfant de la nuit', 'Accomplir une exploration nocturne complete.', 'exploration', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_sea_to_sky', 'De la mer au ciel', 'Utiliser plusieurs types de montures pendant une meme expedition.', 'exploration', '{}'::jsonb, 'manual', TRUE),

  -- Cooperation et serveur communautaire
  (gen_random_uuid(), 'palworld', 'palworld_coop_boss', 'Boss en equipe', 'Vaincre un boss avec un groupe complet de joueurs.', 'cooperation', '{"players": 4}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_coop_no_down', 'Escouade invincible', 'Reussir un combat de groupe sans joueur mis K.O.', 'cooperation', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_shared_base', 'Base communautaire', 'Participer a la construction d''une base partagee.', 'cooperation', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_rescue_team', 'Equipe de secours', 'Aider plusieurs joueurs a recuperer apres une defaite.', 'cooperation', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_newcomer_mentor', 'Mentor de Palworld', 'Accompagner un nouveau joueur jusqu''a un objectif defini.', 'cooperation', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_server_event', 'Evenement historique', 'Participer a un evenement communautaire majeur.', 'cooperation', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_massive_session', 'Grande expedition', 'Participer a une session reunissant beaucoup de joueurs.', 'cooperation', '{"players": 8}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_guild_legacy', 'Heritage de guilde', 'Contribuer a plusieurs objectifs collectifs du serveur.', 'cooperation', '{}'::jsonb, 'manual', TRUE),

  -- Maitrise totale
  (gen_random_uuid(), 'palworld', 'palworld_speedrunner', 'Coureur de Palworld', 'Atteindre un objectif de progression dans un temps record.', 'maitrise', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_survivalist', 'Survie absolue', 'Atteindre une duree elevee sans mort.', 'maitrise', '{"hours": 20}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_completionist', 'Completionniste', 'Debloquer toutes les categories de hauts faits Palworld.', 'maitrise', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_legendary_trainer', 'Dresseur legendaire', 'Reunir progression, elevage, exploration et combats avances.', 'maitrise', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_world_guardian', 'Gardien du monde', 'Proteger plusieurs bases et participer aux defenses du serveur.', 'maitrise', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_immortal_run', 'Parcours immortel', 'Atteindre la fin de parcours sans aucune mort du joueur.', 'maitrise', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_server_champion', 'Champion du serveur', 'Etre premier dans plusieurs classements Palworld.', 'maitrise', '{}'::jsonb, 'manual', TRUE),
  (gen_random_uuid(), 'palworld', 'palworld_community_legend', 'Legende de Palworld', 'Accomplir un ensemble de hauts faits legendaires.', 'maitrise', '{}'::jsonb, 'manual', TRUE)
ON CONFLICT DO NOTHING;

-- ── Configuration par guilde du module ───────────────────────────────────
--
-- Le salon de publication, l'activation des annonces et la mention eventuelle
-- appartiennent a la config de guilde (regle 6 : un reglage lie a une guilde
-- vit dans bot_definitions/bot_guild_config, pas dans une variable d'env).

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
  'nexus-achievements',
  'Hauts faits',
  'Hauts faits Discord et jeux du Game Portal : catalogue, liaison des identites de jeu et publication Discord.',
  '[
    {"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false,
     "description": "Si OFF, aucun haut fait n''est attribue ni publie."},
    {"key": "announce_enabled", "type": "boolean", "label": "Annoncer les hauts faits", "default": "true", "required": false,
     "depends_on": {"key": "enabled", "equals": "true"},
     "description": "INTERRUPTEUR des messages de haut fait. Si OFF, les hauts faits restent attribues mais rien n''est poste."},
    {"key": "announce_channel_id", "type": "channel", "label": "Salon de publication", "required": false,
     "depends_on": {"key": "announce_enabled", "equals": "true"},
     "description": "Salon ou publier les hauts faits. Si vide, publication dans le salon de session du jeu concerne quand il existe."},
    {"key": "mention_role_id", "type": "role", "label": "Role mentionne", "required": false,
     "depends_on": {"key": "announce_enabled", "equals": "true"},
     "description": "Role a mentionner dans l''annonce. Vide = aucune mention (defaut recommande)."},
    {"key": "public_profiles", "type": "boolean", "label": "Profils consultables", "default": "true", "required": false,
     "depends_on": {"key": "enabled", "equals": "true"},
     "description": "Autorise /haut-faits membre pour consulter les hauts faits d''un autre membre."}
  ]'::jsonb
)
ON CONFLICT (bot_name) DO NOTHING;
