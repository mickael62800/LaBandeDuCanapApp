-- Suppression totale de la fonctionnalite « administrateur tournant ».
--
-- Le module (bot + API + core + web) a ete retire du code. Cette migration
-- nettoie la base : definition du bot, config par serveur, et les deux tables
-- dediees. Idempotente.

DELETE FROM bot_guild_config WHERE bot_name = 'rotation-bot';
DELETE FROM bot_definitions WHERE bot_name = 'rotation-bot';

DROP TABLE IF EXISTS admin_rotation_history;
DROP TABLE IF EXISTS admin_rotation;
