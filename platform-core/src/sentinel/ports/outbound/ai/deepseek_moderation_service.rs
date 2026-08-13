//! Port du service de moderateur IA (DeepSeek LLM).
//! remplace l'ancienne approche de classification ONNX locale.

use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeepSeekModerationAnalysis {
    pub toxicity_score: f64,
    pub sentiment: String,
    pub flags: Vec<String>,
    pub recommended_action: String,
    pub reason: String,
}

#[async_trait]
pub trait DeepSeekModerationService: Send + Sync {
    /// Indique si le service DeepSeek est configure et disponible.
    fn is_available(&self) -> bool;

    /// Analyse le contenu d'un message Discord avec son contexte conversationnel optionnel.
    async fn analyze_message(
        &self,
        content: &str,
        context: &[String],
    ) -> Result<DeepSeekModerationAnalysis, String>;
}
