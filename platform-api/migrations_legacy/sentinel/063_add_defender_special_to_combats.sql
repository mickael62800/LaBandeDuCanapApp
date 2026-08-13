-- Ajout de la colonne defender_special pour les objets defensifs en combat
ALTER TABLE coude_combats ADD COLUMN IF NOT EXISTS defender_special TEXT;
