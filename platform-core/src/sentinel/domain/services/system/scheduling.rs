//! Prédicats purs de planification/rétention (consommés par sentinel-worker).

use chrono::{DateTime, Duration, Utc};

/// Garde ANTI-PURGE-TOTALE : une retention <= 0 rend `NOW() - interval '0 day'`
/// egal a `NOW()` -> `WHERE created_at < NOW()` supprimerait TOUTE la table (et
/// une valeur negative supprimerait meme les lignes futures). On refuse alors
/// d'executer le DELETE : mieux vaut conserver les donnees qu'une purge totale
/// declenchee par une simple case de config erronee.
pub fn valid_retention(days: i64) -> Option<i64> {
    if days >= 1 {
        Some(days)
    } else {
        None
    }
}

/// Vrai si une tache periodique est due : jamais executee, ou l'intervalle est
/// ecoule depuis la derniere execution. Un intervalle <= 0 est traite comme le
/// defaut `default_hours` (garde-fou contre une valeur absurde en config).
pub fn is_due(
    last: Option<DateTime<Utc>>,
    interval_hours: i64,
    default_hours: i64,
    now: DateTime<Utc>,
) -> bool {
    let hours = if interval_hours <= 0 {
        default_hours
    } else {
        interval_hours
    };
    match last {
        None => true,
        Some(last) => now - last >= Duration::hours(hours),
    }
}

/// Convertit une heure de publication locale en heure UTC équivalente.
/// `post_hour` est bornée 0..23, `offset_hours` (décalage vs UTC, ex. +1
/// Paris hiver / +2 été) est borné aux fuseaux réels [-12, +14].
pub fn local_hour_to_utc(post_hour: u64, offset_hours: i64) -> u32 {
    let post_hour = post_hour.min(23) as i64;
    let offset = offset_hours.clamp(-12, 14);
    (post_hour - offset).rem_euclid(24) as u32
}

/// Secondes restantes avant la prochaine heure pile (HH:00:00), pour aligner
/// un tick périodique sur l'heure. Retourne 0 si on est exactement à HH:00:00.
pub fn secs_to_next_hour(minute: u32, second: u32) -> u64 {
    let secs_in_hour = (minute as u64) * 60 + second as u64;
    if secs_in_hour == 0 {
        0
    } else {
        3600 - secs_in_hour
    }
}

/// Ajoute `n` mois a une date UTC (résultat au 1er du mois à minuit), en
/// gerant correctement les overflow d'annee.
pub fn add_months(date: DateTime<Utc>, n: u32) -> DateTime<Utc> {
    use chrono::{Datelike, TimeZone};
    let total_months = date.year() * 12 + date.month0() as i32 + n as i32;
    let new_year = total_months / 12;
    let new_month = (total_months % 12) as u32 + 1;
    Utc.with_ymd_and_hms(new_year, new_month, 1, 0, 0, 0)
        .single()
        .unwrap_or(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_positive_ok() {
        assert_eq!(valid_retention(30), Some(30));
        assert_eq!(valid_retention(1), Some(1));
    }

    #[test]
    fn retention_zero_or_negative_refused() {
        assert_eq!(valid_retention(0), None);
        assert_eq!(valid_retention(-7), None);
    }

    #[test]
    fn due_when_never_run() {
        assert!(is_due(None, 24, 24, Utc::now()));
    }

    #[test]
    fn due_when_interval_elapsed() {
        let now = Utc::now();
        assert!(is_due(Some(now - Duration::hours(25)), 24, 24, now));
        assert!(!is_due(Some(now - Duration::hours(23)), 24, 24, now));
    }

    #[test]
    fn local_hour_conversion() {
        // 9h locale à Paris l'hiver (UTC+1) = 8h UTC.
        assert_eq!(local_hour_to_utc(9, 1), 8);
        // 9h locale UTC-5 = 14h UTC.
        assert_eq!(local_hour_to_utc(9, -5), 14);
        // Wrap par minuit : 1h locale UTC+2 = 23h UTC la veille.
        assert_eq!(local_hour_to_utc(1, 2), 23);
        // Bornes : heure > 23 clampée, offset hors fuseau réel clampé.
        assert_eq!(local_hour_to_utc(99, 0), 23);
        assert_eq!(local_hour_to_utc(9, 99), 19); // offset clampé à +14
    }

    #[test]
    fn next_hour_alignment() {
        assert_eq!(secs_to_next_hour(0, 0), 0); // pile HH:00:00
        assert_eq!(secs_to_next_hour(0, 1), 3599);
        assert_eq!(secs_to_next_hour(59, 59), 1);
        assert_eq!(secs_to_next_hour(30, 0), 1800);
    }

    #[test]
    fn add_months_simple() {
        let d = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 3, 15, 12, 30, 0).unwrap();
        let r = add_months(d, 1);
        assert_eq!(r.to_rfc3339(), "2026-04-01T00:00:00+00:00");
    }

    #[test]
    fn add_months_year_overflow() {
        let d = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 11, 20, 0, 0, 0).unwrap();
        assert_eq!(add_months(d, 2).to_rfc3339(), "2027-01-01T00:00:00+00:00");
        assert_eq!(add_months(d, 14).to_rfc3339(), "2028-01-01T00:00:00+00:00");
    }

    #[test]
    fn add_months_zero_normalizes_to_month_start() {
        let d = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 7, 28, 9, 0, 0).unwrap();
        assert_eq!(add_months(d, 0).to_rfc3339(), "2026-07-01T00:00:00+00:00");
    }

    #[test]
    fn invalid_interval_falls_back_to_default() {
        let now = Utc::now();
        // interval 0 -> défaut 24h : pas dû après 12h, dû après 25h.
        assert!(!is_due(Some(now - Duration::hours(12)), 0, 24, now));
        assert!(is_due(Some(now - Duration::hours(25)), -5, 24, now));
    }
}
