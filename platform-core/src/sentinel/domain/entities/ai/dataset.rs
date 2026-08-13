//! Entites du dataset d'entrainement IA : messages utilisateurs collectes et
//! filtres (guild, salon, plage de dates, longueur mini) pour l'export.

/// Un message du dataset (ligne de `ai_dataset_messages`).
#[derive(Debug, Clone)]
pub struct DatasetMessage {
    pub id: String,
    pub user_id: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub content: String,
    /// Horodatage ISO8601 (`YYYY-MM-DDTHH:MM:SSZ`).
    pub created_at: String,
}

/// Filtres de listing (deja bornes/normalises par le use case).
#[derive(Debug, Clone)]
pub struct DatasetQuery {
    pub guild_id: String,
    pub channel_id: Option<String>,
    /// Bornes ISO8601 optionnelles.
    pub from: Option<String>,
    pub to: Option<String>,
    /// Longueur minimale de contenu (>= 0).
    pub min_length: i64,
    /// Nombre max de lignes (1..=1000).
    pub limit: i64,
    /// Decalage de pagination (>= 0).
    pub offset: i64,
}

/// Page de resultats + total (sans filtre de pagination).
#[derive(Debug, Clone)]
pub struct DatasetPage {
    pub items: Vec<DatasetMessage>,
    pub total: i64,
}
