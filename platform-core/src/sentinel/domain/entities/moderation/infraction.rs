use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::sentinel::domain::entities::system::discord_ids::ChannelId;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::MessageId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use crate::sentinel::domain::enums::moderation::action::Action;
#[derive(Debug, Clone)]
pub struct Infraction {
    /// Identifiant stable reliant detection, review, sanction et audit.
    pub id: Uuid,
    /// Serveur dans lequel l'incident a ete observe.
    pub guild_id: GuildId,
    /// Salon source de l'incident.
    pub channel_id: ChannelId,
    /// Utilisateur vise par la detection.
    pub user_id: UserId,
    pub username: String,
    /// Pseudo serveur (nickname) si l user en a un. Lu via LEFT JOIN
    /// `guild_members.display_name`. Optionnel : null si l user n'est plus
    /// dans la guild ou n'a pas de nickname.
    pub display_name: Option<String>,
    /// Message ayant declenche la detection.
    pub message_id: MessageId,
    /// Contenu analyse et affiche dans la review.
    pub content: String,
    /// Signaux detectes ; ils ne constituent pas seuls la decision de sanction.
    pub flags: DetectionFlags,
    /// Score de risque servant a prioriser et suggerer une action.
    pub score: f64,
    /// Action suggeree ou appliquee selon le cas d'usage.
    pub action: Action,
    /// Explication lisible de la detection.
    pub reason: String,
    /// Duree en secondes pour les actions temporaires.
    pub duration: Option<u64>,
    pub created_at: DateTime<Utc>,
}
