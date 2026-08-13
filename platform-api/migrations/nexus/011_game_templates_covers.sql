-- 011_game_templates_covers.sql
--
-- Visuels des jeux dans le catalogue.
--
-- La colonne `cover_image_url` existait depuis 007 mais restait vide : le
-- formulaire de creation n'affichait qu'un emoji. Les vignettes livrees dans
-- web/public/imgs/ la remplissent.
--
-- Chemin RELATIF volontairement : le site le sert tel quel, et le bot le
-- prefixe par WEB_FRONT_URL pour construire l'URL absolue exigee par Discord.
-- Stocker une URL absolue ici figerait le domaine en base.

UPDATE game_templates SET cover_image_url = '/imgs/minecraft_game.jpg'
    WHERE slug = 'minecraft-vanilla';

UPDATE game_templates SET cover_image_url = '/imgs/palworld_game.jpg'
    WHERE slug = 'palworld';

UPDATE game_templates SET cover_image_url = '/imgs/valheim_game.jpg'
    WHERE slug = 'valheim';

UPDATE game_templates SET cover_image_url = '/imgs/factorio_game.jpg'
    WHERE slug = 'factorio';

UPDATE game_templates SET cover_image_url = '/imgs/terraria_game.jpg'
    WHERE slug = 'terraria';

UPDATE game_templates SET cover_image_url = '/imgs/7days2die_game.jpg'
    WHERE slug = '7dtd';

-- ARK n'a pas encore de vignette livree : la colonne reste NULL, le catalogue
-- retombe sur l'emoji du template.
