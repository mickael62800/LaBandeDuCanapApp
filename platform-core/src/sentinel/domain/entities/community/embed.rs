//! Embed builder (style Carl-bot) : une carte Discord entierement configurable
//! (author, titre, description, couleur, image, thumbnail, footer, champs),
//! sauvegardee par nom, postable dans un salon puis editable a la volee.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Un champ d'embed (bloc name/value, cote a cote si `inline`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[derive(Debug, Clone)]
pub struct Embed {
    pub id: Uuid,
    pub guild_id: String,
    pub name: String,
    /// Message texte affiche AU-DESSUS de la carte (hors embed). Optionnel.
    pub content: String,
    // Author (en-tete de la carte).
    pub author_name: String,
    pub author_icon_url: String,
    pub author_url: String,
    // Corps.
    pub title: String,
    pub title_url: String,
    pub description: String,
    /// Couleur de la barre laterale (0xRRGGBB). None = couleur Discord par defaut.
    pub color: Option<i32>,
    pub image_url: String,
    pub thumbnail_url: String,
    // Footer.
    pub footer_text: String,
    pub footer_icon_url: String,
    pub show_timestamp: bool,
    pub fields: Vec<EmbedField>,
    // Dernier message poste (pour l'edition).
    pub last_channel_id: Option<String>,
    pub last_message_id: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Embed {
    /// Un embed doit avoir au moins un contenu visible : titre, description,
    /// author, image ou au moins un champ. Sinon Discord refuse l'envoi.
    pub fn has_visible_content(&self) -> bool {
        !self.title.trim().is_empty()
            || !self.description.trim().is_empty()
            || !self.author_name.trim().is_empty()
            || !self.image_url.trim().is_empty()
            || !self.content.trim().is_empty()
            || self.fields.iter().any(|f| !f.name.trim().is_empty())
    }
}

/// Payload envoye au bot (via le stream `sentinel:events`) pour poster ou
/// editer un embed. Toutes les valeurs sont deja pretes a l'emploi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedEmbedPost {
    pub embed_id: Uuid,
    pub guild_id: String,
    /// Salon cible (pour un post) ou salon du message a editer.
    pub channel_id: String,
    /// Present => on EDITE ce message. Absent => on POSTE un nouveau message.
    #[serde(default)]
    pub message_id: Option<String>,
    pub content: String,
    pub author_name: String,
    pub author_icon_url: String,
    pub author_url: String,
    pub title: String,
    pub title_url: String,
    pub description: String,
    pub color: Option<i32>,
    pub image_url: String,
    pub thumbnail_url: String,
    pub footer_text: String,
    pub footer_icon_url: String,
    pub show_timestamp: bool,
    pub fields: Vec<EmbedField>,
}

impl RenderedEmbedPost {
    pub fn from_embed(e: &Embed, channel_id: String, message_id: Option<String>) -> Self {
        Self {
            embed_id: e.id,
            guild_id: e.guild_id.clone(),
            channel_id,
            message_id,
            content: e.content.clone(),
            author_name: e.author_name.clone(),
            author_icon_url: e.author_icon_url.clone(),
            author_url: e.author_url.clone(),
            title: e.title.clone(),
            title_url: e.title_url.clone(),
            description: e.description.clone(),
            color: e.color,
            image_url: e.image_url.clone(),
            thumbnail_url: e.thumbnail_url.clone(),
            footer_text: e.footer_text.clone(),
            footer_icon_url: e.footer_icon_url.clone(),
            show_timestamp: e.show_timestamp,
            fields: e.fields.clone(),
        }
    }
}
