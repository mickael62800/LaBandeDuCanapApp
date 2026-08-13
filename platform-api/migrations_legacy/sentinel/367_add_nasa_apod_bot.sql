-- Module nasa-apod : publie chaque jour la photo de l'espace de la NASA (APOD)
-- dans un salon textuel configure.
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'nasa-apod-bot',
    'Photo de l''espace (NASA)',
    'Publie chaque jour l''Astronomy Picture of the Day de la NASA dans un salon. Traduction FR optionnelle via DeepL.',
    '[
        {"key":"enabled","label":"Bot actif","type":"boolean","required":false,"default":"false","description":"Active la publication quotidienne de la photo du jour."},
        {"key":"channel_id","label":"Salon de publication","type":"channel","required":true,"default":"","description":"Salon textuel où poster la photo de l''espace chaque jour."},
        {"key":"nasa_api_key","label":"Clé API NASA","type":"text","required":true,"default":"","description":"Clé gratuite à créer sur api.nasa.gov. Obligatoire."},
        {"key":"deepl_api_key","label":"Clé API DeepL (traduction FR)","type":"text","required":false,"default":"","description":"Clé DeepL pour traduire titre et explication en français. Vide = texte original en anglais."},
        {"key":"post_hour","label":"Heure de publication (0-23)","type":"number","required":false,"default":"9","description":"Heure quotidienne de publication, dans le fuseau choisi ci-dessous."},
        {"key":"timezone_offset","label":"Décalage horaire vs UTC","type":"number","required":false,"default":"1","description":"Décalage du fuseau du serveur : Paris = 1 en hiver, 2 en été. 0 = UTC."}
    ]'
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
