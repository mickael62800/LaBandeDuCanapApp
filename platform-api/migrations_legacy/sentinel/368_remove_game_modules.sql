-- Suppression des modules de JEU (Influence, Coup de Coude, Blackjack, Slot,
-- Wheel, Tamagotchi) : le code API + bot a ete retire, on reutilisera ces jeux
-- plus tard dans un bot dedie.
--
-- Cette migration retire uniquement les DEFINITIONS et la CONFIG par serveur de
-- ces modules, pour qu'ils disparaissent de la page Composants. Les TABLES DE
-- DONNEES des jeux (coude_*, influence_*, blackjack*, slot_*, wheel_*,
-- tamagotchi_*, user_wallets, *_taunts*, ...) sont VOLONTAIREMENT CONSERVEES :
-- pas de perte de donnees. Un futur DROP explicite pourra les nettoyer si besoin.
--
-- On garde 'game-bot' (portail de roles de jeux) et 'game-portal' (serveurs).

DELETE FROM bot_guild_config
WHERE bot_name IN (
    'coude-bot',
    'influence-bot',
    'blackjack-bot',
    'slot-bot',
    'wheel-bot',
    'tamagotchi-bot'
);

DELETE FROM bot_definitions
WHERE bot_name IN (
    'coude-bot',
    'influence-bot',
    'blackjack-bot',
    'slot-bot',
    'wheel-bot',
    'tamagotchi-bot'
);
