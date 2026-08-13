//! Décisions pures du bot, testables sans connexion Discord.
//!
//! Le bot répond aux messages privés et aux mentions dans le salon général
//! configuré. Les autres messages sont ignorés afin qu'Atrium ne se comporte
//! pas comme un chatbot global par défaut.

use platform_proto::atrium::welcome::v1::ConversationScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageHandling {
    Ignore,
    Reply(ConversationScope),
}

/// Atrium repond aux MP et aux mentions dans le seul salon general configure.
/// Cette regle empeche qu'il se comporte comme un chatbot dans tout le serveur.
pub fn message_handling(
    is_direct_message: bool,
    is_general: bool,
    is_mentioned: bool,
) -> MessageHandling {
    if is_direct_message {
        MessageHandling::Reply(ConversationScope::Direct)
    } else if is_general && is_mentioned {
        MessageHandling::Reply(ConversationScope::General)
    } else {
        MessageHandling::Ignore
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies_to_every_direct_message() {
        assert_eq!(
            message_handling(true, false, false),
            MessageHandling::Reply(ConversationScope::Direct)
        );
    }

    #[test]
    fn replies_only_when_mentioned_in_general() {
        assert_eq!(
            message_handling(false, true, true),
            MessageHandling::Reply(ConversationScope::General)
        );
        assert_eq!(
            message_handling(false, true, false),
            MessageHandling::Ignore
        );
    }

    #[test]
    fn ignores_other_guild_channels_even_when_mentioned() {
        assert_eq!(
            message_handling(false, false, true),
            MessageHandling::Ignore
        );
    }
}
