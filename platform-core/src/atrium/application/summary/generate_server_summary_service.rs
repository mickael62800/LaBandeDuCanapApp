use std::sync::Arc;

use async_trait::async_trait;

use crate::atrium::{
    domain::{ServerSummaryReply, ServerSummaryRequest, WelcomeError, WelcomePrompt},
    ports::{inbound::GenerateServerSummaryUseCase, outbound::WelcomeAiGateway},
};

/// Cas d'usage de génération du résumé / météo d'ambiance du serveur.
pub struct ServerSummaryService {
    ai: Arc<dyn WelcomeAiGateway>,
}

impl ServerSummaryService {
    pub fn new(ai: Arc<dyn WelcomeAiGateway>) -> Self {
        Self { ai }
    }
}

#[async_trait]
impl GenerateServerSummaryUseCase for ServerSummaryService {
    async fn generate_summary(
        &self,
        request: ServerSummaryRequest,
    ) -> Result<ServerSummaryReply, WelcomeError> {
        if request.guild_id.trim().is_empty() {
            return Err(WelcomeError::Missing("guild_id"));
        }

        let activity = if request.sample_activity.trim().is_empty() {
            "Aucune activité récente sur le serveur.".to_string()
        } else {
            request.sample_activity
        };

        let prompt = WelcomePrompt {
            system: "Tu es Atrium, l'assistant IA et habitué sympa du serveur Discord La Bande du Canapé.\n\
                     Fais un bref résumé 'Météo' (3-4 phrases) très fun, positif et décontracté de l'ambiance globale du serveur.\n\
                     Ne mentionne pas que tu as lu un 'échantillon', fais comme si tu avais tout suivi avec bienveillance.\n\
                     Réponds en français naturel, en tutoyant la communauté et sans formalisme.".to_string(),
            user: format!("Voici un extrait de l'activité récente :\n{}", activity),
        };

        match self.ai.generate(prompt).await {
            Ok(content) if !content.trim().is_empty() => Ok(ServerSummaryReply {
                content: content.trim().to_string(),
                generated_by_ai: true,
            }),
            _ => Ok(ServerSummaryReply {
                content: "Météo ensoleillée et ambiance au beau fixe sur le serveur ! ☀️"
                    .to_string(),
                generated_by_ai: false,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atrium::ports::outbound::AiProviderError;

    struct FakeAi(Result<String, AiProviderError>);
    #[async_trait]
    impl WelcomeAiGateway for FakeAi {
        async fn generate(&self, _: WelcomePrompt) -> Result<String, AiProviderError> {
            self.0
                .as_ref()
                .map(Clone::clone)
                .map_err(|_| AiProviderError)
        }
    }

    #[tokio::test]
    async fn falls_back_when_ai_fails() {
        let service = ServerSummaryService::new(Arc::new(FakeAi(Err(AiProviderError))));
        let res = service
            .generate_summary(ServerSummaryRequest {
                guild_id: "123".into(),
                sample_activity: "message".into(),
            })
            .await
            .unwrap();
        assert!(!res.generated_by_ai);
        assert!(res.content.contains("Météo"));
    }

    #[tokio::test]
    async fn generates_summary_successfully() {
        let service =
            ServerSummaryService::new(Arc::new(FakeAi(Ok("Excellente ambiance !".into()))));
        let res = service
            .generate_summary(ServerSummaryRequest {
                guild_id: "123".into(),
                sample_activity: "message".into(),
            })
            .await
            .unwrap();
        assert!(res.generated_by_ai);
        assert_eq!(res.content, "Excellente ambiance !");
    }

    #[tokio::test]
    async fn rejects_missing_guild_id() {
        let service = ServerSummaryService::new(Arc::new(FakeAi(Ok("ok".into()))));
        let res = service
            .generate_summary(ServerSummaryRequest {
                guild_id: "   ".into(),
                sample_activity: "message".into(),
            })
            .await;
        assert!(matches!(res, Err(WelcomeError::Missing("guild_id"))));
    }

    #[tokio::test]
    async fn uses_empty_activity_default_when_sample_is_blank() {
        let service = ServerSummaryService::new(Arc::new(FakeAi(Ok("Bonne journée!".into()))));
        let res = service
            .generate_summary(ServerSummaryRequest {
                guild_id: "123".into(),
                sample_activity: "   ".into(),
            })
            .await
            .unwrap();
        assert!(res.generated_by_ai);
        assert_eq!(res.content, "Bonne journée!");
    }

    #[tokio::test]
    async fn fallback_when_ai_returns_blank() {
        let service = ServerSummaryService::new(Arc::new(FakeAi(Ok("   ".into()))));
        let res = service
            .generate_summary(ServerSummaryRequest {
                guild_id: "123".into(),
                sample_activity: "message".into(),
            })
            .await
            .unwrap();
        assert!(!res.generated_by_ai);
        assert!(res.content.contains("Météo"));
    }

    #[tokio::test]
    async fn fallback_when_guild_empty() {
        let service = ServerSummaryService::new(Arc::new(FakeAi(Ok("text".into()))));
        let res = service
            .generate_summary(ServerSummaryRequest {
                guild_id: "".into(),
                sample_activity: "message".into(),
            })
            .await;
        assert!(matches!(res, Err(WelcomeError::Missing("guild_id"))));
    }
}
