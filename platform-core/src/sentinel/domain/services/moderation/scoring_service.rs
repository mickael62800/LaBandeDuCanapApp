use crate::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::sentinel::domain::entities::system::rule::Rule;
use crate::sentinel::domain::enums::moderation::action::Action;
use crate::sentinel::domain::enums::moderation::flag_type::FlagType;
/// Poids par défaut quand aucune règle n'est configurée pour un flag.
const DEFAULT_WEIGHT_SPAM: f64 = 3.0;
const DEFAULT_WEIGHT_INSULT: f64 = 5.0;
/// Juron d'exclamation. Volontairement SOUS le seuil d'avertissement (2.0) :
/// « merde j'ai oublie » ne doit rien declencher seul. Combine a un autre
/// signal, il pese quand meme dans la balance.
const DEFAULT_WEIGHT_PROFANITY: f64 = 1.0;
const DEFAULT_WEIGHT_LINK: f64 = 1.0;
const DEFAULT_WEIGHT_PHISHING: f64 = 7.0;
// IA Vision
const DEFAULT_WEIGHT_NSFW: f64 = 8.0;
const DEFAULT_WEIGHT_ILLICIT: f64 = 9.0;
// IA Text Sentiment
const DEFAULT_WEIGHT_ANGER: f64 = 3.0;
const DEFAULT_WEIGHT_RAGE: f64 = 6.0;
const DEFAULT_WEIGHT_THREAT: f64 = 8.0;
const DEFAULT_WEIGHT_HARASSMENT: f64 = 7.0;

/// Seuils par défaut.
const DEFAULT_THRESHOLD_WARN: f64 = 2.0;
const DEFAULT_THRESHOLD_DELETE: f64 = 4.0;
const DEFAULT_THRESHOLD_MUTE: f64 = 6.0;
const DEFAULT_THRESHOLD_BAN: f64 = 9.0;

/// Durée de mute par défaut (secondes).
const DEFAULT_MUTE_DURATION: u64 = 600;

/// Modèle de scoring paramétrable : poids par flag + seuils d'action.
///
/// Le domaine reste PUR : cette structure est passée EN ENTRÉE (as data) ;
/// le service ne lit jamais la config du serveur lui-même. La couche
/// application construit un `ScoringConfig` depuis la config `automod-bot`
/// (poids/seuils éditables par serveur) puis le fournit ici.
///
/// L'implémentation `Default` reproduit EXACTEMENT les constantes historiques,
/// si bien que le comportement est inchangé tant qu'aucune surcharge n'est
/// configurée.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoringConfig {
    pub weight_spam: f64,
    pub weight_insult: f64,
    pub weight_profanity: f64,
    pub weight_link: f64,
    pub weight_phishing: f64,
    pub weight_nsfw: f64,
    pub weight_illicit: f64,
    pub weight_anger: f64,
    pub weight_rage: f64,
    pub weight_threat: f64,
    pub weight_harassment: f64,
    pub threshold_warn: f64,
    pub threshold_delete: f64,
    pub threshold_mute: f64,
    pub threshold_ban: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            weight_spam: DEFAULT_WEIGHT_SPAM,
            weight_insult: DEFAULT_WEIGHT_INSULT,
            weight_profanity: DEFAULT_WEIGHT_PROFANITY,
            weight_link: DEFAULT_WEIGHT_LINK,
            weight_phishing: DEFAULT_WEIGHT_PHISHING,
            weight_nsfw: DEFAULT_WEIGHT_NSFW,
            weight_illicit: DEFAULT_WEIGHT_ILLICIT,
            weight_anger: DEFAULT_WEIGHT_ANGER,
            weight_rage: DEFAULT_WEIGHT_RAGE,
            weight_threat: DEFAULT_WEIGHT_THREAT,
            weight_harassment: DEFAULT_WEIGHT_HARASSMENT,
            threshold_warn: DEFAULT_THRESHOLD_WARN,
            threshold_delete: DEFAULT_THRESHOLD_DELETE,
            threshold_mute: DEFAULT_THRESHOLD_MUTE,
            threshold_ban: DEFAULT_THRESHOLD_BAN,
        }
    }
}

impl ScoringConfig {
    /// Poids de base (défaut/baseline) pour un flag donné. Une règle DB
    /// spécifique reste prioritaire (cf. `score_with_config`).
    pub fn weight_for(&self, flag: &FlagType) -> f64 {
        match flag {
            FlagType::Spam => self.weight_spam,
            FlagType::Insult => self.weight_insult,
            FlagType::Profanity => self.weight_profanity,
            FlagType::Link => self.weight_link,
            FlagType::Phishing => self.weight_phishing,
            FlagType::Nsfw => self.weight_nsfw,
            FlagType::Illicit => self.weight_illicit,
            FlagType::Anger => self.weight_anger,
            FlagType::Rage => self.weight_rage,
            FlagType::Threat => self.weight_threat,
            FlagType::Harassment => self.weight_harassment,
        }
    }
}

/// Résultat du scoring.
pub struct ScoringResult {
    pub score: f64,
    pub action: Action,
    pub reason: String,
    pub duration: Option<u64>,
}

/// Service pur de scoring — aucune dépendance externe.
pub struct ScoringService;

impl ScoringService {
    /// Calcule le score d'un message à partir de ses flags et des règles du serveur.
    ///
    /// Algorithme :
    /// 1. Pour chaque flag actif, récupérer le poids (règle custom ou défaut)
    /// 2. Sommer les poids → score total
    /// 3. Comparer le score aux seuils (du plus sévère au moins sévère)
    /// 4. Retourner l'action correspondante
    pub fn score(flags: &DetectionFlags, rules: &[Rule]) -> ScoringResult {
        Self::score_with_config(
            flags,
            rules,
            &ScoringConfig::default(),
            DEFAULT_MUTE_DURATION,
        )
    }

    /// Version paramétrique : `config` fournit les poids/seuils de BASELINE
    /// (éditables par serveur, injectés par la couche application) et
    /// `mute_duration` la durée de mute. Les règles DB spécifiques à un flag
    /// restent prioritaires sur le baseline.
    pub fn score_with_config(
        flags: &DetectionFlags,
        rules: &[Rule],
        config: &ScoringConfig,
        mute_duration: u64,
    ) -> ScoringResult {
        let active = flags.active_flags();

        if active.is_empty() {
            return ScoringResult {
                score: 0.0,
                action: Action::None,
                reason: String::new(),
                duration: None,
            };
        }

        // Calculer le score
        let mut total_score = 0.0;
        let mut triggered: Vec<&str> = Vec::new();

        for flag in &active {
            let rule = rules.iter().find(|r| r.flag_type == *flag && r.enabled);
            let weight = match rule {
                Some(r) => r.weight,
                None => config.weight_for(flag),
            };
            total_score += weight;
            triggered.push(flag.as_str());
        }

        // Déterminer les seuils à partir des SEULES règles dont le flag a été
        // déclenché (per-flag-type) : une règle stricte sur une catégorie sans
        // rapport (ex. liens) ne doit pas abaisser le seuil d'une autre (ex. insulte).
        let (t_warn, t_delete, t_mute, t_ban) = resolve_thresholds(rules, &active, config);

        // Déterminer l'action (du plus sévère au moins sévère)
        let (action, duration) = if total_score >= t_ban {
            (Action::Ban, None)
        } else if total_score >= t_mute {
            (Action::Mute, Some(mute_duration))
        } else if total_score >= t_delete {
            (Action::Delete, None)
        } else if total_score >= t_warn {
            (Action::Warn, None)
        } else {
            (Action::None, None)
        };

        let reason = format!(
            "Détection : {} (score: {:.1})",
            triggered.join(", "),
            total_score
        );

        ScoringResult {
            score: total_score,
            action,
            reason,
            duration,
        }
    }
}

/// Poids par défaut (baseline historique) pour un flag. Conservé comme
/// helper de commodité pour les tests — délègue au `Default` de `ScoringConfig`.
#[cfg(test)]
fn default_weight(flag: &FlagType) -> f64 {
    ScoringConfig::default().weight_for(flag)
}

/// Résout les seuils depuis les règles, en ne considérant QUE les règles dont
/// le `flag_type` figure parmi les flags réellement déclenchés (`fired`).
///
/// Motivation (correctness) : le score somme les poids des flags déclenchés ;
/// les seuils doivent donc venir des mêmes catégories. Avant, on prenait le
/// minimum des seuils sur TOUTES les règles activées, si bien qu'une règle très
/// stricte (seuil bas) sur une catégorie sans rapport abaissait le seuil de
/// toutes les autres détections. On restreint désormais aux règles pertinentes.
///
/// Comportement : parmi les règles activées dont le flag est déclenché, on
/// prend le seuil le plus bas (le plus strict) à chaque niveau. Si aucune règle
/// ne correspond aux flags déclenchés, on retombe sur les seuils par défaut.
pub fn resolve_thresholds(
    rules: &[Rule],
    fired: &[FlagType],
    config: &ScoringConfig,
) -> (f64, f64, f64, f64) {
    let relevant: Vec<&Rule> = rules
        .iter()
        .filter(|r| r.enabled && fired.contains(&r.flag_type))
        .collect();

    if relevant.is_empty() {
        return (
            config.threshold_warn,
            config.threshold_delete,
            config.threshold_mute,
            config.threshold_ban,
        );
    }

    let warn = relevant
        .iter()
        .map(|r| r.threshold_warn)
        .fold(f64::MAX, f64::min);
    let delete = relevant
        .iter()
        .map(|r| r.threshold_delete)
        .fold(f64::MAX, f64::min);
    let mute = relevant
        .iter()
        .map(|r| r.threshold_mute)
        .fold(f64::MAX, f64::min);
    let ban = relevant
        .iter()
        .map(|r| r.threshold_ban)
        .fold(f64::MAX, f64::min);

    (warn, delete, mute, ban)
}

#[cfg(test)]
#[path = "tests/scoring_service.rs"]
mod tests;
