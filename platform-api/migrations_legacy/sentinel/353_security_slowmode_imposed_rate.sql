-- Le revert du slowmode anti-raid (chemin worker + consumer) restaurait
-- l'ancien rate aveuglement, ecrasant un slowmode pose manuellement par un modo
-- pendant la fenetre. On persiste le rate IMPOSE par le raid pour que le
-- consumer ne restaure un salon que s'il porte ENCORE cette valeur.
ALTER TABLE security_slowmode_active
    ADD COLUMN IF NOT EXISTS imposed_rate INT NOT NULL DEFAULT 0;
