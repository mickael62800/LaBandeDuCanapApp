-- Suppression DEFINITIVE du jeu Coup de Coude, a la demande de l'utilisateur.
-- Le code (bot, API, worker, proto) a deja ete retire ; cette migration
-- supprime la vue materialisee, toutes les tables de donnees coude_* et
-- l'enum coude_class. LES DONNEES COUDE SONT PERDUES, C'EST VOULU.
--
-- Les tables partagees ou des autres jeux (user_wallets, wallet_transactions,
-- blackjack*, slot_*, wheel_*, tamagotchi_*, influence_*, user_cache, ...)
-- ne sont PAS touchees.

DROP MATERIALIZED VIEW IF EXISTS mv_coude_leaderboard;

DROP TABLE IF EXISTS coude_bets CASCADE;
DROP TABLE IF EXISTS coude_bounties CASCADE;
DROP TABLE IF EXISTS coude_bounty_contributions CASCADE;
DROP TABLE IF EXISTS coude_cashbox CASCADE;
DROP TABLE IF EXISTS coude_cashbox_redistribution_entries CASCADE;
DROP TABLE IF EXISTS coude_cashbox_redistributions CASCADE;
DROP TABLE IF EXISTS coude_casino_log CASCADE;
DROP TABLE IF EXISTS coude_coalition_members CASCADE;
DROP TABLE IF EXISTS coude_coalitions CASCADE;
DROP TABLE IF EXISTS coude_combats CASCADE;
DROP TABLE IF EXISTS coude_cooldowns CASCADE;
DROP TABLE IF EXISTS coude_curses CASCADE;
DROP TABLE IF EXISTS coude_daily_chaos CASCADE;
DROP TABLE IF EXISTS coude_dons CASCADE;
DROP TABLE IF EXISTS coude_events CASCADE;
DROP TABLE IF EXISTS coude_flavor_templates CASCADE;
DROP TABLE IF EXISTS coude_heist_attempts CASCADE;
DROP TABLE IF EXISTS coude_insurances CASCADE;
DROP TABLE IF EXISTS coude_inventory CASCADE;
DROP TABLE IF EXISTS coude_players CASCADE;
DROP TABLE IF EXISTS coude_primes CASCADE;
DROP TABLE IF EXISTS coude_prison CASCADE;
DROP TABLE IF EXISTS coude_refusal_counts CASCADE;
DROP TABLE IF EXISTS coude_safety_nets CASCADE;
DROP TABLE IF EXISTS coude_season_titles CASCADE;
DROP TABLE IF EXISTS coude_seasons CASCADE;
DROP TABLE IF EXISTS coude_steal_attempts CASCADE;
DROP TABLE IF EXISTS coude_steal_boosts CASCADE;
DROP TABLE IF EXISTS coude_steal_protections CASCADE;
DROP TABLE IF EXISTS coude_taunts_config CASCADE;
DROP TABLE IF EXISTS coude_taunts_opt_outs CASCADE;
DROP TABLE IF EXISTS coude_tout_ou_rien_log CASCADE;
DROP TABLE IF EXISTS coude_ultimate_states CASCADE;
DROP TABLE IF EXISTS coude_vendettas CASCADE;
DROP TABLE IF EXISTS coude_weekly_tournaments CASCADE;

DROP TYPE IF EXISTS coude_class;
