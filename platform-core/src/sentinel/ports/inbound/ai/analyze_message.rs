use async_trait::async_trait;

use crate::sentinel::domain::entities::ai::message_analysis::MessageAnalysis;
use crate::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::sentinel::domain::entities::system::discord_ids::ChannelId;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::MessageId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use crate::sentinel::domain::errors::DomainError;

/// Entree de contexte conversationnel (message precedent dans le canal).
pub struct ContextMessageEntry {
    pub username: String,
    pub content: String,
}

pub struct AnalyzeMessageCommand {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub username: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub message_id: MessageId,
    pub timestamp: String,
    /// Messages de contexte conversationnel pour l'analyse de sentiment.
    pub context_messages: Vec<ContextMessageEntry>,
}

/// Decision d'auto-protection face a un flood, prise cote serveur a partir
/// de la config guild (`auto_protect_enabled`, `severe_flood_max_messages`).
/// Le bot detecte le flood (tracker rate en memoire, legitime) puis demande
/// le verdict ici au lieu de comparer a un seuil local.
pub struct FloodDecision {
    /// True si une protection automatique (mute + suppression) doit s'appliquer.
    pub severe: bool,
    /// Duree du mute a appliquer si `severe` (secondes).
    pub mute_duration_secs: i64,
    /// Score de confiance a afficher sur la carte de review (0.0..1.0). Fabrique
    /// cote serveur — le bot ne l'invente plus.
    pub score: f64,
}

/// Decision d'analyse d'une piece jointe suspecte, prise cote serveur. La regle
/// (liste des extensions dangereuses + `suspicious_file_extensions` config +
/// toggle `suspicious_files_enabled`) vit dans le core ; le bot envoie les noms
/// de fichiers et n'EXECUTE que l'action renvoyee.
pub struct AttachmentDecision {
    /// True si au moins une piece jointe est jugee suspecte.
    pub suspicious: bool,
    /// Action arbitree (Delete si suspecte, None sinon).
    pub action: crate::sentinel::domain::enums::moderation::action::Action,
    /// Raison lisible (inclut le nom du fichier fautif).
    pub reason: String,
    /// Score de confiance a afficher sur la carte de review.
    pub score: f64,
    /// Nom du fichier fautif (vide si aucun).
    pub filename: String,
}

/// Decision de score pour une detection de CAPS (majuscules), prise cote
/// serveur. La detection (forme/longueur) reste locale au bot ; seul le SCORE
/// de confiance affiche vient d'ici — le bot ne le fabrique plus (avant : 0.8
/// code en dur cote bot).
pub struct CapsDecision {
    /// Score de confiance a afficher sur la carte de review / l'embed (0.0..1.0).
    pub score: f64,
}

#[async_trait]
pub trait AnalyzeMessageUseCase: Send + Sync {
    async fn analyze(&self, command: AnalyzeMessageCommand)
        -> Result<MessageAnalysis, DomainError>;

    /// Evalue un signal de flood (nombre de messages dans la fenetre) et
    /// renvoie la decision d'auto-protection. La regle (seuil severe, toggle)
    /// vit cote serveur, pas dans le bot.
    async fn evaluate_flood(
        &self,
        guild_id: &str,
        flood_count: i32,
    ) -> Result<FloodDecision, DomainError>;

    /// Evalue une liste de pieces jointes (noms de fichiers) et renvoie la
    /// decision. La regle (extensions dangereuses + config) vit cote serveur.
    async fn evaluate_attachments(
        &self,
        guild_id: &str,
        filenames: Vec<String>,
    ) -> Result<AttachmentDecision, DomainError>;

    /// Renvoie le score de confiance a afficher pour une detection de CAPS.
    /// La detection reste locale au bot ; le SCORE affiche vit cote serveur.
    async fn evaluate_caps(&self, guild_id: &str) -> Result<CapsDecision, DomainError>;
}
