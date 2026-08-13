use async_trait::async_trait;
use serde::Deserialize;

use crate::sentinel::domain::entities::system::channel_access::ChannelOverwrite;
use crate::sentinel::domain::errors::DomainError;

/// Trait pour les appels a l'API Discord. Permet de mocker le service
/// dans les tests d'integration HTTP sans taper la vraie API.
#[async_trait]
pub trait DiscordApi: Send + Sync {
    async fn list_text_channels(&self, guild_id: &str) -> Result<Vec<DiscordChannel>, DomainError>;
    /// Liste tous les salons utiles d'une guild (texte + voice + stage),
    /// chacun annote avec son `kind`. Utilise par les pickers config qui
    /// s'appliquent aux deux types (xp_channel_multipliers).
    async fn list_all_channels(&self, guild_id: &str) -> Result<Vec<DiscordChannel>, DomainError>;
    /// Liste les emojis custom du serveur.
    async fn list_emojis(&self, _guild_id: &str) -> Result<Vec<DiscordEmoji>, DomainError> {
        Err(DomainError::Internal(
            "Liste des emojis non supportee".into(),
        ))
    }
    /// Cree un salon (ou une categorie) et renvoie son ID Discord.
    ///
    /// Implementation par defaut en erreur : la plupart des doubles de test ne
    /// touchent jamais a la creation de salons, et leur imposer un stub vide
    /// n'apporterait rien. L'adaptateur reel surcharge.
    async fn create_channel(
        &self,
        _guild_id: &str,
        _spec: &NewChannel<'_>,
    ) -> Result<String, DomainError> {
        Err(DomainError::Internal(
            "Creation de salon non supportee par cet adaptateur Discord".into(),
        ))
    }
    /// Supprime un salon (ou une categorie). Voir la note ci-dessus pour le
    /// defaut.
    async fn delete_channel(&self, _channel_id: &str) -> Result<(), DomainError> {
        Err(DomainError::Internal(
            "Suppression de salon non supportee par cet adaptateur Discord".into(),
        ))
    }
    /// Liste les roles du serveur EN DIRECT depuis Discord.
    ///
    /// A ne pas confondre avec le repository `discord_role` (Postgres), qui
    /// sert un cache synchronise : pour composer des permissions on veut
    /// l'etat reel du serveur, y compris un role cree il y a dix secondes.
    async fn list_roles(&self, _guild_id: &str) -> Result<Vec<DiscordRoleInfo>, DomainError> {
        Err(DomainError::Internal(
            "Liste des roles non supportee par cet adaptateur Discord".into(),
        ))
    }
    async fn upload_emoji(
        &self,
        guild_id: &str,
        name: &str,
        image_bytes: &[u8],
        mime: &str,
    ) -> Result<(String, String, bool), DomainError>;
    async fn ban_user(
        &self,
        guild_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<(), DomainError>;
    async fn list_members(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<DiscordMember>, DomainError>;
    async fn send_dm(&self, user_id: &str, content: &str) -> Result<(), DomainError>;
    /// Poste un embed dans un salon.
    ///
    /// Existe pour que les handlers cessent de fabriquer un `reqwest::Client`
    /// et d'appeler `discord.com` eux-memes : un adaptateur inbound qui fait de
    /// l'I/O sortante court-circuite le port, donc l'inversion de dependance.
    ///
    /// L'implementation reelle VALIDE `channel_id` comme snowflake avant de
    /// l'interpoler dans l'URL. Cette garantie appartient a l'adaptateur, pas
    /// a l'appelant : c'est lui qui construit l'URL, donc lui qui doit
    /// empecher qu'un identifiant malforme atteigne un autre endpoint de l'API
    /// Discord avec le token du bot.
    ///
    /// Implementation par defaut en erreur : les doubles de test qui ne postent
    /// rien n'ont pas a la stubber.
    async fn send_channel_embed(
        &self,
        _channel_id: &str,
        _embed: serde_json::Value,
    ) -> Result<(), DomainError> {
        Err(DomainError::Internal(
            "Envoi d'embed non supporte par cet adaptateur Discord".into(),
        ))
    }
    async fn create_role(
        &self,
        guild_id: &str,
        name: &str,
        color: u32,
        permissions: Option<&str>,
    ) -> Result<serde_json::Value, DomainError>;
    async fn edit_role(
        &self,
        guild_id: &str,
        role_id: &str,
        name: Option<&str>,
        color: Option<u32>,
        permissions: Option<&str>,
        mentionable: Option<bool>,
        hoist: Option<bool>,
    ) -> Result<serde_json::Value, DomainError>;
    async fn delete_role(&self, guild_id: &str, role_id: &str) -> Result<(), DomainError>;
    async fn unban_user(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn remove_timeout(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn apply_timeout(
        &self,
        guild_id: &str,
        user_id: &str,
        duration_seconds: u64,
    ) -> Result<(), DomainError>;
    async fn get_user_guilds(&self, access_token: &str) -> Result<Vec<UserGuild>, DomainError>;
    async fn get_user_me(&self, access_token: &str) -> Result<DiscordUser, DomainError>;
}

/// Salon a creer, exprime dans les termes du domaine. Le `kind` est deja la
/// valeur numerique attendue par Discord (cf.
/// `PlannedChannelKind::discord_type`) : c'est la seule notion Discord que le
/// port laisse passer, le reste (overwrites de permission pour `private`,
/// forme du corps HTTP) appartient a l'adaptateur.
#[derive(Debug, Clone)]
pub struct NewChannel<'a> {
    pub name: &'a str,
    pub kind: u8,
    /// Categorie parente (ID Discord), si le salon doit y etre range.
    pub parent_id: Option<&'a str>,
    pub topic: Option<&'a str>,
    pub slowmode: u32,
    pub user_limit: Option<u32>,
    pub nsfw: bool,
    /// Overwrites de permission a poser des la creation, deja calcules par le
    /// domaine (`channel_access`) : l'adaptateur ne fait que les serialiser.
    pub overwrites: &'a [ChannelOverwrite],
}

/// Role du serveur, lu en direct depuis Discord. Sous-ensemble utile a la
/// composition de permissions : `managed` marque les roles pilotes par une
/// integration (bots), qu'on n'attribue pas a la main.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DiscordRoleInfo {
    pub id: String,
    pub name: String,
    pub color: u32,
    pub position: i64,
    #[serde(default)]
    pub managed: bool,
}

#[derive(Debug, serde::Serialize, Deserialize, Clone)]
pub struct DiscordEmoji {
    pub id: String,
    pub name: String,
    pub animated: bool,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DiscordMember {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Phase 2 B — Subset des champs Discord renvoyes par GET /users/@me/guilds
/// dont on a besoin pour l'auth multi-tenant. On capture juste l'id pour
/// minimiser la deserialization (Discord renvoie name/icon/permissions etc.).
#[derive(Debug, Clone, Deserialize)]
pub struct UserGuild {
    pub id: String,
}

/// Phase 7 B — Info minimal d'un user Discord recupere via `/users/@me`.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub avatar: Option<String>,
    /// Pseudo d'affichage global (Discord 2023). Absent des vieux comptes,
    /// d'ou le `default`. Le flux OAuth le renvoie au front pour afficher le
    /// nom que le membre a reellement choisi.
    #[serde(default)]
    pub global_name: Option<String>,
}

/// Phase 9 Part E — Salon d'une guild (pour channel picker web).
/// `kind` : "text" | "announcement" | "voice" | "stage". Permet aux
/// pickers web d'afficher l'icone correcte (# pour le texte, 🔊 pour le
/// voice) et sert aussi de filtre.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscordChannel {
    pub id: String,
    pub name: String,
    pub position: i64,
    #[serde(default = "default_text_kind")]
    pub kind: String,
}

fn default_text_kind() -> String {
    "text".to_string()
}
