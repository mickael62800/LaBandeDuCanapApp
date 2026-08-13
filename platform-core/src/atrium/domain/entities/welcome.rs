use std::fmt;

// Contrats métier de la réponse Atrium. Le domaine reçoit les données déjà
// contextualisées par l'API, valide les champs et construit le prompt ; il ne
// publie jamais directement sur Discord.

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
    /// Guilde Discord concernée.
    pub guild_id: String,
    /// Membre à qui la réponse est destinée.
    pub member_id: String,
    /// Nom affiché utilisé pour personnaliser la réponse.
    pub member_display_name: String,
    /// Salon Discord ou identifiant logique de conversation.
    pub channel_id: String,
    pub scope: ConversationScope,
    /// Texte fourni par le membre, limité à 1 500 caractères.
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
    /// Texte à publier par le bot.
    pub content: String,
    /// `false` si un message de secours a été utilisé.
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

#[cfg(test)]
mod tests {
    use super::ConversationScope;

    #[test]
    fn scope_labels_are_human_readable() {
        assert_eq!(ConversationScope::General.label(), "salon general public");
        assert_eq!(ConversationScope::Direct.to_string(), "message prive");
    }
}
