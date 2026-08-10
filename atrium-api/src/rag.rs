//! RAG vectoriel Atrium : indexe les documents approuves et interroge pgvector.

use std::sync::Arc;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::AppConfig;

const MAX_RESULTS: i64 = 4;
const MIN_SIMILARITY: f64 = 0.35;
const CHUNK_CHARS: usize = 1_800;

struct KnowledgeDocument {
    title: &'static str,
    source: &'static str,
    content: &'static str,
}

const KNOWLEDGE: &[KnowledgeDocument] = &[
    KnowledgeDocument {
        title: "Règlement — La Bande du Canapé",
        source: "knowledge://Reglement_La_Bande_du_Canape.md",
        content: include_str!("../knowledge/Reglement_La_Bande_du_Canape.md"),
    },
    KnowledgeDocument {
        title: "Les règles du canapé",
        source: "knowledge://les-regles-du-canape.md",
        content: include_str!("../knowledge/les-regles-du-canape.md"),
    },
    KnowledgeDocument {
        title: "S.O.S canapé",
        source: "knowledge://sos-canape.md",
        content: include_str!("../knowledge/sos-canape.md"),
    },
    KnowledgeDocument {
        title: "Créer ton canapé",
        source: "knowledge://cree-ton-canape.md",
        content: include_str!("../knowledge/cree-ton-canape.md"),
    },
    KnowledgeDocument {
        title: "Canapé vocal",
        source: "knowledge://canape-vocal.md",
        content: include_str!("../knowledge/canape-vocal.md"),
    },
    KnowledgeDocument {
        title: "Autour du canapé",
        source: "knowledge://autour-du-canape.md",
        content: include_str!("../knowledge/autour-du-canape.md"),
    },
    KnowledgeDocument {
        title: "Choisis ton jeu",
        source: "knowledge://choisis-ton-jeu.md",
        content: include_str!("../knowledge/choisis-ton-jeu.md"),
    },
    KnowledgeDocument {
        title: "Table de blackjack",
        source: "knowledge://table-de-blackjack.md",
        content: include_str!("../knowledge/table-de-blackjack.md"),
    },
    KnowledgeDocument {
        title: "La roue du canapé",
        source: "knowledge://la-roue-du-canape.md",
        content: include_str!("../knowledge/la-roue-du-canape.md"),
    },
    KnowledgeDocument {
        title: "Le coussin piège",
        source: "knowledge://le-coussin-piege.md",
        content: include_str!("../knowledge/le-coussin-piege.md"),
    },
    KnowledgeDocument {
        title: "Propulseur du canapé",
        source: "knowledge://propulseur-du-canape.md",
        content: include_str!("../knowledge/propulseur-du-canape.md"),
    },
    KnowledgeDocument {
        title: "Cherche des potes",
        source: "knowledge://cherche-des-potes.md",
        content: include_str!("../knowledge/cherche-des-potes.md"),
    },
    KnowledgeDocument {
        title: "Le coin des guides",
        source: "knowledge://le-coin-des-guides.md",
        content: include_str!("../knowledge/le-coin-des-guides.md"),
    },
    KnowledgeDocument {
        title: "Les bonnes idées",
        source: "knowledge://les-bonnes-idees.md",
        content: include_str!("../knowledge/les-bonnes-idees.md"),
    },
    KnowledgeDocument {
        title: "Les nouvelles du canapé",
        source: "knowledge://les-nouvelles-du-canape.md",
        content: include_str!("../knowledge/les-nouvelles-du-canape.md"),
    },
    KnowledgeDocument {
        title: "Confessions",
        source: "knowledge://confessions.md",
        content: include_str!("../knowledge/confessions.md"),
    },
    KnowledgeDocument {
        title: "Album du canapé",
        source: "knowledge://album-du-canape.md",
        content: include_str!("../knowledge/album-du-canape.md"),
    },
    KnowledgeDocument {
        title: "Lofi music",
        source: "knowledge://lofi-music.md",
        content: include_str!("../knowledge/lofi-music.md"),
    },
    KnowledgeDocument {
        title: "Sieste AFK",
        source: "knowledge://sieste-afk.md",
        content: include_str!("../knowledge/sieste-afk.md"),
    },
];

#[derive(Clone)]
pub struct RagService {
    pool: PgPool,
    embeddings: EmbeddingsClient,
}

/// Un document INDEXE en base, vu par l'administration.
///
/// A ne pas confondre avec `KnowledgeDocument` plus haut, qui decrit les
/// fichiers Markdown embarques dans le binaire *avant* indexation.
///
/// Le contenu n'est PAS renvoye : la page liste ce qu'Atrium sait, elle n'est
/// pas une visionneuse de documents. `chunk_count` a zero signale un document
/// enregistre mais jamais vectorise — donc invisible pour les reponses.
#[derive(Debug, FromRow, Serialize)]
pub struct IndexedDocument {
    pub id: Uuid,
    pub title: String,
    pub source_url: Option<String>,
    pub enabled: bool,
    pub chunk_count: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl RagService {
    /// Documents indexes pour une guilde, les plus recemment modifies d'abord.
    pub async fn documents(&self, guild_id: &str) -> Result<Vec<IndexedDocument>, sqlx::Error> {
        sqlx::query_as::<_, IndexedDocument>(
            "SELECT d.id, d.title, d.source_url, d.enabled, \
                    COUNT(c.id)::bigint AS chunk_count, d.updated_at \
             FROM atrium_knowledge_documents d \
             LEFT JOIN atrium_knowledge_chunks c ON c.document_id = d.id \
             WHERE d.guild_id = $1 \
             GROUP BY d.id \
             ORDER BY d.updated_at DESC \
             LIMIT 200",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
    }

    pub fn new(pool: PgPool, config: &AppConfig) -> Self {
        Self {
            pool,
            embeddings: EmbeddingsClient::new(config),
        }
    }

    pub async fn index_knowledge(&self) -> Result<(), String> {
        for document in KNOWLEDGE {
            let hash = content_hash(document.content);
            let existing = sqlx::query_as::<_, ExistingDocument>(
                "SELECT d.id, d.content_hash, COUNT(c.id)::bigint AS chunk_count \
                 FROM atrium_knowledge_documents d LEFT JOIN atrium_knowledge_chunks c ON c.document_id = d.id \
                 WHERE d.source_url = $1 GROUP BY d.id, d.content_hash LIMIT 1",
            )
            .bind(document.source)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| error.to_string())?;

            if existing
                .as_ref()
                .is_some_and(|item| item.content_hash == hash && item.chunk_count > 0)
            {
                continue;
            }
            let id = existing
                .as_ref()
                .map(|item| item.id)
                .unwrap_or_else(Uuid::new_v4);
            let chunks = split_chunks(document.content);
            let vectors = self.embeddings.embed_many(&chunks).await?;
            if vectors.len() != chunks.len() {
                return Err("nombre d'embeddings invalide".into());
            }

            sqlx::query(
                "INSERT INTO atrium_knowledge_documents (id, guild_id, title, source_url, content_hash) \
                 VALUES ($1, '*', $2, $3, $4) \
                 ON CONFLICT (id) DO UPDATE SET title = EXCLUDED.title, source_url = EXCLUDED.source_url, \
                 content_hash = EXCLUDED.content_hash, enabled = TRUE, updated_at = now()",
            )
            .bind(id).bind(document.title).bind(document.source).bind(&hash).execute(&self.pool).await
            .map_err(|error| error.to_string())?;
            sqlx::query("DELETE FROM atrium_knowledge_chunks WHERE document_id = $1")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(|error| error.to_string())?;
            for (ordinal, (content, embedding)) in chunks.iter().zip(vectors.iter()).enumerate() {
                sqlx::query("INSERT INTO atrium_knowledge_chunks (id, document_id, ordinal, content, embedding) VALUES ($1, $2, $3, $4, $5::vector)")
                    .bind(Uuid::new_v4()).bind(id).bind(ordinal as i32).bind(content).bind(vector_literal(embedding))
                    .execute(&self.pool).await.map_err(|error| error.to_string())?;
            }
            tracing::info!(
                source = document.source,
                chunks = chunks.len(),
                "Document RAG indexe"
            );
        }
        Ok(())
    }

    pub async fn context_for(&self, guild_id: &str, question: &str) -> Result<String, String> {
        if question.trim().is_empty() {
            return Ok(String::new());
        }
        let vector = vector_literal(&self.embeddings.embed(question).await?);
        let results = sqlx::query_as::<_, SearchResult>(
            "SELECT c.content, d.title, d.source_url, 1 - (c.embedding <=> $1::vector) AS similarity \
             FROM atrium_knowledge_chunks c JOIN atrium_knowledge_documents d ON d.id = c.document_id \
             WHERE d.enabled AND d.guild_id IN ($2, '*') ORDER BY c.embedding <=> $1::vector LIMIT $3",
        )
        .bind(vector).bind(guild_id).bind(MAX_RESULTS).fetch_all(&self.pool).await
        .map_err(|error| error.to_string())?;
        Ok(results
            .into_iter()
            .filter(|item| item.similarity >= MIN_SIMILARITY)
            .map(|item| {
                format!(
                    "## Source approuvée : {} ({})\n{}",
                    item.title,
                    item.source_url.unwrap_or_default(),
                    item.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n"))
    }

    pub async fn search_chunks(
        &self,
        guild_id: &str,
        question: &str,
        limit: u32,
    ) -> Result<Vec<(String, String, f64)>, String> {
        if question.trim().is_empty() {
            return Ok(vec![]);
        }
        let vector = vector_literal(&self.embeddings.embed(question).await?);
        let max = if limit == 0 {
            MAX_RESULTS
        } else {
            i64::from(limit).min(20)
        };
        let results = sqlx::query_as::<_, SearchResult>(
            "SELECT c.content, d.title, d.source_url, 1 - (c.embedding <=> $1::vector) AS similarity \
             FROM atrium_knowledge_chunks c JOIN atrium_knowledge_documents d ON d.id = c.document_id \
             WHERE d.enabled AND d.guild_id IN ($2, '*') ORDER BY c.embedding <=> $1::vector LIMIT $3",
        )
        .bind(vector).bind(guild_id).bind(max).fetch_all(&self.pool).await
        .map_err(|error| error.to_string())?;

        Ok(results
            .into_iter()
            .filter(|item| item.similarity >= MIN_SIMILARITY)
            .map(|item| {
                (
                    item.source_url.unwrap_or_default(),
                    item.content,
                    item.similarity,
                )
            })
            .collect())
    }
}

pub fn service(config: &AppConfig) -> Result<Arc<RagService>, sqlx::Error> {
    Ok(Arc::new(RagService::new(
        PgPool::connect_lazy(&config.rag_database_url)?,
        config,
    )))
}

#[derive(FromRow)]
struct ExistingDocument {
    id: Uuid,
    content_hash: String,
    chunk_count: i64,
}
#[derive(FromRow)]
struct SearchResult {
    content: String,
    title: String,
    source_url: Option<String>,
    similarity: f64,
}

#[derive(Clone)]
struct EmbeddingsClient {
    client: Client,
    url: String,
    api_key: Option<String>,
    model: String,
}
#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: &'a [String],
}
#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}
#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}
impl EmbeddingsClient {
    fn new(config: &AppConfig) -> Self {
        Self {
            client: Client::new(),
            url: format!(
                "{}/embeddings",
                config.embeddings_base_url.trim_end_matches('/')
            ),
            api_key: config.embeddings_api_key.clone(),
            model: config.embeddings_model.clone(),
        }
    }
    async fn embed(&self, input: &str) -> Result<Vec<f32>, String> {
        Ok(self
            .embed_many(&[input.to_owned()])
            .await?
            .pop()
            .unwrap_or_default())
    }
    async fn embed_many(&self, input: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let mut request = self.client.post(&self.url).json(&EmbeddingRequest {
            model: &self.model,
            input,
        });
        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key);
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("Ollama indisponible: {error}"))?;
        if !response.status().is_success() {
            return Err(format!("Ollama embeddings: {}", response.status()));
        }
        let payload: EmbeddingResponse =
            response.json().await.map_err(|error| error.to_string())?;
        if payload.data.len() != input.len() {
            return Err("Ollama n'a pas retourne tous les embeddings demandes".into());
        }
        if payload.data.iter().any(|item| item.embedding.len() != 768) {
            return Err("le modele d'embeddings doit produire 768 dimensions".into());
        }
        Ok(payload
            .data
            .into_iter()
            .map(|item| item.embedding)
            .collect())
    }
}

fn vector_literal(values: &[f32]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn split_chunks(value: &str) -> Vec<String> {
    let chars: Vec<char> = value.chars().collect();
    chars
        .chunks(CHUNK_CHARS)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}

fn content_hash(value: &str) -> String {
    // FNV-1a : empreinte stable suffisante pour savoir si une source embarquee
    // doit etre re-indexee. Ce n'est pas un mecanisme de securite.
    let hash = value.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}
