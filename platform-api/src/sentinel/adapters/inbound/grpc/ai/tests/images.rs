use super::*;

#[test]
fn action_to_proto_all_variants() {
    assert_eq!(action_to_proto(Action::None), proto::Action::None as i32);
    assert_eq!(action_to_proto(Action::Warn), proto::Action::Warn as i32);
    assert_eq!(
        action_to_proto(Action::Delete),
        proto::Action::Delete as i32
    );
    assert_eq!(action_to_proto(Action::Mute), proto::Action::Mute as i32);
    assert_eq!(action_to_proto(Action::Ban), proto::Action::Ban as i32);
}

#[test]
fn classification_to_proto_mapping() {
    let c = ImageClassification {
        label: "weapon".into(),
        confidence: 0.92,
    };
    let p = classification_to_proto(c);
    assert_eq!(p.label, "weapon");
    assert!((p.confidence - 0.92).abs() < 1e-6);
}

#[test]
fn analysis_to_proto_full_mapping() {
    let a = ImageAnalysis {
        action: Action::Delete,
        reason: "violence detectee".into(),
        score: 0.87,
        duration: Some(150),
        classifications: vec![
            ImageClassification {
                label: "violence".into(),
                confidence: 0.87,
            },
            ImageClassification {
                label: "neutral".into(),
                confidence: 0.13,
            },
        ],
    };
    let p = analysis_to_proto(a);
    assert_eq!(p.action, proto::Action::Delete as i32);
    assert_eq!(p.reason, "violence detectee");
    assert!((p.score - 0.87).abs() < 1e-6);
    assert_eq!(p.duration, Some(150));
    assert_eq!(p.classifications.len(), 2);
    assert_eq!(p.classifications[0].label, "violence");
}

#[test]
fn analysis_to_proto_no_action_no_classifications() {
    let a = ImageAnalysis {
        action: Action::None,
        reason: "ok".into(),
        score: 0.0,
        duration: None,
        classifications: vec![],
    };
    let p = analysis_to_proto(a);
    assert_eq!(p.action, proto::Action::None as i32);
    assert!(p.classifications.is_empty());
    assert!(p.duration.is_none());
}

// ── RPC handler tests avec mock ──

use async_trait::async_trait;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::ai::analyze_image::AnalyzeImageCommand;
use platform_core::sentinel::ports::inbound::ai::analyze_image::AnalyzeImageUseCase;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Default)]
struct MockAnalyzeImage {
    calls: Mutex<Vec<AnalyzeImageCommand>>,
}

#[async_trait]
impl AnalyzeImageUseCase for MockAnalyzeImage {
    async fn analyze_image(&self, cmd: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError> {
        self.calls.lock().unwrap().push(cmd);
        Ok(ImageAnalysis {
            action: Action::Warn,
            reason: "suspicious".into(),
            score: 0.65,
            duration: None,
            classifications: vec![ImageClassification {
                label: "nudity".into(),
                confidence: 0.65,
            }],
        })
    }
}

#[tokio::test]
async fn analyze_image_delegates_to_uc() {
    let uc = Arc::new(MockAnalyzeImage::default());
    let g = ImagesGrpc { uc: uc.clone() };
    let resp = g
        .analyze_image(Request::new(proto::AnalyzeImageRequest {
            guild_id: "g".into(),
            channel_id: "c".into(),
            user_id: "u".into(),
            username: "alice".into(),
            message_id: "m".into(),
            image_data: vec![0u8, 1u8, 2u8, 3u8],
            content_type: "image/png".into(),
            filename: "pic.png".into(),
        }))
        .await
        .unwrap();

    let inner = resp.into_inner();
    assert_eq!(inner.action, proto::Action::Warn as i32);
    assert_eq!(inner.reason, "suspicious");
    assert_eq!(inner.classifications.len(), 1);

    let calls = uc.calls.lock().unwrap();
    assert_eq!(calls[0].guild_id, "g".into());
    assert_eq!(calls[0].filename, "pic.png");
    assert_eq!(calls[0].content_type, "image/png");
    assert_eq!(calls[0].image_bytes.len(), 4);
}

#[tokio::test]
async fn analyze_image_empty_bytes_still_delegated() {
    // Pas de validation sur la taille cote handler → le UC recoit l'input vide.
    let uc = Arc::new(MockAnalyzeImage::default());
    let g = ImagesGrpc { uc: uc.clone() };
    let _ = g
        .analyze_image(Request::new(proto::AnalyzeImageRequest {
            guild_id: "g".into(),
            channel_id: "c".into(),
            user_id: "u".into(),
            username: "a".into(),
            message_id: "m".into(),
            image_data: vec![],
            content_type: "image/jpeg".into(),
            filename: "empty.jpg".into(),
        }))
        .await
        .unwrap();
    assert_eq!(uc.calls.lock().unwrap()[0].image_bytes.len(), 0);
}
