-- Ajout du bot image-bot et des definitions de configuration

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'image-bot',
    'Image Bot',
    'Bot de detection d''images NSFW et produits illicites via inference IA (ONNX)',
    '[
        {"key": "log_channel_id", "label": "Salon de logs", "type": "channel", "required": false},
        {"key": "confidence_threshold", "label": "Seuil de confiance IA (0.0 a 1.0)", "type": "number", "required": false, "default": "0.5"},
        {"key": "scan_embeds", "label": "Scanner les images dans les embeds", "type": "boolean", "required": false, "default": "true"},
        {"key": "max_image_size_mb", "label": "Taille max image (MB)", "type": "number", "required": false, "default": "10"},
        {"key": "ignored_roles", "label": "Roles ignores (IDs separes par des virgules)", "type": "text", "required": false}
    ]'
) ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;

-- Commentaire : les nouveaux flag_type IA sont supportes nativement
-- car la colonne rules.flag_type est de type TEXT.
-- Types disponibles : spam, insult, link, phishing, nsfw, illicit, anger, rage, threat, harassment
-- Les regles IA sont creees a la demande par guild via l'API existante POST /rules
