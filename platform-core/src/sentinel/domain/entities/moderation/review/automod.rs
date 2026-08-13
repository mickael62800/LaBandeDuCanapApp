//! Carte de review automod persistee (cf. migration 176).
//!
//! Le bot poste un embed avec boutons (Apply / Warn / Mute / Ban / Ignore)
//! dans le salon de logs ; en parallele il INSERT une `AutomodReview` dans
//! cette table et register l'`action_id` dans `discord_action_messages`.
//! Du coup la web peut lister les reviews pending et resoudre depuis l UI ;
//! le bot edite la carte Discord en reaction (sync bilateral).

use crate::sentinel::domain::entities::system::discord_ids::ChannelId;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::MessageId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

/// Carte de review close (applied|ignored) et expiree : encore mappee a un
/// message Discord, a faire disparaitre par le bot. Le `action_id` est l'id
/// de la review ; le mapping est retire cote repo.
#[derive(Debug, Clone)]
pub struct ExpiredReviewCard {
    pub action_id: Uuid,
    pub channel_id: String,
    pub message_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestedAction {
    Warn,
    Delete,
    Mute,
    Ban,
}

impl SuggestedAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Delete => "delete",
            Self::Mute => "mute",
            Self::Ban => "ban",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "warn" => Some(Self::Warn),
            "delete" => Some(Self::Delete),
            "mute" => Some(Self::Mute),
            "ban" => Some(Self::Ban),
            _ => None,
        }
    }
    /// Rang de severite (warn < delete < mute < ban).
    pub fn severity(&self) -> u8 {
        match self {
            Self::Warn => 1,
            Self::Delete => 2,
            Self::Mute => 3,
            Self::Ban => 4,
        }
    }
}

/// Retourne la plus severe des deux actions suggerees (strings). Sert a
/// l'agregation : l'action d'une carte regroupee escalade vers le pire vu.
/// En cas de valeur inconnue, on retombe sur l'autre (ou "warn").
pub fn more_severe_suggested(a: &str, b: &str) -> String {
    let rank = |s: &str| {
        SuggestedAction::from_str(s)
            .map(|x| x.severity())
            .unwrap_or(0)
    };
    if rank(a) >= rank(b) {
        if rank(a) == 0 {
            "warn".to_string()
        } else {
            a.to_string()
        }
    } else {
        b.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppliedAction {
    /// Cran le plus leger : prevention (tracee, hors escalade).
    Prevention,
    Warn,
    Delete,
    Mute,
    Ban,
    Ignore,
}

impl AppliedAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Prevention => "prevention",
            Self::Warn => "warn",
            Self::Delete => "delete",
            Self::Mute => "mute",
            Self::Ban => "ban",
            Self::Ignore => "ignore",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "prevention" => Some(Self::Prevention),
            "warn" => Some(Self::Warn),
            "delete" => Some(Self::Delete),
            "mute" => Some(Self::Mute),
            "ban" => Some(Self::Ban),
            "ignore" => Some(Self::Ignore),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutomodReview {
    /// Identifiant de la carte de review et de son cycle de resolution.
    pub id: Uuid,
    /// Perimetre Discord de la review.
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub user_id: UserId,
    pub user_name: String,
    pub content_preview: String,
    /// Action proposee par AutoMod ; elle reste modifiable par un moderateur.
    pub suggested_action: String,
    /// Score ayant justifie la mise en review.
    pub score: f64,
    /// Motif explique a l'operateur.
    pub reason: String,
    pub flags: serde_json::Value,
    /// Etat persiste (`pending`, `voting`, `decided`, `applied`, `ignored`, `expired`).
    pub status: String,
    /// Action effectivement retenue apres review ou vote.
    pub applied_action: Option<String>,
    pub resolved_by_id: Option<String>,
    pub resolved_by_name: Option<String>,
    pub resolved_source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    // ── Systeme de vote (cf. migration 251) ──
    /// Echeance du vote (statut 'voting'). None si review hors mode vote.
    pub voting_deadline: Option<DateTime<Utc>>,
    /// Sanction retenue apres depouillement (statut 'decided'+).
    pub decided_action: Option<String>,
    /// Le quorum minimum de votes a-t-il ete atteint ?
    pub quorum_met: bool,
    /// Horodatage du depouillement.
    pub decided_at: Option<DateTime<Utc>>,
    // ── Agregation par utilisateur (cf. migration 264) ──
    /// Nombre d'incidents agreges dans cette carte (1 si pas de regroupement).
    pub incident_count: i32,
    /// Somme des scores des incidents agreges (le champ `score` reste le max).
    pub cumulative_score: f64,
    /// Liste JSON des incidents agreges
    /// (`[{message_id, channel_id, content_preview, score, reason, suggested_action, at}]`).
    pub incidents: serde_json::Value,
    /// `true` si une sanction de membre a déjà été journalisée pour cet incident
    /// (auto-protection sévère). La finalisation de la carte NE re-journalise
    /// PAS la sanction dans ce cas (anti double-strike, cf. C1).
    pub sanction_logged: bool,
}

// ── Vote des moderateurs ──────────────────────────────────────────────

/// Sanction qu'un moderateur peut voter (identique a AppliedAction, mais
/// nommee distinctement pour exprimer l'intention "vote").
pub type VoteAction = AppliedAction;

impl AppliedAction {
    /// Rang de severite : sert au tie-break (plus clemente / plus severe).
    /// ignore (0) < prevention (1) < warn (2) < delete (3) < mute (4) < ban (5).
    pub fn severity(&self) -> u8 {
        match self {
            Self::Ignore => 0,
            Self::Prevention => 1,
            Self::Warn => 2,
            Self::Delete => 3,
            Self::Mute => 4,
            Self::Ban => 5,
        }
    }
}

/// Un vote individuel persiste (table automod_review_votes).
#[derive(Debug, Clone)]
pub struct ReviewVote {
    pub id: Uuid,
    pub review_id: Uuid,
    pub voter_id: String,
    pub voter_name: String,
    pub vote_action: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Strategie de departage en cas d'egalite de voix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TieAction {
    /// Aucune sanction.
    Ignore,
    /// La sanction la plus clemente parmi les ex-aequo.
    Clemente,
    /// La sanction la plus severe parmi les ex-aequo.
    Severe,
}

impl TieAction {
    pub fn from_str(s: &str) -> Self {
        match s {
            "clemente" => Self::Clemente,
            "severe" => Self::Severe,
            _ => Self::Ignore,
        }
    }
}

/// Resultat d'un depouillement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TallyResult {
    /// Sanction retenue. `Ignore` = aucune sanction (refus, quorum non
    /// atteint, ou egalite resolue en ignore).
    pub decided: AppliedAction,
    /// Le quorum a-t-il ete atteint ?
    pub quorum_met: bool,
    /// Nombre total de votes exprimes.
    pub total_votes: usize,
}

/// Depouille les votes : majorite des voix, quorum minimum, tie-break.
///
/// - Si `total < quorum` -> Ignore, quorum_met=false (alerte ignoree).
/// - Sinon la sanction avec le plus de voix gagne.
/// - En cas d'egalite entre plusieurs sanctions, applique `tie`.
pub fn tally_votes(votes: &[VoteAction], quorum: usize, tie: TieAction) -> TallyResult {
    let total = votes.len();
    if total == 0 || total < quorum.max(1) {
        return TallyResult {
            decided: AppliedAction::Ignore,
            quorum_met: false,
            total_votes: total,
        };
    }

    // Comptage par action.
    let mut counts: std::collections::HashMap<u8, (AppliedAction, usize)> =
        std::collections::HashMap::new();
    for v in votes {
        let entry = counts.entry(v.severity()).or_insert((v.clone(), 0));
        entry.1 += 1;
    }

    let max_count = counts.values().map(|(_, c)| *c).max().unwrap_or(0);
    let mut leaders: Vec<AppliedAction> = counts
        .values()
        .filter(|(_, c)| *c == max_count)
        .map(|(a, _)| a.clone())
        .collect();

    let decided = if leaders.len() == 1 {
        leaders.remove(0)
    } else {
        // Egalite : departage.
        match tie {
            TieAction::Ignore => AppliedAction::Ignore,
            TieAction::Clemente => leaders.into_iter().min_by_key(|a| a.severity()).unwrap(),
            TieAction::Severe => leaders.into_iter().max_by_key(|a| a.severity()).unwrap(),
        }
    };

    TallyResult {
        decided,
        quorum_met: true,
        total_votes: total,
    }
}

// ── Salon de discussion lie a une review ─────────────────────────────

/// Salon textuel ouvert pour discuter d'une review (membre + modos).
/// Persiste pour l'audit et l'idempotence (un seul salon par review).
#[derive(Debug, Clone)]
pub struct DiscussionChannel {
    pub id: Uuid,
    pub review_id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub opened_by_id: String,
    pub opened_by_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewDiscussionChannel {
    pub review_id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub opened_by_id: String,
    pub opened_by_name: String,
}

/// Un message capture dans le salon de discussion (transcript persistant).
#[derive(Debug, Clone)]
pub struct DiscussionMessage {
    pub review_id: Uuid,
    pub discord_message_id: String,
    pub author_id: String,
    pub author_name: String,
    pub author_is_bot: bool,
    pub content: String,
    pub sent_at: DateTime<Utc>,
}

/// Faits Discord du demandeur, fournis par l'adapter bot. La DECISION
/// d'autorisation (les regles ci-dessous) est prise par le domaine, pas par
/// le bot — utilise pour le vote, la finalisation et l'ouverture de discussion.
#[derive(Debug, Clone, Default)]
pub struct ModeratorFacts {
    pub is_admin: bool,
    pub has_moderate_members: bool,
    pub has_manage_messages: bool,
    /// Porte le role moderateur configure (`vote_mod_role_id`).
    pub has_mod_role: bool,
    /// Porte le role admin configure (`vote_admin_role_id`).
    pub has_admin_role: bool,
}

/// Regle metier : qui est "moderateur" (peut voter, ouvrir une discussion).
/// Admin, "Moderer les membres", "Gerer les messages", ou role modo configure.
pub fn is_moderator(f: &ModeratorFacts) -> bool {
    f.is_admin || f.has_moderate_members || f.has_manage_messages || f.has_mod_role
}

/// Regle metier : qui peut FINALISER un vote (appliquer la sanction).
/// Reserve aux administrateurs (permission ADMINISTRATOR ou role admin configure).
pub fn can_finalize_review(f: &ModeratorFacts) -> bool {
    f.is_admin || f.has_admin_role
}

/// Regle metier : qui peut ouvrir un salon de discussion (= moderateur).
pub fn can_open_discussion(f: &ModeratorFacts) -> bool {
    is_moderator(f)
}

// ── Mesure des faux positifs (over-block) de l'automod ───────────────────

/// Plafond d'echantillon pour l'agregation FP : au-dela on tronque et on
/// signale `capped=true`.
pub const FP_STATS_MAX_ROWS: i64 = 5000;

/// Borne saine de l'échéance d'un vote (heures) : 1h à 30 jours. Source
/// unique, partagée entre le use case (reopen) et le bot (post de carte).
pub fn clamp_vote_deadline_hours(hours: i64) -> i64 {
    hours.clamp(1, 720)
}

/// Décision de journalisation à la finalisation d'une carte automod.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizeSanctionPlan {
    /// Action non journalisable (`delete`/`ignore`/inconnue) : rien à faire.
    Nothing,
    /// C1 — anti double-strike : l'auto-protection sévère a déjà journalisé
    /// une sanction (mute auto) et la finalisation n'est pas plus sévère —
    /// on ne re-journalise pas (sinon un incident = deux strikes).
    AlreadyLogged,
    /// Chemin nominal : journaliser l'action AVEC strike.
    LogWithStrike,
    /// BUG #5 — la finalisation est PLUS SÉVÈRE que le mute auto : on
    /// journalise l'escalade (sinon l'action lourde n'apparaît nulle part)
    /// mais SANS second strike (le mute auto a déjà compté celui de
    /// l'incident).
    LogWithoutStrike,
}

/// Règle de journalisation à la finalisation : quelle action journaliser et
/// avec quel effet strike, selon que l'auto-protection a déjà sanctionné.
pub fn finalize_sanction_plan(applied_action: &str, sanction_logged: bool) -> FinalizeSanctionPlan {
    if !matches!(applied_action, "prevention" | "warn" | "mute" | "ban") {
        return FinalizeSanctionPlan::Nothing;
    }
    if !sanction_logged {
        return FinalizeSanctionPlan::LogWithStrike;
    }
    // L'auto-protection sévère journalise un mute : c'est la référence.
    let auto_severity = AppliedAction::Mute.severity();
    let finalized_severity = AppliedAction::from_str(applied_action)
        .map(|a| a.severity())
        .unwrap_or(0);
    if finalized_severity <= auto_severity {
        FinalizeSanctionPlan::AlreadyLogged
    } else {
        FinalizeSanctionPlan::LogWithoutStrike
    }
}

/// Ligne terminale (statut applied|ignored|decided) chargee pour mesurer le
/// taux de faux positifs. Donnee brute cote repo, agregee en Rust.
#[derive(Debug, Clone)]
pub struct FpTerminalReview {
    pub suggested_action: String,
    pub applied_action: Option<String>,
    pub decided_action: Option<String>,
    /// Objet JSONB de booleens (flags detecteurs actifs).
    pub flags: serde_json::Value,
}

/// Severite unifiee (echelle AppliedAction : ignore=0 < prevention < warn <
/// delete < mute < ban). Valeur absente/inconnue vaut 0 (= aucune sanction).
fn fp_severity(action: Option<&str>) -> u8 {
    action
        .and_then(AppliedAction::from_str)
        .map(|a| a.severity())
        .unwrap_or(0)
}

#[derive(Default, Clone)]
struct FpAcc {
    total: i64,
    overturned: i64,
    ignored: i64,
}

impl FpAcc {
    fn add(&mut self, overturned: bool, ignored: bool) {
        self.total += 1;
        if overturned {
            self.overturned += 1;
        }
        if ignored {
            self.ignored += 1;
        }
    }
    fn rate(&self) -> f64 {
        if self.total > 0 {
            self.overturned as f64 / self.total as f64
        } else {
            0.0
        }
    }
}

/// Stat globale (over-block agrege).
#[derive(Debug, Clone)]
pub struct FpBucket {
    pub total: i64,
    pub overturned: i64,
    pub ignored: i64,
    pub fp_rate: f64,
}

#[derive(Debug, Clone)]
pub struct FpFlagStat {
    pub flag: String,
    pub total: i64,
    pub overturned: i64,
    pub ignored: i64,
    pub fp_rate: f64,
}

#[derive(Debug, Clone)]
pub struct FpActionStat {
    pub suggested_action: String,
    pub total: i64,
    pub overturned: i64,
    pub ignored: i64,
    pub fp_rate: f64,
}

#[derive(Debug, Clone)]
pub struct FpStats {
    pub days: i64,
    /// True si l'echantillon a ete tronque a `FP_STATS_MAX_ROWS`.
    pub capped: bool,
    pub overall: FpBucket,
    pub by_flag: Vec<FpFlagStat>,
    pub by_suggested_action: Vec<FpActionStat>,
}

/// Agrege les reviews terminales et mesure le taux de faux positifs
/// (over-block) global, par flag detecteur et par action suggeree.
///
/// Une review est un "faux positif" quand l'automod a SUGGERE une vraie
/// sanction mais que la decision humaine terminale est plus clemente
/// (downgrade ou "ignore").
pub fn compute_fp_stats(days: i64, rows: &[FpTerminalReview], capped: bool) -> FpStats {
    use std::collections::BTreeMap;

    let mut overall = FpAcc::default();
    // Ordre stable (alpha) pour un rendu deterministe.
    let mut by_flag: BTreeMap<String, FpAcc> = BTreeMap::new();
    let mut by_action: BTreeMap<String, FpAcc> = BTreeMap::new();

    for r in rows {
        let suggested_sev = fp_severity(Some(&r.suggested_action));
        // Action humaine terminale : la resolution (applied_action) prime, sinon
        // le verdict de vote (decided_action). Absente => aucune sanction (0).
        let terminal = r.applied_action.as_deref().or(r.decided_action.as_deref());
        let terminal_sev = fp_severity(terminal);

        // Over-block : l'automod a suggere une vraie sanction ET l'humain a
        // tranche plus clement (downgrade ou ignore).
        let overturned = suggested_sev > 0 && terminal_sev < suggested_sev;
        let ignored = terminal == Some("ignore") || terminal.is_none();

        overall.add(overturned, ignored);
        by_action
            .entry(r.suggested_action.clone())
            .or_default()
            .add(overturned, ignored);

        // Explose les flags detecteurs actifs (objet JSONB de booleens).
        if let Some(map) = r.flags.as_object() {
            for (flag, val) in map {
                if val.as_bool() == Some(true) {
                    by_flag
                        .entry(flag.clone())
                        .or_default()
                        .add(overturned, ignored);
                }
            }
        }
    }

    let mut by_flag_dto: Vec<FpFlagStat> = by_flag
        .into_iter()
        .map(|(flag, a)| FpFlagStat {
            flag,
            total: a.total,
            overturned: a.overturned,
            ignored: a.ignored,
            fp_rate: a.rate(),
        })
        .collect();
    // Tri par taux de FP decroissant (les detecteurs les plus "bruyants" en tete).
    by_flag_dto.sort_by(|a, b| {
        b.fp_rate
            .partial_cmp(&a.fp_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.total.cmp(&a.total))
    });

    let by_action_dto: Vec<FpActionStat> = by_action
        .into_iter()
        .map(|(suggested_action, a)| FpActionStat {
            suggested_action,
            total: a.total,
            overturned: a.overturned,
            ignored: a.ignored,
            fp_rate: a.rate(),
        })
        .collect();

    FpStats {
        days,
        capped,
        overall: FpBucket {
            total: overall.total,
            overturned: overall.overturned,
            ignored: overall.ignored,
            fp_rate: overall.rate(),
        },
        by_flag: by_flag_dto,
        by_suggested_action: by_action_dto,
    }
}

#[derive(Debug, Clone)]
pub struct NewAutomodReview {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub user_id: UserId,
    pub user_name: String,
    pub content_preview: String,
    pub suggested_action: SuggestedAction,
    pub score: f64,
    pub reason: String,
    pub flags: serde_json::Value,
    /// Si Some, la review naît en mode VOTE (statut 'voting') avec cette
    /// echeance. Si None, comportement historique (statut 'pending').
    pub voting_deadline: Option<DateTime<Utc>>,
    /// `true` si une sanction de membre a DÉJÀ été journalisée pour cet incident
    /// (ex. l'auto-protection sévère a mute + tracé la sanction AVANT de poster
    /// la carte). Évite le double comptage de strike lors de la finalisation
    /// de la carte (cf. C1). Défaut `false`.
    pub sanction_logged: bool,
}

#[cfg(test)]
#[path = "tests/automod.rs"]
mod tests;
