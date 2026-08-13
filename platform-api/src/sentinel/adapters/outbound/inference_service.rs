use ndarray::Array2;
use ndarray::Array4;
use ort::session::Session;
use ort::value::Value;
use std::path::Path;
use std::sync::Mutex;
use std::sync::RwLock;
use tracing::info;
use tracing::warn;

pub use platform_core::sentinel::ports::outbound::ai::inference_service::{
    InferenceClassification, InferenceService as InferenceServicePort,
};

/// Service d'inference ONNX — charge les modeles au demarrage.
/// Les sessions sont protegees par Mutex car `session.run()` requiert `&mut`.
/// Le RwLock externe permet de recharger les modeles a chaud.
pub struct InferenceService {
    vision_session: RwLock<Option<Mutex<Session>>>,
    text_session: RwLock<Option<Mutex<Session>>>,
}

impl InferenceService {
    fn load_session(path: &str, label: &str) -> Option<Mutex<Session>> {
        if !Path::new(path).exists() {
            warn!(path = %path, "Modele {} ONNX introuvable — inference desactivee", label);
            return None;
        }
        let result = (|| -> Result<Session, Box<dyn std::error::Error>> {
            let builder = Session::builder()?;
            let mut builder = builder
                .with_intra_threads(4)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let session = builder.commit_from_file(path)?;
            Ok(session)
        })();
        match result {
            Ok(session) => {
                info!(path = %path, "Modele {} ONNX charge", label);
                Some(Mutex::new(session))
            }
            Err(e) => {
                warn!(error = %e, "Erreur chargement modele {} ONNX", label);
                None
            }
        }
    }

    /// Charge les modeles ONNX depuis les chemins fournis.
    /// Si un modele n'est pas trouve, le service fonctionne en mode degrade.
    pub fn new(vision_model_path: Option<&str>, text_model_path: Option<&str>) -> Self {
        let vision_session = vision_model_path.and_then(|p| Self::load_session(p, "vision"));
        let text_session = text_model_path.and_then(|p| Self::load_session(p, "text"));

        Self {
            vision_session: RwLock::new(vision_session),
            text_session: RwLock::new(text_session),
        }
    }

    /// Recharge un modele a chaud sans redemarrage.
    pub fn reload(&self, model_type: &str) -> Result<String, String> {
        match model_type {
            "text-sentiment" | "text" => {
                let path = std::env::var("TEXT_MODEL_PATH").unwrap_or_default();
                if path.is_empty() {
                    return Err("TEXT_MODEL_PATH non configure".into());
                }
                let session = Self::load_session(&path, "text");
                let loaded = session.is_some();
                *self
                    .text_session
                    .write()
                    .map_err(|e| format!("Lock error: {e}"))? = session;
                if loaded {
                    Ok(format!("Modele text recharge depuis {}", path))
                } else {
                    Err(format!("Echec rechargement text depuis {}", path))
                }
            }
            "image-classification" | "vision" => {
                let path = std::env::var("VISION_MODEL_PATH").unwrap_or_default();
                if path.is_empty() {
                    return Err("VISION_MODEL_PATH non configure".into());
                }
                let session = Self::load_session(&path, "vision");
                let loaded = session.is_some();
                *self
                    .vision_session
                    .write()
                    .map_err(|e| format!("Lock error: {e}"))? = session;
                if loaded {
                    Ok(format!("Modele vision recharge depuis {}", path))
                } else {
                    Err(format!("Echec rechargement vision depuis {}", path))
                }
            }
            _ => Err(format!("Type de modele inconnu: {}", model_type)),
        }
    }

    pub fn vision_available(&self) -> bool {
        self.vision_session
            .read()
            .map(|s| s.is_some())
            .unwrap_or(false)
    }

    pub fn text_available(&self) -> bool {
        self.text_session
            .read()
            .map(|s| s.is_some())
            .unwrap_or(false)
    }
}

impl InferenceServicePort for InferenceService {
    fn vision_available(&self) -> bool {
        InferenceService::vision_available(self)
    }
    fn text_available(&self) -> bool {
        InferenceService::text_available(self)
    }
    fn classify_image(&self, t: Array4<f32>) -> Result<Vec<InferenceClassification>, String> {
        InferenceService::classify_image(self, t)
    }
    fn classify_text(
        &self,
        ids: Array2<i64>,
        mask: Array2<i64>,
    ) -> Result<Vec<InferenceClassification>, String> {
        InferenceService::classify_text(self, ids, mask)
    }
}

impl InferenceService {
    /// Inference vision : prend une image preprocessee (1, 3, 224, 224) normalisee.
    pub fn classify_image(
        &self,
        image_tensor: Array4<f32>,
    ) -> Result<Vec<InferenceClassification>, String> {
        let guard = self
            .vision_session
            .read()
            .map_err(|e| format!("RwLock error: {e}"))?;
        let mutex = guard.as_ref().ok_or("Modele vision non charge")?;
        let mut session = mutex
            .lock()
            .map_err(|e| format!("Lock session vision: {e}"))?;

        let input =
            Value::from_array(image_tensor).map_err(|e| format!("Erreur creation tensor: {e}"))?;

        let outputs = session
            .run(ort::inputs![input])
            .map_err(|e| format!("Erreur inference vision: {e}"))?;

        let output_view = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Erreur extraction output: {e}"))?;

        let logits = output_view.1;
        let probabilities = softmax(logits);

        let labels = ["safe", "nsfw", "illicit"];
        Ok(labels
            .iter()
            .zip(probabilities.iter())
            .map(|(label, &confidence)| InferenceClassification {
                label: label.to_string(),
                confidence,
            })
            .collect())
    }

    /// Inference text : prend des token IDs et un attention mask.
    pub fn classify_text(
        &self,
        input_ids: Array2<i64>,
        attention_mask: Array2<i64>,
    ) -> Result<Vec<InferenceClassification>, String> {
        let guard = self
            .text_session
            .read()
            .map_err(|e| format!("RwLock error: {e}"))?;
        let mutex = guard.as_ref().ok_or("Modele text non charge")?;
        let mut session = mutex
            .lock()
            .map_err(|e| format!("Lock session text: {e}"))?;

        let ids_value = Value::from_array(input_ids)
            .map_err(|e| format!("Erreur creation tensor input_ids: {e}"))?;
        let mask_value = Value::from_array(attention_mask)
            .map_err(|e| format!("Erreur creation tensor attention_mask: {e}"))?;

        let outputs = session
            .run(ort::inputs![ids_value, mask_value])
            .map_err(|e| format!("Erreur inference text: {e}"))?;

        let output_view = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Erreur extraction output: {e}"))?;

        let logits = output_view.1;
        let probabilities = softmax(logits);

        // Nouveau modele 2 classes (cf. platform-ml/text/configs/train_config.yaml).
        // 0 = safe (neutral + anger + harassment leger), 1 = severe (rage + threat).
        let labels = ["safe", "severe"];
        Ok(labels
            .iter()
            .zip(probabilities.iter())
            .map(|(label, &confidence)| InferenceClassification {
                label: label.to_string(),
                confidence,
            })
            .collect())
    }
}

/// Softmax sur un slice de logits.
fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    exps.into_iter().map(|e| e / sum).collect()
}

#[cfg(test)]
#[path = "tests/inference_service.rs"]
mod tests;
