use async_trait::async_trait;

use crate::atrium::domain::WelcomePrompt;

#[derive(Debug, thiserror::Error)]
#[error("le fournisseur IA est indisponible")]
pub struct AiProviderError;

/// Passerelle de chat générique (prompt système + utilisateur → texte).
///
/// Le nom reste « welcome » pour raisons d'historique, mais elle sert aussi
/// l'apaisement (`CalmingService`) : les deux cas d'usage n'ont besoin que d'un
/// aller-retour prompt→réponse, et dupliquer l'adaptateur DeepSeek pour le
/// second coûterait plus que ce nom imparfait.
#[async_trait]
pub trait WelcomeAiGateway: Send + Sync {
    async fn generate(&self, prompt: WelcomePrompt) -> Result<String, AiProviderError>;
}
