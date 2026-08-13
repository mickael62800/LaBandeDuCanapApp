-- Ajout du message de retour pour les membres qui reviennent
ALTER TABLE welcome_config
    ADD COLUMN rejoin_message TEXT NOT NULL DEFAULT 'Content de te revoir {user} ! Tu nous avais manque.';
