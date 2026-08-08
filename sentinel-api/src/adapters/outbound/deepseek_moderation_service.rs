//! Adaptateur HTTP Outbound DeepSeek Moderation pour `sentinel-api`.
//! Effectue des requetes vers l'API Cloud OpenAI-compatible de DeepSeek pour l'analyse du sentiment et de la toxicite.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

pub use sentinel_core::ports::outbound::ai::deepseek_moderation_service::{
    DeepSeekModerationAnalysis, DeepSeekModerationService,
};

const DEFAULT_DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";
const DEFAULT_MODEL: &str = "deepseek-chat";

pub struct DeepSeekModerationAdapter {
    client: Client,
    api_key: Option<String>,
    model: String,
    endpoint: String,
}

impl DeepSeekModerationAdapter {
    pub fn new(api_key: Option<String>, model: Option<String>, endpoint: Option<String>) -> Self {
        let model = model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let endpoint = endpoint.unwrap_or_else(|| DEFAULT_DEEPSEEK_URL.to_string());
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_key,
            model,
            endpoint,
        }
    }

    pub fn from_env() -> Self {
        let api_key = std::env::var("DEEPSEEK_MODERATION_API_KEY")
            .or_else(|_| std::env::var("DEEPSEEK_API_KEY"))
            .ok();
        let model = std::env::var("DEEPSEEK_MODERATION_MODEL").ok();
        let endpoint = std::env::var("DEEPSEEK_MODERATION_ENDPOINT").ok();
        Self::new(api_key, model, endpoint)
    }
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
}

#[async_trait]
impl DeepSeekModerationService for DeepSeekModerationAdapter {
    fn is_available(&self) -> bool {
        self.api_key
            .as_ref()
            .map(|k| !k.trim().is_empty())
            .unwrap_or(false)
    }

    async fn analyze_message(
        &self,
        content: &str,
        context: &[String],
    ) -> Result<DeepSeekModerationAnalysis, String> {
        let api_key = match &self.api_key {
            Some(key) if !key.trim().is_empty() => key,
            _ => return Err("DEEPSEEK_API_KEY non configuree".to_string()),
        };

        let system_prompt = r#"Tu es un expert en modération de chat Discord.
Ton rôle est d'analyser la toxicité, le discours de haine, la colère et le harcèlement dans les messages.
Analyse le message de l'utilisateur (ainsi que le contexte conversationnel si fourni).
Réponds EXCLUSIVEMENT avec un objet JSON valide structuré exactement comme suit :
{
  "toxicity_score": <float de 0.0 a 1.0>,
  "sentiment": "<hate|anger|harassment|spam|nsfw|neutral>",
  "flags": ["<liste des regles violees ex: hate_speech, aggressive, harassment>"],
  "recommended_action": "<none|warn|delete|mute|ban>",
  "reason": "<explication tres courte et claire en francais pour les moderateurs>"
}"#;

        let user_content = if context.is_empty() {
            format!("Message à analyser: \"{}\"", content)
        } else {
            format!(
                "Contexte récent de la discussion:\n{}\n\nMessage à analyser: \"{}\"",
                context.join("\n"),
                content
            )
        };

        let payload = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_content,
                },
            ],
            temperature: 0.1,
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
        };

        debug!(endpoint = %self.endpoint, model = %self.model, "Envoi requete DeepSeek Moderation...");

        let res = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau DeepSeek: {e}"))?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            warn!(status = %status, err = %err_text, "Refus API DeepSeek");
            return Err(format!("DeepSeek API Error HTTP {status}: {err_text}"));
        }

        let body: ChatCompletionResponse = res
            .json()
            .await
            .map_err(|e| format!("Erreur deserialisation reponse DeepSeek: {e}"))?;

        let text_res = body
            .choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .ok_or_else(|| "Reponse DeepSeek vide".to_string())?;

        let analysis: DeepSeekModerationAnalysis = serde_json::from_str(text_res)
            .map_err(|e| format!("Parsing JSON DeepSeek echoue: {e} (Raw: {text_res})"))?;

        Ok(analysis)
    }
}
