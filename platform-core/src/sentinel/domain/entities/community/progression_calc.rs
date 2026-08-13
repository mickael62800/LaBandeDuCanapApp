//! Calcul PUR de l'XP progression (porte depuis l'ancien `sentinel-bot`).
//!
//! Regroupe les formules historiques du bot pour qu'elles vivent cote API :
//! - selection des multiplicateurs channel/role,
//! - montant final `base x channel x role x streak` clampe,
//! - calcul du streak (jours consecutifs) + multiplicateur bonus.
//!
//! Aucune I/O ici : toutes les entrees (config, etat streak persiste) sont
//! passees en argument.

/// Etat de streak persiste pour un utilisateur.
#[derive(Debug, Clone, Copy, Default)]
pub struct StreakState {
    pub current: u32,
    pub best: u32,
    pub last_day: u32,
    pub last_year: i32,
}

/// Resultat du calcul de streak pour une nouvelle activite.
#[derive(Debug, Clone, Copy)]
pub struct StreakOutcome {
    /// `true` si c'est un nouveau jour d'activite (persistance a mettre a jour).
    pub new_day: bool,
    pub current: u32,
    pub best: u32,
    /// Multiplicateur XP bonus (1.0 = aucun bonus).
    pub multiplier: f64,
}

/// Bonus de multiplicateur XP par semaine complete de streak (defaut historique).
pub const DEFAULT_STREAK_BONUS_PER_WEEK: f64 = 0.1;
/// Plafond du multiplicateur XP de streak (defaut historique).
pub const DEFAULT_STREAK_MAX_MULTIPLIER: f64 = 1.5;

/// Parse les multiplicateurs depuis le format config : "id:multiplier" par
/// ligne. Ignore les lignes invalides et les multiplicateurs <= 0.
pub fn parse_multipliers(raw: &str) -> Vec<(u64, f64)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (id_str, val_str) = line.split_once(':')?;
            let id: u64 = id_str.trim().parse().ok()?;
            let val: f64 = val_str.trim().parse().ok()?;
            Some((id, val))
        })
        .filter(|(_, v)| *v > 0.0)
        .collect()
}

/// Retourne le multiplicateur pour un channel (defaut 1.0).
pub fn get_channel_multiplier(multipliers: &[(u64, f64)], channel_id: u64) -> f64 {
    multipliers
        .iter()
        .find(|(k, _)| *k == channel_id)
        .map(|(_, v)| *v)
        .unwrap_or(1.0)
}

/// Retourne le meilleur multiplicateur parmi les roles de l'utilisateur (defaut 1.0).
pub fn get_role_multiplier(multipliers: &[(u64, f64)], user_roles: &[u64]) -> f64 {
    let mut best = 1.0f64;
    for role_id in user_roles {
        if let Some((_, mult)) = multipliers.iter().find(|(id, _)| id == role_id) {
            if *mult > best {
                best = *mult;
            }
        }
    }
    best
}

/// Calcul XP final unifie (texte + voice) appliquant `base x channel x role x streak`.
/// Le clamp evite qu'un boost donne 0 (clamp_min) ou explose (clamp_max).
/// Retourne un i64 borne aux limites de clamp.
pub fn calc_xp_amount(
    base_xp: f64,
    channel_mult: f64,
    role_mult: f64,
    streak_mult: f64,
    clamp_min: f64,
    clamp_max: f64,
) -> i64 {
    (base_xp * channel_mult * role_mult * streak_mult)
        .round()
        .clamp(clamp_min, clamp_max) as i64
}

/// Multiplicateur XP de streak parametrable (fonction pure). Le plafond est
/// garde >= 1.0 pour ne jamais reduire l'XP en dessous du base.
pub fn streak_multiplier_with(streak_days: u32, bonus_per_week: f64, max_multiplier: f64) -> f64 {
    let max_multiplier = max_multiplier.max(1.0);
    let bonus = (streak_days / 7) as f64 * bonus_per_week.max(0.0);
    (1.0 + bonus).min(max_multiplier)
}

/// Verifie si (day2, year2) est le jour suivant (day1, year1).
fn is_next_day(day1: u32, year1: i32, day2: u32, year2: i32) -> bool {
    if year1 == year2 {
        day2 == day1 + 1
    } else if year2 == year1 + 1 {
        day1 >= 365 && day2 == 1
    } else {
        false
    }
}

/// Calcule la mise a jour du streak a partir de l'etat persiste et du jour
/// courant. Reproduit fidelement `StreakTracker::record_activity` du bot.
pub fn compute_streak(
    state: StreakState,
    today_day: u32,
    today_year: i32,
    bonus_per_week: f64,
    max_multiplier: f64,
) -> StreakOutcome {
    // Meme jour -> pas de mise a jour.
    if state.last_day == today_day && state.last_year == today_year {
        return StreakOutcome {
            new_day: false,
            current: state.current,
            best: state.best,
            multiplier: streak_multiplier_with(state.current, bonus_per_week, max_multiplier),
        };
    }

    let current = if is_next_day(state.last_day, state.last_year, today_day, today_year) {
        state.current + 1
    } else {
        1
    };
    let best = current.max(state.best);

    StreakOutcome {
        new_day: true,
        current,
        best,
        multiplier: streak_multiplier_with(current, bonus_per_week, max_multiplier),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_neutral_returns_base() {
        assert_eq!(calc_xp_amount(15.0, 1.0, 1.0, 1.0, 1.0, 1000.0), 15);
    }

    #[test]
    fn calc_vip_in_nerf_channel_returns_normal() {
        assert_eq!(calc_xp_amount(15.0, 0.5, 2.0, 1.0, 1.0, 1000.0), 15);
    }

    #[test]
    fn calc_channel_x05_halves() {
        assert_eq!(calc_xp_amount(15.0, 0.5, 1.0, 1.0, 1.0, 1000.0), 8);
    }

    #[test]
    fn calc_with_streak_compounds() {
        assert_eq!(calc_xp_amount(15.0, 2.0, 2.0, 1.5, 1.0, 1000.0), 90);
    }

    #[test]
    fn calc_clamp_min_protects_against_zero() {
        assert_eq!(calc_xp_amount(0.49, 1.0, 1.0, 1.0, 1.0, 1000.0), 1);
    }

    #[test]
    fn calc_clamp_max_caps() {
        assert_eq!(calc_xp_amount(100.0, 100.0, 1.0, 1.0, 1.0, 1000.0), 1000);
    }

    #[test]
    fn calc_voice_15_minutes_x2_channel() {
        let base = 15.0 * 5.0;
        assert_eq!(calc_xp_amount(base, 2.0, 1.0, 1.0, 0.0, 100_000.0), 150);
    }

    #[test]
    fn multipliers_parse_and_lookup() {
        let mults = parse_multipliers("123:2.0\n456:1.5\n789:0.0");
        assert_eq!(mults.len(), 2);
        assert_eq!(get_channel_multiplier(&mults, 123), 2.0);
        assert_eq!(get_channel_multiplier(&mults, 999), 1.0);
        assert_eq!(get_role_multiplier(&mults, &[456, 999]), 1.5);
        assert_eq!(get_role_multiplier(&mults, &[999]), 1.0);
    }

    #[test]
    fn streak_multiplier_steps() {
        assert_eq!(
            streak_multiplier_with(
                6,
                DEFAULT_STREAK_BONUS_PER_WEEK,
                DEFAULT_STREAK_MAX_MULTIPLIER
            ),
            1.0
        );
        assert_eq!(
            streak_multiplier_with(
                7,
                DEFAULT_STREAK_BONUS_PER_WEEK,
                DEFAULT_STREAK_MAX_MULTIPLIER
            ),
            1.1
        );
        assert_eq!(
            streak_multiplier_with(
                100,
                DEFAULT_STREAK_BONUS_PER_WEEK,
                DEFAULT_STREAK_MAX_MULTIPLIER
            ),
            1.5
        );
    }

    #[test]
    fn streak_first_activity() {
        let out = compute_streak(
            StreakState::default(),
            42,
            2025,
            DEFAULT_STREAK_BONUS_PER_WEEK,
            DEFAULT_STREAK_MAX_MULTIPLIER,
        );
        assert!(out.new_day);
        assert_eq!(out.current, 1);
        assert_eq!(out.best, 1);
    }

    #[test]
    fn streak_same_day_no_update() {
        let state = StreakState {
            current: 3,
            best: 5,
            last_day: 42,
            last_year: 2025,
        };
        let out = compute_streak(state, 42, 2025, 0.1, 1.5);
        assert!(!out.new_day);
        assert_eq!(out.current, 3);
    }

    #[test]
    fn streak_consecutive_and_break() {
        let state = StreakState {
            current: 3,
            best: 5,
            last_day: 42,
            last_year: 2025,
        };
        let next = compute_streak(state, 43, 2025, 0.1, 1.5);
        assert_eq!(next.current, 4);
        assert_eq!(next.best, 5);
        let broken = compute_streak(state, 45, 2025, 0.1, 1.5);
        assert_eq!(broken.current, 1);
    }

    #[test]
    fn streak_across_year() {
        let state = StreakState {
            current: 2,
            best: 2,
            last_day: 365,
            last_year: 2025,
        };
        let out = compute_streak(state, 1, 2026, 0.1, 1.5);
        assert_eq!(out.current, 3);
    }
}
