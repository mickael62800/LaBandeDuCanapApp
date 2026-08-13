-- Base de connaissances Atrium : un document est decoupe en fragments courts
-- avant indexation. Les embeddings sont fournis par un service configurable.
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS atrium_knowledge_documents (
    id UUID PRIMARY KEY,
    guild_id TEXT NOT NULL,
    title TEXT NOT NULL,
    source_url TEXT,
    content_hash TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (guild_id, content_hash)
);

CREATE TABLE IF NOT EXISTS atrium_knowledge_chunks (
    id UUID PRIMARY KEY,
    document_id UUID NOT NULL REFERENCES atrium_knowledge_documents(id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    content TEXT NOT NULL CHECK (char_length(content) BETWEEN 1 AND 6000),
    -- 1536 dimensions : valeur initiale du fournisseur OpenAI-compatible.
    embedding vector(1536) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (document_id, ordinal)
);

CREATE INDEX IF NOT EXISTS idx_atrium_knowledge_documents_guild
    ON atrium_knowledge_documents (guild_id) WHERE enabled;
CREATE INDEX IF NOT EXISTS idx_atrium_knowledge_chunks_embedding
    ON atrium_knowledge_chunks USING hnsw (embedding vector_cosine_ops);
