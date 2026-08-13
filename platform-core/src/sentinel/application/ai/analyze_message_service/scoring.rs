use super::*;

/// Fonction pure : transforme les classifications IA en score, flags et raison.
/// Retourne None si aucun sentiment toxique n'est detecte au-dessus du seuil.
pub fn score_classifications(
    classifications: &[crate::sentinel::ports::outbound::ai::inference_service::InferenceClassification],
    rules: &[crate::sentinel::domain::entities::system::rule::Rule],
    threshold: f32,
    scoring_config: &ScoringConfig,
) -> Option<(f64, Vec<FlagType>, String)> {
    let mut detected: Vec<(FlagType, f32)> = Vec::new();

    for c in classifications {
        let flag = match c.label.as_str() {
            // Modele 2 classes : severe = rage + threat agreges.
            // On mappe sur FlagType::Harassment (la plus generique des flags
            // toxiques) pour que le scoring existant fonctionne sans ajouter
            // un nouveau type.
            "severe" if c.confidence >= threshold => Some(FlagType::Harassment),
            // Legacy 5 classes (si vieux modele encore charge).
            "anger" if c.confidence >= threshold => Some(FlagType::Anger),
            "rage" if c.confidence >= threshold => Some(FlagType::Rage),
            "threat" if c.confidence >= threshold => Some(FlagType::Threat),
            "harassment" if c.confidence >= threshold => Some(FlagType::Harassment),
            _ => None,
        };

        if let Some(flag_type) = flag {
            detected.push((flag_type, c.confidence));
        }
    }

    if detected.is_empty() {
        return None;
    }

    let mut ia_score = 0.0;
    let mut triggered: Vec<String> = Vec::new();

    for (flag_type, confidence) in &detected {
        let rule = rules
            .iter()
            .find(|r| r.flag_type == *flag_type && r.enabled);
        let base_weight = match rule {
            Some(r) => r.weight,
            None => scoring_config.weight_for(flag_type),
        };
        let weighted = base_weight * (*confidence as f64);
        ia_score += weighted;
        triggered.push(format!(
            "{}({:.0}%)",
            flag_type.as_str(),
            confidence * 100.0
        ));
    }

    let reason = format!("IA sentiment : {}", triggered.join(", "));
    Some((
        ia_score,
        detected.into_iter().map(|(f, _)| f).collect(),
        reason,
    ))
}

/// Transforme la réponse DeepSeek en signal de modération pondéré.
///
/// DeepSeek retourne une confiance de toxicité entre 0 et 1. Cette confiance
/// doit être multipliée par le poids de la règle correspondante, exactement
/// comme les classifications ONNX locales ; l'ajouter directement au score
/// rendrait les seuils configurés (2, 4, 6, 9…) inatteignables.
pub fn score_deepseek_analysis(
    analysis: &crate::sentinel::ports::outbound::ai::deepseek_moderation_service::DeepSeekModerationAnalysis,
    rules: &[crate::sentinel::domain::entities::system::rule::Rule],
    threshold: f32,
    scoring_config: &ScoringConfig,
) -> Option<(f64, Vec<FlagType>, String)> {
    if analysis.toxicity_score < threshold as f64 {
        return None;
    }

    let flag_for_label = |label: &str| match label.trim().to_ascii_lowercase().as_str() {
        "anger" | "angry" | "colere" | "colère" => Some(FlagType::Anger),
        "rage" | "aggressive" | "agressif" | "agression" => Some(FlagType::Rage),
        "threat" | "threatening" | "menace" => Some(FlagType::Threat),
        "harassment" | "hate" | "hate_speech" | "toxic" | "toxicity" | "harcelement"
        | "harcèlement" => Some(FlagType::Harassment),
        "insult" | "insulte" => Some(FlagType::Insult),
        "profanity" | "profanite" | "profanité" => Some(FlagType::Profanity),
        "spam" => Some(FlagType::Spam),
        "nsfw" => Some(FlagType::Nsfw),
        _ => None,
    };

    let mut detected = Vec::new();
    if let Some(flag) = flag_for_label(&analysis.sentiment) {
        detected.push(flag);
    }
    for label in &analysis.flags {
        if let Some(flag) = flag_for_label(label) {
            if !detected.contains(&flag) {
                detected.push(flag);
            }
        }
    }
    // Une réponse IA explicitement toxique doit toujours produire un poids,
    // même si le fournisseur a utilisé un libellé non encore connu.
    if detected.is_empty() {
        detected.push(FlagType::Harassment);
    }

    let confidence = analysis.toxicity_score.clamp(0.0, 1.0);
    let score = detected
        .iter()
        .map(|flag| {
            rules
                .iter()
                .find(|rule| rule.flag_type == *flag && rule.enabled)
                .map(|rule| rule.weight)
                .unwrap_or_else(|| scoring_config.weight_for(flag))
                * confidence
        })
        .sum();
    let labels = detected
        .iter()
        .map(FlagType::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let reason = format!(
        "DeepSeek [{} — {:.0}%] : {}",
        labels,
        confidence * 100.0,
        analysis.reason
    );

    Some((score, detected, reason))
}

/// Construit un contenu enrichi avec le contexte conversationnel pour l'inference IA.
/// Le message analyse est place en premier (safe si le tokenizer tronque la fin).
/// - "natural" : conversation brute separee par des retours a la ligne
/// - "tagged"  : balises [message]/[context] pour structurer l'input
pub(super) fn build_contextual_content(
    content: &str,
    context: &[crate::sentinel::ports::inbound::ai::analyze_message::ContextMessageEntry],
    format: &str,
) -> String {
    if context.is_empty() {
        return content.to_string();
    }
    let ctx_str: String = context
        .iter()
        .map(|m| format!("{}: {}", m.username, m.content))
        .collect::<Vec<_>>()
        .join("\n");
    match format {
        "tagged" => format!(
            "[message] {} [/message] [context] {} [/context]",
            content, ctx_str
        ),
        _ => format!("{}\n---\n{}", ctx_str, content),
    }
}

/// C5 — empêche qu'une détection IA fasse, à elle seule, basculer un message en
/// Ban AUTOMATIQUE. Le score combiné `bot + IA` peut, sur un premier message,
/// dépasser le seuil de ban sans aucune escalade. Si l'action calculée est Ban
/// alors que le score BOT seul n'atteignait pas le seuil de ban, on plafonne
/// l'action à Mute (le Ban reste atteignable via l'escalade de strikes ou une
/// décision humaine sur la carte de review). Le Ban auto déclenché par le seul
/// score bot (≥ seuil) est préservé (comportement historique).
pub(crate) fn cap_ia_induced_ban(
    action: Action,
    duration: Option<u64>,
    bot_score: f64,
    t_ban: f64,
    mute_duration_secs: u64,
) -> (Action, Option<u64>) {
    if matches!(action, Action::Ban) && bot_score < t_ban {
        (Action::Mute, Some(mute_duration_secs))
    } else {
        (action, duration)
    }
}
