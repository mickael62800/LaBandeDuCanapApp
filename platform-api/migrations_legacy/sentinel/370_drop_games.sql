-- Suppression DEFINITIVE des jeux restants (Blackjack, Slot, Wheel, Tamagotchi,
-- Influence) et des tables partagees de l'economie de jeu, a la demande de
-- l'utilisateur. Le code (bot, API, worker, proto) a deja ete retire (368) et
-- Coude a ete droppe en 369 ; cette migration supprime les tables de donnees
-- restantes. LES DONNEES DE JEU SONT PERDUES, C'EST VOULU.
--
-- user_wallets / wallet_transactions / mv_wallet_leaderboard ne servaient plus
-- qu'aux jeux (le bump ne credite plus de wallet depuis 368) : supprimes aussi.
-- game-bot (portail de roles) et game_portal (serveurs de jeu conteneurises)
-- sont CONSERVES.

-- Vues materialisees d'abord (dependent des tables).
DROP MATERIALIZED VIEW IF EXISTS mv_wallet_leaderboard;

-- Blackjack
DROP TABLE IF EXISTS blackjack_games CASCADE;
DROP TABLE IF EXISTS blackjack_table_players CASCADE;
DROP TABLE IF EXISTS blackjack_tables CASCADE;

-- Slot
DROP TABLE IF EXISTS slot_daily_claims CASCADE;
DROP TABLE IF EXISTS slot_jackpot_pool CASCADE;
DROP TABLE IF EXISTS slot_spin_log CASCADE;

-- Wheel
DROP TABLE IF EXISTS wheel_daily_claims CASCADE;
DROP TABLE IF EXISTS wheel_spin_log CASCADE;

-- Tamagotchi
DROP TABLE IF EXISTS pet_events CASCADE;
DROP TABLE IF EXISTS pets CASCADE;

-- Influence
DROP TABLE IF EXISTS influence_archives CASCADE;
DROP TABLE IF EXISTS influence_capital_movements CASCADE;
DROP TABLE IF EXISTS influence_citizens CASCADE;
DROP TABLE IF EXISTS influence_information CASCADE;
DROP TABLE IF EXISTS influence_investigations CASCADE;
DROP TABLE IF EXISTS influence_laws CASCADE;
DROP TABLE IF EXISTS influence_motions CASCADE;
DROP TABLE IF EXISTS influence_org_members CASCADE;
DROP TABLE IF EXISTS influence_org_relations CASCADE;
DROP TABLE IF EXISTS influence_org_treasury_movements CASCADE;
DROP TABLE IF EXISTS influence_organizations CASCADE;
DROP TABLE IF EXISTS influence_reputation_dims CASCADE;
DROP TABLE IF EXISTS influence_votes CASCADE;

-- Economie partagee des jeux
DROP TABLE IF EXISTS wallet_transactions CASCADE;
DROP TABLE IF EXISTS user_wallets CASCADE;

-- Config RBAC des composants web supprimes avec les jeux.
DELETE FROM rbac_component_visibility
WHERE component_key IN ('db.purge.blackjack', 'db.reset.wallets');
