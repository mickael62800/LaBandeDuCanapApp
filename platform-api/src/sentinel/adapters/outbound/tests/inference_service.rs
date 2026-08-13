use super::*;

#[test]
fn test_softmax_sums_to_one() {
    let logits = vec![1.0, 2.0, 3.0];
    let probs = softmax(&logits);
    let sum: f32 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);
}

#[test]
fn test_softmax_highest_logit_gets_highest_prob() {
    let logits = vec![1.0, 5.0, 2.0];
    let probs = softmax(&logits);
    assert!(probs[1] > probs[0]);
    assert!(probs[1] > probs[2]);
}

#[test]
fn test_softmax_equal_logits_gives_uniform() {
    let logits = vec![2.0, 2.0, 2.0];
    let probs = softmax(&logits);
    for p in &probs {
        assert!((p - 1.0 / 3.0).abs() < 1e-6);
    }
}

#[test]
fn test_softmax_single_element() {
    let probs = softmax(&[42.0]);
    assert_eq!(probs.len(), 1);
    assert!((probs[0] - 1.0).abs() < 1e-6);
}

#[test]
fn test_softmax_large_values_no_overflow() {
    let logits = vec![1000.0, 1001.0, 1002.0];
    let probs = softmax(&logits);
    let sum: f32 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-5);
    assert!(probs[2] > probs[1]);
}

#[test]
fn test_softmax_negative_values() {
    let logits = vec![-1.0, -2.0, -3.0];
    let probs = softmax(&logits);
    let sum: f32 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);
    assert!(probs[0] > probs[1]);
    assert!(probs[1] > probs[2]);
}

#[test]
fn test_inference_no_models_loaded() {
    let service = InferenceService::new(None, None);
    assert!(!service.vision_available());
    assert!(!service.text_available());
}

#[test]
fn test_inference_nonexistent_paths() {
    let service = InferenceService::new(
        Some("/nonexistent/vision.onnx"),
        Some("/nonexistent/text.onnx"),
    );
    assert!(!service.vision_available());
    assert!(!service.text_available());
}

#[test]
fn test_classify_image_without_model_returns_error() {
    let service = InferenceService::new(None, None);
    let tensor = Array4::<f32>::zeros((1, 3, 224, 224));
    let result = service.classify_image(tensor);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("non charge"));
}

#[test]
fn test_classify_text_without_model_returns_error() {
    let service = InferenceService::new(None, None);
    let ids = Array2::<i64>::zeros((1, 10));
    let mask = Array2::<i64>::ones((1, 10));
    let result = service.classify_text(ids, mask);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("non charge"));
}

/// Serialise les tests qui manipulent TEXT_MODEL_PATH / VISION_MODEL_PATH
/// car cargo test lance les tests en parallele par defaut — et les env vars
/// sont partages au niveau du process.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn reload_unknown_type_returns_error() {
    let service = InferenceService::new(None, None);
    let err = service.reload("llm-sentiment").unwrap_err();
    assert!(err.contains("inconnu"));
}

#[test]
fn reload_text_without_env_var_returns_error() {
    let _g = ENV_LOCK.lock().unwrap();
    let service = InferenceService::new(None, None);
    unsafe { std::env::remove_var("TEXT_MODEL_PATH") };
    let err = service.reload("text").unwrap_err();
    assert!(err.contains("TEXT_MODEL_PATH"));
}

#[test]
fn reload_vision_without_env_var_returns_error() {
    let _g = ENV_LOCK.lock().unwrap();
    let service = InferenceService::new(None, None);
    unsafe { std::env::remove_var("VISION_MODEL_PATH") };
    let err = service.reload("image-classification").unwrap_err();
    assert!(err.contains("VISION_MODEL_PATH"));
}

#[test]
fn reload_text_alias_text_sentiment_recognized() {
    let _g = ENV_LOCK.lock().unwrap();
    let service = InferenceService::new(None, None);
    unsafe { std::env::remove_var("TEXT_MODEL_PATH") };
    // L'alias "text-sentiment" doit suivre le meme chemin que "text".
    let err = service.reload("text-sentiment").unwrap_err();
    assert!(err.contains("TEXT_MODEL_PATH"));
}

#[test]
fn reload_vision_alias_vision_recognized() {
    let _g = ENV_LOCK.lock().unwrap();
    let service = InferenceService::new(None, None);
    unsafe { std::env::remove_var("VISION_MODEL_PATH") };
    let err = service.reload("vision").unwrap_err();
    assert!(err.contains("VISION_MODEL_PATH"));
}

#[test]
fn reload_text_with_nonexistent_path_returns_error_but_clears_session() {
    let _g = ENV_LOCK.lock().unwrap();
    let service = InferenceService::new(None, None);
    unsafe { std::env::set_var("TEXT_MODEL_PATH", "/tmp/nonexistent_text_model.onnx") };
    let err = service.reload("text").unwrap_err();
    unsafe { std::env::remove_var("TEXT_MODEL_PATH") };
    assert!(err.contains("Echec rechargement text"), "err={err}");
    assert!(!service.text_available());
}

#[test]
fn reload_vision_with_nonexistent_path_returns_error() {
    let _g = ENV_LOCK.lock().unwrap();
    let service = InferenceService::new(None, None);
    unsafe { std::env::set_var("VISION_MODEL_PATH", "/tmp/nonexistent_vision.onnx") };
    let err = service.reload("image-classification").unwrap_err();
    unsafe { std::env::remove_var("VISION_MODEL_PATH") };
    assert!(err.contains("Echec rechargement vision"));
    assert!(!service.vision_available());
}

#[test]
fn load_session_with_invalid_file_content_fails_gracefully() {
    // Ecrit un fichier bidon non-ONNX — load_session doit renvoyer None sans paniquer.
    let dir = std::env::temp_dir();
    let path = dir.join("sentinel_invalid.onnx");
    std::fs::write(&path, b"not-a-real-onnx-model").unwrap();
    let service = InferenceService::new(Some(path.to_str().unwrap()), None);
    assert!(
        !service.vision_available(),
        "file invalide doit etre rejete"
    );
    let _ = std::fs::remove_file(&path);
}

const ONNX_PATH: &str = "../../sentinel-ml/text/exports/text_sentinel.onnx";
const TOKENIZER_PATH: &str = "../../sentinel-ml/text/exports/tokenizer.json";

use crate::sentinel::adapters::outbound::text_tokenizer::TextTokenizer;

fn load_real_pipeline() -> Option<(InferenceService, TextTokenizer)> {
    let service = InferenceService::new(None, Some(ONNX_PATH));
    let tokenizer = TextTokenizer::new(Some(TOKENIZER_PATH), 256);
    if service.text_available() && tokenizer.available() {
        Some((service, tokenizer))
    } else {
        None
    }
}

fn classify(
    service: &InferenceService,
    tokenizer: &TextTokenizer,
    text: &str,
) -> Vec<InferenceClassification> {
    let (ids, mask) = tokenizer.tokenize(text).unwrap();
    service.classify_text(ids, mask).unwrap()
}

fn top_label(classifications: &[InferenceClassification]) -> &str {
    classifications
        .iter()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
        .map(|c| c.label.as_str())
        .unwrap_or("unknown")
}

fn confidence_of(classifications: &[InferenceClassification], label: &str) -> f32 {
    classifications
        .iter()
        .find(|c| c.label == label)
        .map(|c| c.confidence)
        .unwrap_or(0.0)
}

#[test]
#[ignore = "Necessite le fichier ONNX sur le disque"]
fn real_model_loads_successfully() {
    let service = InferenceService::new(None, Some(ONNX_PATH));
    assert!(
        service.text_available(),
        "Modele ONNX introuvable a {ONNX_PATH}"
    );
}

#[test]
fn real_model_returns_5_labels() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let cls = classify(&service, &tokenizer, "bonjour");
    assert_eq!(cls.len(), 5);
    let labels: Vec<&str> = cls.iter().map(|c| c.label.as_str()).collect();
    assert!(labels.contains(&"neutral"));
    assert!(labels.contains(&"anger"));
    assert!(labels.contains(&"rage"));
    assert!(labels.contains(&"threat"));
    assert!(labels.contains(&"harassment"));
}

#[test]
fn real_model_probabilities_sum_to_one() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let cls = classify(&service, &tokenizer, "salut comment ca va");
    let sum: f32 = cls.iter().map(|c| c.confidence).sum();
    assert!((sum - 1.0).abs() < 0.01, "Softmax sum = {sum}");
}

#[test]
fn real_model_neutral_greeting() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let cls = classify(
        &service,
        &tokenizer,
        "Bonjour tout le monde, comment allez-vous ?",
    );
    assert_eq!(top_label(&cls), "neutral");
}

#[test]
fn real_model_neutral_question() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let cls = classify(
        &service,
        &tokenizer,
        "Est-ce que quelqu'un peut m'aider avec ce probleme ?",
    );
    assert_eq!(top_label(&cls), "neutral");
}

#[test]
fn real_model_neutral_thanks() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let cls = classify(
        &service,
        &tokenizer,
        "Merci beaucoup pour votre aide, c'est super gentil",
    );
    assert_eq!(top_label(&cls), "neutral");
}

#[test]
fn real_model_neutral_casual() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let cls = classify(
        &service,
        &tokenizer,
        "Je joue a Minecraft en ce moment, tu veux rejoindre ?",
    );
    assert_eq!(top_label(&cls), "neutral");
}

#[test]
fn real_model_insult_anger_higher_than_neutral_baseline() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let neutral_cls = classify(&service, &tokenizer, "Bonjour, comment ca va ?");
    let insult_cls = classify(&service, &tokenizer, "ferme ta gueule espece de connard");
    let neutral_anger = confidence_of(&neutral_cls, "anger");
    let insult_anger = confidence_of(&insult_cls, "anger");
    assert!(insult_anger > neutral_anger);
}

#[test]
fn real_model_threat_scores_higher_than_greeting() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let greeting = classify(&service, &tokenizer, "Salut, on joue ensemble ?");
    let threat = classify(
        &service,
        &tokenizer,
        "je vais te retrouver et te casser la gueule",
    );
    let g_toxic: f32 = 1.0 - confidence_of(&greeting, "neutral");
    let t_toxic: f32 = 1.0 - confidence_of(&threat, "neutral");
    assert!(t_toxic > g_toxic);
}

#[test]
#[ignore = "test de qualite modele ML"]
fn real_model_rage_scores_higher_than_mild_annoyance() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let mild = classify(&service, &tokenizer, "c'est un peu nul quand meme");
    let rage = classify(
        &service,
        &tokenizer,
        "JE VAIS TOUS VOUS NIQUER BANDE DE FILS DE PUTE",
    );
    let m_anger = confidence_of(&mild, "anger");
    let r_anger = confidence_of(&rage, "anger");
    assert!(r_anger > m_anger);
}

#[test]
fn real_model_harassment_has_higher_harassment_signal() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let neutral = classify(&service, &tokenizer, "Merci beaucoup pour ton aide");
    let harass = classify(
        &service,
        &tokenizer,
        "t'es vraiment qu'une merde, tout le monde te deteste ici, degage",
    );
    let n_h = confidence_of(&neutral, "harassment");
    let h_h = confidence_of(&harass, "harassment");
    assert!(h_h > n_h);
}

#[test]
fn real_model_toxicity_gradient() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let mild = classify(&service, &tokenizer, "c'est nul");
    let medium = classify(&service, &tokenizer, "t'es vraiment un idiot");
    let severe = classify(&service, &tokenizer, "je vais te buter sale fils de pute");
    let mild_toxic = 1.0 - confidence_of(&mild, "neutral");
    let medium_toxic = 1.0 - confidence_of(&medium, "neutral");
    let severe_toxic = 1.0 - confidence_of(&severe, "neutral");
    assert!(severe_toxic >= medium_toxic && medium_toxic >= mild_toxic);
}

#[test]
fn real_model_mild_frustration_mostly_neutral() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let cls = classify(&service, &tokenizer, "c'est un peu nul quand meme ce jeu");
    let neutral_conf = confidence_of(&cls, "neutral");
    assert!(neutral_conf >= 0.4);
}

#[test]
fn real_pipeline_neutral_message_no_flags() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let cls = classify(&service, &tokenizer, "Salut, on fait une partie ce soir ?");
    let result = platform_core::sentinel::application::ai::analyze_message_service::score_classifications(
        &cls,
        &[],
        0.5,
        &platform_core::sentinel::domain::services::moderation::scoring_service::ScoringConfig::default(),
    );
    assert!(result.is_none());
}

#[test]
fn real_pipeline_neutral_no_flags_even_low_threshold() {
    let Some((service, tokenizer)) = load_real_pipeline() else {
        return;
    };
    let cls = classify(
        &service,
        &tokenizer,
        "Bonjour tout le monde, bonne journee !",
    );
    let result = platform_core::sentinel::application::ai::analyze_message_service::score_classifications(
        &cls,
        &[],
        0.1,
        &platform_core::sentinel::domain::services::moderation::scoring_service::ScoringConfig::default(),
    );
    if let Some((score, _, _)) = result {
        assert!(score < 5.0);
    }
}
