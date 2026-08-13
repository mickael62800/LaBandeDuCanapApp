//! Regles d'eligibilite Community (pures) : prerequis de role et validation de
//! parrainage. Deplacees du bot vers le domaine — la DECISION est server-side ;
//! le bot ne fournit que les donnees Discord (roles actuels, dates de join).

/// Decision d'eligibilite : autorise, ou refus avec une raison affichable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibilityDecision {
    pub allowed: bool,
    /// Raison du refus (message utilisateur). `None` si autorise.
    pub reason: Option<String>,
}

impl EligibilityDecision {
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reason: None,
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
        }
    }
}

/// Prerequis pour obtenir un role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prerequisite {
    RequiresRole(u64),
    MinDays(u64),
}

/// Nombre de jours d'anciennete a partir des timestamps unix (secondes).
/// Reproduit EXACTEMENT le calcul historique du bot :
/// `((now - joined) / 86400).max(0) as u64`.
pub fn days_since(now_unix: i64, joined_unix: i64) -> u64 {
    ((now_unix - joined_unix) / 86_400).max(0) as u64
}

/// Parse les prerequis depuis le format config `role_prerequisites`.
/// Formats supportes par ligne :
/// - `role_id:requires_role:other_role_id`
/// - `role_id:min_days:N`
pub fn parse_prerequisites(raw: &str) -> Vec<(u64, Vec<Prerequisite>)> {
    let mut result: Vec<(u64, Vec<Prerequisite>)> = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 3 {
            continue;
        }

        let role_id: u64 = match parts[0].trim().parse() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let prereq = match parts[1].trim() {
            "requires_role" => match parts[2].trim().parse::<u64>() {
                Ok(rid) => Prerequisite::RequiresRole(rid),
                Err(_) => continue,
            },
            "min_days" => match parts[2].trim().parse::<u64>() {
                Ok(d) => Prerequisite::MinDays(d),
                Err(_) => continue,
            },
            _ => continue,
        };

        if let Some(entry) = result.iter_mut().find(|(id, _)| *id == role_id) {
            entry.1.push(prereq);
        } else {
            result.push((role_id, vec![prereq]));
        }
    }

    result
}

/// Evalue les prerequis d'un role. Retourne une decision (autorise / refus).
pub fn check_prerequisites(
    prereqs: &[(u64, Vec<Prerequisite>)],
    role_id: u64,
    user_roles: &[u64],
    joined_days: u64,
) -> EligibilityDecision {
    let entry = match prereqs.iter().find(|(id, _)| *id == role_id) {
        Some(e) => e,
        None => return EligibilityDecision::allow(), // Pas de prerequis
    };

    for prereq in &entry.1 {
        match prereq {
            Prerequisite::RequiresRole(required_role) => {
                if !user_roles.contains(required_role) {
                    return EligibilityDecision::deny(format!(
                        "Vous devez avoir le role <@&{}> pour obtenir ce role.",
                        required_role
                    ));
                }
            }
            Prerequisite::MinDays(min) => {
                if joined_days < *min {
                    return EligibilityDecision::deny(format!(
                        "Vous devez etre dans le serveur depuis au moins {} jours (actuellement {} jours).",
                        min, joined_days
                    ));
                }
            }
        }
    }

    EligibilityDecision::allow()
}

/// Evalue les regles d'eligibilite d'un parrainage (anti-self + seuils
/// d'anciennete). Les checks Discord (bot, appartenance, deja parraine) restent
/// cote bot ; ici uniquement les regles de config.
///
/// - `sponsor_days` : anciennete du parrain en jours (voir `days_since`).
/// - `sponsored_days` : anciennete du filleul en jours.
/// - `min_parrain_days` / `max_filleul_days` : seuils de config.
pub fn evaluate_sponsorship(
    sponsor_id: u64,
    sponsored_id: u64,
    sponsor_days: u64,
    sponsored_days: u64,
    min_parrain_days: u64,
    max_filleul_days: u64,
) -> EligibilityDecision {
    // 1. Anti self-sponsor
    if sponsored_id == sponsor_id {
        return EligibilityDecision::deny(
            "\u{274c} Vous ne pouvez pas vous parrainer vous-meme.".to_string(),
        );
    }

    // 2. Parrain doit etre sur le serveur depuis >= min_parrain_days jours
    if sponsor_days < min_parrain_days {
        let remaining = min_parrain_days - sponsor_days;
        return EligibilityDecision::deny(format!(
            "\u{274c} Le parrain doit etre membre depuis au moins **{min_parrain_days} jours**. \
             Encore **{remaining}** jour(s) a attendre."
        ));
    }

    // 3. Filleul doit etre un membre recent (<= max_filleul_days jours)
    if sponsored_days > max_filleul_days {
        return EligibilityDecision::deny(format!(
            "\u{274c} <@{}> est sur le serveur depuis plus de **{max_filleul_days} jours**, \
             il n'est plus eligible au parrainage.",
            sponsored_id
        ));
    }

    EligibilityDecision::allow()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_role() {
        let raw = "100:requires_role:200";
        let prereqs = parse_prerequisites(raw);
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].0, 100);
        assert_eq!(prereqs[0].1, vec![Prerequisite::RequiresRole(200)]);
    }

    #[test]
    fn parse_min_days() {
        let raw = "100:min_days:30";
        let prereqs = parse_prerequisites(raw);
        assert_eq!(prereqs[0].1, vec![Prerequisite::MinDays(30)]);
    }

    #[test]
    fn parse_multiple_for_same_role() {
        let raw = "100:requires_role:200\n100:min_days:7";
        let prereqs = parse_prerequisites(raw);
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].1.len(), 2);
    }

    #[test]
    fn parse_multiple_roles() {
        let raw = "100:requires_role:200\n300:min_days:7";
        let prereqs = parse_prerequisites(raw);
        assert_eq!(prereqs.len(), 2);
    }

    #[test]
    fn parse_ignores_invalid() {
        let raw = "invalid\n100:unknown:x\n200:requires_role:300";
        let prereqs = parse_prerequisites(raw);
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].0, 200);
    }

    #[test]
    fn parse_empty() {
        assert!(parse_prerequisites("").is_empty());
    }

    #[test]
    fn check_no_prereqs_passes() {
        let prereqs = vec![];
        assert!(check_prerequisites(&prereqs, 100, &[], 0).allowed);
    }

    #[test]
    fn check_requires_role_passes() {
        let prereqs = vec![(100, vec![Prerequisite::RequiresRole(200)])];
        assert!(check_prerequisites(&prereqs, 100, &[200, 300], 0).allowed);
    }

    #[test]
    fn check_requires_role_fails() {
        let prereqs = vec![(100, vec![Prerequisite::RequiresRole(200)])];
        let d = check_prerequisites(&prereqs, 100, &[300], 0);
        assert!(!d.allowed);
        assert!(d.reason.unwrap().contains("200"));
    }

    #[test]
    fn check_min_days_passes() {
        let prereqs = vec![(100, vec![Prerequisite::MinDays(30)])];
        assert!(check_prerequisites(&prereqs, 100, &[], 30).allowed);
    }

    #[test]
    fn check_min_days_fails() {
        let prereqs = vec![(100, vec![Prerequisite::MinDays(30)])];
        assert!(!check_prerequisites(&prereqs, 100, &[], 10).allowed);
    }

    #[test]
    fn check_multiple_prereqs_all_pass() {
        let prereqs = vec![(
            100,
            vec![Prerequisite::RequiresRole(200), Prerequisite::MinDays(7)],
        )];
        assert!(check_prerequisites(&prereqs, 100, &[200], 10).allowed);
    }

    #[test]
    fn check_multiple_prereqs_one_fails() {
        let prereqs = vec![(
            100,
            vec![Prerequisite::RequiresRole(200), Prerequisite::MinDays(30)],
        )];
        assert!(!check_prerequisites(&prereqs, 100, &[200], 10).allowed);
    }

    #[test]
    fn check_role_not_in_prereqs_passes() {
        let prereqs = vec![(100, vec![Prerequisite::MinDays(30)])];
        assert!(check_prerequisites(&prereqs, 999, &[], 0).allowed);
    }

    #[test]
    fn days_since_matches_bot_formula() {
        // 10 jours pile
        assert_eq!(days_since(864_000, 0), 10);
        // negatif clampe a 0
        assert_eq!(days_since(0, 864_000), 0);
    }

    #[test]
    fn sponsorship_anti_self() {
        let d = evaluate_sponsorship(42, 42, 100, 1, 7, 30);
        assert!(!d.allowed);
        assert!(d.reason.unwrap().contains("vous-meme"));
    }

    #[test]
    fn sponsorship_parrain_too_recent() {
        let d = evaluate_sponsorship(1, 2, 3, 1, 7, 30);
        assert!(!d.allowed);
        assert!(d.reason.unwrap().contains("Encore **4** jour"));
    }

    #[test]
    fn sponsorship_filleul_too_old() {
        let d = evaluate_sponsorship(1, 2, 10, 45, 7, 30);
        assert!(!d.allowed);
        assert!(d.reason.unwrap().contains("plus de **30 jours**"));
    }

    #[test]
    fn sponsorship_ok() {
        let d = evaluate_sponsorship(1, 2, 10, 5, 7, 30);
        assert!(d.allowed);
        assert!(d.reason.is_none());
    }

    #[test]
    fn sponsorship_boundaries() {
        // parrain pile au seuil OK, filleul pile au seuil OK
        assert!(evaluate_sponsorship(1, 2, 7, 30, 7, 30).allowed);
    }
}
