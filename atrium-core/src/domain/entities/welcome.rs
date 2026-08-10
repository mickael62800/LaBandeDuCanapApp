use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationScope {
    General,
    Direct,
}

impl ConversationScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "salon general public",
            Self::Direct => "message prive",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WelcomeRequest {
    pub guild_id: String,
    pub member_id: String,
    pub member_display_name: String,
    pub channel_id: String,
    pub scope: ConversationScope,
    pub member_message: String,
    /// Derniers echanges avec ce membre, fournis par la memoire Atrium.
    pub conversation_history: String,
    /// Texte approuve par les administrateurs : FAQ, regles et orientation.
    pub server_context: String,
    /// Instruction de comportement configuree par serveur (`welcome_context`).
    /// Contrairement a `server_context` (des FAITS approuves), ceci ajuste le
    /// TON/la personnalite et s'injecte dans le prompt systeme. Vide = defaut.
    pub admin_context: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WelcomeReply {
    pub content: String,
    pub generated_by_ai: bool,
}

#[derive(Debug, Clone)]
pub struct WelcomePrompt {
    pub system: String,
    pub user: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WelcomeError {
    #[error("{0} est obligatoire")]
    Missing(&'static str),
    #[error("{field} depasse la limite de {limit} caracteres")]
    TooLong { field: &'static str, limit: usize },
}

impl fmt::Display for ConversationScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
