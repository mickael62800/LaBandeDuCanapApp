//! Entites de la file d'attente des jobs IA (`ai_jobs`). Un bot enqueue un job
//! (analyse texte/image), l'ai-worker le depile et ecrit le resultat.

/// Commande de creation d'un job IA (deja validee par le use case).
#[derive(Debug, Clone)]
pub struct NewAiJob {
    pub guild_id: String,
    /// "analyze_text" ou "analyze_image".
    pub job_type: String,
    pub input_payload: serde_json::Value,
}

/// Etat courant d'un job IA (ligne de `ai_jobs`).
#[derive(Debug, Clone)]
pub struct AiJob {
    pub id: uuid::Uuid,
    pub guild_id: String,
    pub job_type: String,
    pub status: String,
    pub input_payload: serde_json::Value,
    pub result_payload: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub retries: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}
