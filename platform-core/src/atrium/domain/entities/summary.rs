//! Entités du résumé d'activité / météo d'ambiance du serveur.

#[derive(Debug, Clone)]
pub struct ServerSummaryRequest {
    pub guild_id: String,
    pub sample_activity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSummaryReply {
    pub content: String,
    pub generated_by_ai: bool,
}
