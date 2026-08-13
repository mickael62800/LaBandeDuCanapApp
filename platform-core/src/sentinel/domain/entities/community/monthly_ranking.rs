//! Entites du classement mensuel d'activite (texte / vocal / global).
//!
//! Modele "baseline" : `user_levels` ne stocke que l'XP CUMULEE. Au debut de
//! chaque mois on capture une baseline (snapshot). Le classement du mois ecoule
//! = XP actuelle - baseline du debut de ce mois. Tout ce module est PUR (pas de
//! SQL, pas d'IO) : l'assemblage du classement est de la logique metier.

use chrono::{DateTime, Datelike, Utc};

/// Delta d'XP (texte / vocal) d'un membre pour une periode donnee.
#[derive(Debug, Clone)]
pub struct RankingRow {
    pub user_id: String,
    pub d_text: i64,
    pub d_voice: i64,
}

/// Une ligne de classement structuree (rendu delegue a l'appelant).
#[derive(Debug, Clone)]
pub struct RankingEntry {
    pub user_id: String,
    pub xp: i64,
}

/// Donnees d'un classement forme (publication forcee) : les trois tops +
/// libelle du mois + note eventuelle (fallback cumul total).
#[derive(Debug, Clone)]
pub struct MonthlyRankingData {
    pub period_label: String,
    pub note: Option<String>,
    pub text: Vec<RankingEntry>,
    pub voice: Vec<RankingEntry>,
    pub global: Vec<RankingEntry>,
}

/// Un classement pret a poster sur Discord (publication auto d'un mois complet).
/// Les blocs sont deja rendus (top N, deltas > 0) ; l'appelant construit l'embed.
#[derive(Debug, Clone)]
pub struct MonthlyPublishItem {
    pub guild_id: String,
    pub channel_id: String,
    /// Periode de reference (`YYYY-MM`) — sert au callback `mark_published`.
    pub period: String,
    pub period_label: String,
    pub text_block: String,
    pub voice_block: String,
    pub global_block: String,
}

/// Plan de publication mensuelle : classements a poster + compteurs (baseline
/// posee / guilds ecartees). Le nombre de publications reellement postees est
/// determine par l'appelant apres l'envoi Discord.
#[derive(Debug, Clone, Default)]
pub struct MonthlyPublishPlan {
    pub publications: Vec<MonthlyPublishItem>,
    pub baselined: usize,
    pub skipped: usize,
}

/// Formate une periode `YYYY-MM` a partir d'une annee et d'un mois.
pub fn period_string(year: i32, month: u32) -> String {
    format!("{:04}-{:02}", year, month)
}

/// `(periode courante, periode precedente)` a partir d'un instant.
pub fn current_and_prev_periods(now: DateTime<Utc>) -> (String, String) {
    let this = period_string(now.year(), now.month());
    let (py, pm) = if now.month() == 1 {
        (now.year() - 1, 12)
    } else {
        (now.year(), now.month() - 1)
    };
    (this, period_string(py, pm))
}

/// Libelle FR d'une periode `YYYY-MM` (ex: "Juillet 2026"). Periode invalide ->
/// renvoyee telle quelle.
pub fn month_label_fr(period: &str) -> String {
    const MOIS: [&str; 12] = [
        "Janvier",
        "Fevrier",
        "Mars",
        "Avril",
        "Mai",
        "Juin",
        "Juillet",
        "Aout",
        "Septembre",
        "Octobre",
        "Novembre",
        "Decembre",
    ];
    let parts: Vec<&str> = period.split('-').collect();
    if parts.len() == 2 {
        if let (Ok(y), Ok(m)) = (parts[0].parse::<i32>(), parts[1].parse::<usize>()) {
            if (1..=12).contains(&m) {
                return format!("{} {}", MOIS[m - 1], y);
            }
        }
    }
    period.to_string()
}

/// Construit un bloc de classement Discord (top N, deltas > 0 uniquement).
pub fn build_ranking_block(mut rows: Vec<(String, i64)>, top: usize) -> String {
    rows.sort_by_key(|r| std::cmp::Reverse(r.1));
    let lines: Vec<String> = rows
        .into_iter()
        .filter(|(_, xp)| *xp > 0)
        .take(top)
        .enumerate()
        .map(|(i, (uid, xp))| format!("**{}.** <@{}> — {} XP", i + 1, uid, xp))
        .collect();
    if lines.is_empty() {
        "_Aucune activite ce mois-ci._".to_string()
    } else {
        lines.join("\n")
    }
}

/// Top N (deltas > 0), trie decroissant, selon la cle donnee (texte / vocal /
/// global). Renvoie une liste structuree pour que l'appelant fasse le rendu.
pub fn top_entries(
    rows: &[RankingRow],
    top: usize,
    key: impl Fn(i64, i64) -> i64,
) -> Vec<RankingEntry> {
    let mut entries: Vec<RankingEntry> = rows
        .iter()
        .map(|r| RankingEntry {
            user_id: r.user_id.clone(),
            xp: key(r.d_text, r.d_voice),
        })
        .filter(|e| e.xp > 0)
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.xp));
    entries.truncate(top);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(uid: &str, t: i64, v: i64) -> RankingRow {
        RankingRow {
            user_id: uid.to_string(),
            d_text: t,
            d_voice: v,
        }
    }

    #[test]
    fn month_label_fr_formats_period() {
        assert_eq!(month_label_fr("2026-07"), "Juillet 2026");
        assert_eq!(month_label_fr("2026-01"), "Janvier 2026");
        assert_eq!(month_label_fr("bogus"), "bogus");
    }

    #[test]
    fn period_string_zero_pads() {
        assert_eq!(period_string(2026, 7), "2026-07");
        assert_eq!(period_string(2026, 12), "2026-12");
    }

    #[test]
    fn top_entries_sorts_filters_and_truncates() {
        let rows = vec![
            row("a", 10, 5),
            row("b", 30, 0),
            row("c", 0, 40),
            row("d", -5, 0),
        ];

        let text = top_entries(&rows, 2, |t, _| t);
        assert_eq!(text.len(), 2);
        assert_eq!(text[0].user_id, "b");
        assert_eq!(text[0].xp, 30);
        assert_eq!(text[1].user_id, "a");

        let global = top_entries(&rows, 10, |t, v| t + v);
        assert_eq!(global.len(), 3);
        assert_eq!(global[0].user_id, "c");
        assert_eq!(global[0].xp, 40);
    }

    #[test]
    fn top_entries_empty_when_no_positive() {
        let rows = vec![row("a", 0, 0), row("b", -1, -1)];
        assert!(top_entries(&rows, 10, |t, v| t + v).is_empty());
    }

    #[test]
    fn build_ranking_block_empty_placeholder() {
        assert_eq!(
            build_ranking_block(vec![("a".into(), 0)], 10),
            "_Aucune activite ce mois-ci._"
        );
    }
}
