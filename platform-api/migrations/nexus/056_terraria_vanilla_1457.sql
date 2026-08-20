-- 056_terraria_vanilla_1457.sql
--
-- Terraria passe de TShock a l'image vanilla, en version 1.4.5.7.
--
-- POURQUOI. Le client Terraria se met a jour tout seul par Steam ; le serveur,
-- lui, est fige au digest (migration 029). Le jeu est passe en 1.4.5.7 le
-- 20 aout 2026, alors que le dernier serveur TShock publie reste
-- `tshock-1.4.5.6-6.1.0`, du 11 mars. Les joueurs recoivent donc une erreur de
-- version au moment de se connecter, et AUCUN tag TShock ne permet d'y
-- remedier : il n'y a rien de plus recent a epingler.
--
-- L'epinglage par digest n'est pas en cause et n'est pas remis en question :
-- il empeche le registre de changer le contenu d'un tag sous nos pieds. Il
-- rend simplement visible un decalage qui existait de toute facon.
--
-- CE QUE L'ON PERD. TShock apportait des greffons et une console
-- d'administration. La fiche ne s'en servait pas : `supports_rcon` et
-- `supports_mods` valent faux depuis la migration 007. La perte est donc
-- theorique tant que personne n'ecrit de greffon.
--
-- CE QUE L'ON GARDE. Le monde. Le fichier `.wld` est au format standard du
-- jeu et vit sur le volume, dans `/root/.local/share/Terraria/Worlds` : le
-- serveur vanilla le recharge tel quel. Le port et l'adresse ne changent pas
-- non plus.
--
-- QUAND TSHOCK REVIENDRA, une migration pourra reprendre le tag tshock d'une
-- version 1.4.5.7 des qu'il existera ; rien ici ne l'en empeche.

UPDATE game_templates
SET image = 'ryshe/terraria:vanilla-1.4.5.7@sha256:00358c39e6df934dc90cb55191ad5253f3af033197cee932356ef1af9b3cb416',

    -- CONFIGPATH et LOGPATH designaient des repertoires de TShock, absents de
    -- l'image vanilla. On reprend les chemins par defaut documentes par
    -- l'image. WORLDPATH est rendu explicite : c'est le repertoire monte en
    -- volume, donc le seul endroit ou un monde survit a la recreation du
    -- conteneur.
    default_env = (default_env - 'CONFIGPATH' - 'LOGPATH') || jsonb_build_object(
        'WORLDPATH', '/root/.local/share/Terraria/Worlds',
        'LOGPATH', '/terraria-server/logs'
    ),

    -- Le `config.json` de TShock n'a plus de destinataire. Le laisser
    -- deposerait a chaque demarrage un fichier que rien ne lit — et il portait
    -- le mot de passe du serveur, qui n'a rien a faire sur un volume sans
    -- raison.
    init_files = '[]'::jsonb,

    -- Consequence directe : le mot de passe passait par ce fichier. Sans
    -- `-password` sur la ligne de commande, le reglage resterait affiche a
    -- l'ecran, modifiable, et sans le moindre effet — un serveur cense etre
    -- protege serait ouvert a tous.
    --
    -- Une valeur vide vaut « pas de mot de passe », ce qui est bien le
    -- comportement attendu du reglage.
    command_template = '["-autocreate", "{{WORLD_SIZE}}", "-worldname", "{{WORLD_NAME}}", "-difficulty", "{{DIFFICULTY}}", "-port", "7777", "-maxplayers", "{{MAX_PLAYERS}}", "-motd", "{{MOTD}}", "-password", "{{PASSWORD}}"]',

    description = 'Bac a sable 2D, exploration et boss. Image vanilla 1.4.5.7 : la version du serveur doit suivre celle du client Steam.',
    updated_at = now()
WHERE slug = 'terraria';

-- Le conteneur en service porte encore l'ancienne image : Docker fige l'image
-- au moment de la creation. Ce drapeau demande sa recreation au prochain
-- demarrage, faute de quoi la fiche serait a jour et le serveur toujours en
-- 1.4.5.6.
UPDATE game_servers AS s
SET config_dirty = true,
    updated_at = now()
FROM game_templates AS t
WHERE s.template_id = t.id
  AND t.slug = 'terraria'
  AND s.container_id IS NOT NULL;
