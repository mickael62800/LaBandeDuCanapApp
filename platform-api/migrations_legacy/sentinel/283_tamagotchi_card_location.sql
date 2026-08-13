-- Localisation de la carte Discord (salon prive du joueur) pour permettre au
-- bot de rafraichir automatiquement l'affichage (re-edition horaire + sur
-- maladie/mort) sans que le joueur ait besoin de cliquer.
ALTER TABLE pets
    ADD COLUMN IF NOT EXISTS card_channel_id TEXT,
    ADD COLUMN IF NOT EXISTS card_message_id TEXT;
