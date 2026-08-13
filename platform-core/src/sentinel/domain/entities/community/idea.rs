use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// Une idee proposee par un membre du serveur. Le staff en discute avec
/// l'auteur dans un salon prive dedie, puis tranche.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idea {
    pub id: Uuid,
    pub guild_id: String,
    pub title: String,
    pub description: String,
    /// Voir `IdeaStatus` : persiste en texte pour rester lisible en base.
    pub status: String,
    pub category: String,
    pub author_id: String,
    pub author_name: String,
    /// Salon prive dedie. `None` si sa creation a echoue cote Discord.
    pub channel_id: Option<String>,
    pub decided_by: Option<String>,
    pub decided_by_name: Option<String>,
    pub decision_reason: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Un message echange dans le salon d'une idee, synchronise par le bot pour
/// etre relu depuis le web.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaMessage {
    pub id: Uuid,
    pub idea_id: Uuid,
    pub author_name: String,
    /// "auteur" ou "staff".
    pub author_role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaDetail {
    pub idea: Idea,
    pub messages: Vec<IdeaMessage>,
}
