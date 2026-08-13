-- Ajout des parametres de contexte conversationnel pour l'analyse de sentiment IA
ALTER TABLE ia_config
    ADD COLUMN context_dampening     DOUBLE PRECISION NOT NULL DEFAULT 0.65,
    ADD COLUMN context_format        TEXT NOT NULL DEFAULT 'natural',
    ADD COLUMN context_max_messages  INTEGER NOT NULL DEFAULT 3,
    ADD COLUMN context_max_chars     INTEGER NOT NULL DEFAULT 200;
