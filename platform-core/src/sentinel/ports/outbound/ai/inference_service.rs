//! Port d'inference ML (modeles ONNX). L'adapter impl charge les sessions
//! ort et expose vision/text via les memes traits.

use ndarray::{Array2, Array4};

#[derive(Debug, Clone)]
pub struct InferenceClassification {
    pub label: String,
    pub confidence: f32,
}

pub trait InferenceService: Send + Sync {
    fn vision_available(&self) -> bool;
    fn text_available(&self) -> bool;
    fn classify_image(
        &self,
        image_tensor: Array4<f32>,
    ) -> Result<Vec<InferenceClassification>, String>;
    fn classify_text(
        &self,
        input_ids: Array2<i64>,
        attention_mask: Array2<i64>,
    ) -> Result<Vec<InferenceClassification>, String>;
}
