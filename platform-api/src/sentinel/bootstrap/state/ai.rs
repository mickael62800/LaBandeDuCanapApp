//! Etat du domaine IA : analyse de messages et d'images, file de jobs, dataset.

use std::sync::Arc;

use axum::extract::FromRef;
use platform_core::sentinel::ports::inbound::ai::analyze_image::AnalyzeImageUseCase;
use platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use platform_core::sentinel::ports::inbound::ai::manage_ai_jobs::ManageAiJobsUseCase;
use platform_core::sentinel::ports::inbound::ai::manage_dataset::ManageDatasetUseCase;

use crate::sentinel::adapters::outbound::inference_service::InferenceService;
use crate::sentinel::adapters::outbound::ws::broadcaster::EventBroadcaster;
use crate::sentinel::bootstrap::state::AppState;

/// Ports dont dependent les handlers IA, et eux seuls.
///
/// Un handler qui declare `State<AiState>` ne peut pas, meme par accident,
/// toucher a la moderation ou au systeme : le compilateur ne lui en donne pas
/// les moyens. C'est la raison d'etre de ce decoupage — pas l'esthetique.
#[derive(Clone)]
pub struct AiState {
    pub analyze_uc: Arc<dyn AnalyzeMessageUseCase>,
    pub analyze_image_uc: Arc<dyn AnalyzeImageUseCase>,
    pub dataset_uc: Arc<dyn ManageDatasetUseCase>,
    pub ai_jobs_uc: Arc<dyn ManageAiJobsUseCase>,
    /// Runtime ONNX partage. `None` possible cote bootstrap : l'API demarre en
    /// mode degrade (scoring par regles) si les modeles sont absents.
    pub inference: Arc<InferenceService>,

    // ── Dependances transverses du domaine ──
    /// Une analyse qui declenche un flag est diffusee en direct au dashboard.
    pub broadcaster: Arc<EventBroadcaster>,
}

impl FromRef<AppState> for AiState {
    fn from_ref(state: &AppState) -> Self {
        state.ai.clone()
    }
}
