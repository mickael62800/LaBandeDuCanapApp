-- 037_modules_enabled_explicite.sql
--
-- Rend explicite l'activation des modules Nexus deja en service.
--
-- Le dashboard applique la regle du depot : sans ligne `enabled`, un module
-- est eteint. L'API, elle, partait de `enabled: true` par defaut. Resultat :
-- le dashboard affichait « inactif » pendant que Discord repondait
-- normalement, et rien ne permettait de savoir lequel des deux disait vrai.
--
-- Le defaut cote API vient de passer a `false` (fail closed). Sans cette
-- migration, tous les serveurs qui jouent aujourd'hui verraient leur module
-- s'eteindre au deploiement — une regle de securite ne doit pas se payer par
-- la disparition silencieuse d'un jeu en cours.
--
-- On inscrit donc `enabled = true` pour les seules guildes qui utilisent
-- REELLEMENT le module : celles qui ont deja une configuration ou des
-- donnees de jeu. Une guilde qui n'a jamais touche au module reste eteinte,
-- ce qui est precisement le comportement voulu.
--
-- Idempotente : `ON CONFLICT DO NOTHING` preserve un `enabled` deja pose,
-- y compris un `false` volontaire.

-- ── Coussin Piege ──

INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
SELECT guild_id, 'nexus-coussin', 'enabled', 'true'
FROM (
    -- Une configuration existante prouve que quelqu'un s'est occupe du module.
    SELECT DISTINCT guild_id FROM bot_guild_config WHERE bot_name = 'nexus-coussin'
    UNION
    -- Des joueurs prouvent que le module a servi, meme sans reglage touche.
    SELECT DISTINCT guild_id FROM nexus_coussin_players
) AS actives
ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

-- ── Economie (portefeuilles, roue) ──

INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
SELECT guild_id, 'nexus-economy', 'enabled', 'true'
FROM (
    SELECT DISTINCT guild_id FROM bot_guild_config WHERE bot_name = 'nexus-economy'
    UNION
    SELECT DISTINCT guild_id FROM nexus_wallets
) AS actives
ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;
