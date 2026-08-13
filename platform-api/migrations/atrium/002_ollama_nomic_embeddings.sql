-- Migration distincte : ne jamais modifier une migration deja appliquee.
-- Les anciens chunks sont recrees au boot depuis `knowledge/` avec Ollama.
DELETE FROM atrium_knowledge_chunks;
DROP INDEX IF EXISTS idx_atrium_knowledge_chunks_embedding;
ALTER TABLE atrium_knowledge_chunks
    ALTER COLUMN embedding TYPE vector(768);
CREATE INDEX idx_atrium_knowledge_chunks_embedding
    ON atrium_knowledge_chunks USING hnsw (embedding vector_cosine_ops);
