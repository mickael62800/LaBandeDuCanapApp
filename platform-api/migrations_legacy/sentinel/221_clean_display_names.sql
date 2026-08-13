-- UI : nettoyage des display_name redondants. Apres la fusion, il
-- n'y a plus qu'un seul process worker et un seul process bot, donc
-- preciser "Worker" / "Bots" dans le nom du module est redondant
-- avec la section deja affichee dans la sidebar.

UPDATE bot_definitions SET display_name = 'Surveillance'
    WHERE bot_name = 'monitoring' AND display_name = 'Surveillance bots / workers';

UPDATE bot_definitions SET display_name = 'IA (texte + vision)'
    WHERE bot_name = 'ai' AND display_name = 'Workers IA (texte + vision)';

UPDATE bot_definitions SET display_name = 'Cache Redis'
    WHERE bot_name = 'cache' AND display_name = 'Cache Redis (warm)';
