//! Annonce planifiee : message Discord poste automatiquement par le bot
//! a une frequence configuree. Logique de recurrence + interpolation
//! des variables centralisee dans ce fichier (testable unitairement).

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurrenceType {
    Once,
    Daily,
    Weekly,
    Monthly,
    /// Une fois par an, a une date (mois + jour) et heure fixes. Pour les
    /// annonces saisonnieres (ete, hiver, anniversaire du serveur...).
    Yearly,
}

impl RecurrenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "once" => Some(Self::Once),
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            "yearly" => Some(Self::Yearly),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Success,
    Danger,
    Link,
}

impl ButtonStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Success => "success",
            Self::Danger => "danger",
            Self::Link => "link",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "primary" => Some(Self::Primary),
            "secondary" => Some(Self::Secondary),
            "success" => Some(Self::Success),
            "danger" => Some(Self::Danger),
            "link" => Some(Self::Link),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncementButton {
    pub label: String,
    pub style: String, // "primary" | "secondary" | "success" | "danger" | "link"
    pub custom_id: Option<String>,
    pub url: Option<String>,
    pub emoji: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Text,
    Embed,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Embed => "embed",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "embed" => Some(Self::Embed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScheduledAnnouncement {
    pub id: Uuid,
    pub guild_id: String,
    pub name: String,
    pub enabled: bool,

    pub recurrence_type: RecurrenceType,
    pub recurrence_hour: u8,
    pub recurrence_minute: u8,
    pub recurrence_day_of_week: Option<u8>,
    pub recurrence_day_of_month: Option<u8>,
    /// Mois (1-12) pour la recurrence annuelle. None sinon.
    pub recurrence_month: Option<u8>,
    pub scheduled_at: Option<DateTime<Utc>>,

    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,

    pub content_type: ContentType,
    pub content_text: String,
    pub embed_title: Option<String>,
    pub embed_color: Option<i32>,
    pub embed_image_url: Option<String>,
    pub embed_thumbnail_url: Option<String>,
    /// Texte de pied d'embed. Discord le rend SOUS l'image : c'est la zone
    /// "texte du bas" (petit, gris, sans markdown, 2048 caracteres max).
    pub embed_footer_text: Option<String>,

    pub mention_everyone: bool,
    pub mention_here: bool,
    pub mention_role_ids: Vec<String>,

    pub channel_ids: Vec<String>,

    /// Boutons interactifs (max 5). Vide si aucun.
    pub buttons: Vec<AnnouncementButton>,
    /// Emojis a ajouter en reaction au message apres post. Vide si aucun.
    pub auto_reactions: Vec<String>,

    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Success,
    Partial,
    Error,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Success => "success",
            Self::Partial => "partial",
            Self::Error => "error",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "success" => Some(Self::Success),
            "partial" => Some(Self::Partial),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// Trace une interaction d'un user sur un bouton de l'annonce.
#[derive(Debug, Clone)]
pub struct ButtonInteraction {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub run_id: Option<Uuid>,
    pub user_id: String,
    pub user_name: Option<String>,
    pub button_custom_id: String,
    pub button_label: Option<String>,
    pub clicked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPostResult {
    pub channel_id: String,
    pub message_id: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnnouncementRun {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub guild_id: String,
    pub ran_at: DateTime<Utc>,
    pub channels_posted: Vec<ChannelPostResult>,
    pub status: RunStatus,
    pub error: Option<String>,
}

// ── Calcul de la prochaine execution ────────────────────────────────────

/// Calcule le prochain `next_run_at` pour une annonce, a partir d'un
/// instant donne (typiquement `Utc::now()` apres un run).
///
/// Renvoie None si :
/// - `recurrence_type == Once` et l'annonce a deja tourne
/// - `end_date` defini et tous les futurs runs seraient au-dela
///
/// Toutes les comparaisons sont en UTC. La conversion fuseau horaire
/// (si on en ajoute un jour) doit se faire au niveau presentation.
#[allow(clippy::too_many_arguments)]
pub fn compute_next_run_at(
    recurrence_type: RecurrenceType,
    hour: u8,
    minute: u8,
    day_of_week: Option<u8>,
    day_of_month: Option<u8>,
    month: Option<u8>,
    scheduled_at: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    after: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let candidate = match recurrence_type {
        RecurrenceType::Once => scheduled_at?,
        RecurrenceType::Daily => next_daily(hour, minute, after),
        RecurrenceType::Weekly => next_weekly(day_of_week?, hour, minute, after),
        RecurrenceType::Monthly => next_monthly(day_of_month?, hour, minute, after),
        RecurrenceType::Yearly => next_yearly(month?, day_of_month?, hour, minute, after),
    };

    // Pour 'once', si le scheduled_at est passe, on ne reprogramme pas.
    if recurrence_type == RecurrenceType::Once && candidate <= after {
        return None;
    }

    // Plage de validite : si end_date dépasse, plus de runs.
    if let Some(end) = end_date {
        if candidate > end {
            return None;
        }
    }

    Some(candidate)
}

fn next_daily(hour: u8, minute: u8, after: DateTime<Utc>) -> DateTime<Utc> {
    let today = after.date_naive();
    let candidate = at_hour_minute(today, hour, minute);
    if candidate > after {
        candidate
    } else {
        at_hour_minute(today + Duration::days(1), hour, minute)
    }
}

fn next_weekly(day_of_week: u8, hour: u8, minute: u8, after: DateTime<Utc>) -> DateTime<Utc> {
    // chrono::Datelike::weekday : Mon=0 ... Sun=6 via num_days_from_monday()
    let today = after.date_naive();
    let today_dow = today.weekday().num_days_from_monday() as u8;
    let delta = (day_of_week + 7 - today_dow) % 7;
    let candidate = at_hour_minute(today + Duration::days(delta as i64), hour, minute);
    if delta == 0 && candidate <= after {
        // Aujourd'hui mais l'heure est passee -> +7j.
        return at_hour_minute(today + Duration::days(7), hour, minute);
    }
    candidate
}

fn next_monthly(day_of_month: u8, hour: u8, minute: u8, after: DateTime<Utc>) -> DateTime<Utc> {
    // Construit le candidat dans le mois courant. Si le mois a moins de
    // jours que `day_of_month` (ex 31 fevrier), on clampe au dernier jour
    // du mois courant (28/29) plutot que de sauter au mois suivant.
    let year = after.year();
    let month = after.month();

    let day = day_of_month.min(last_day_of_month(year, month));
    if let Some(candidate) = at_year_month_day_hour_minute(year, month, day, hour, minute) {
        if candidate > after {
            return candidate;
        }
    }

    // Sinon : mois suivant, avec le meme clamp au dernier jour du mois.
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let next_day = day_of_month.min(last_day_of_month(next_year, next_month));
    at_year_month_day_hour_minute(next_year, next_month, next_day, hour, minute)
        .expect("jour clampe au dernier jour du mois toujours valide")
}

fn next_yearly(
    month: u8,
    day_of_month: u8,
    hour: u8,
    minute: u8,
    after: DateTime<Utc>,
) -> DateTime<Utc> {
    // Candidat cette annee. Clamp du jour au dernier jour du mois vise (ex.
    // 29 fevrier une annee non bissextile -> 28).
    let year = after.year();
    let day = day_of_month.min(last_day_of_month(year, month as u32));
    if let Some(candidate) = at_year_month_day_hour_minute(year, month as u32, day, hour, minute) {
        if candidate > after {
            return candidate;
        }
    }
    // Sinon : l'annee prochaine, meme clamp.
    let next_day = day_of_month.min(last_day_of_month(year + 1, month as u32));
    at_year_month_day_hour_minute(year + 1, month as u32, next_day, hour, minute)
        .expect("jour clampe au dernier jour du mois toujours valide")
}

fn at_hour_minute(date: NaiveDate, hour: u8, minute: u8) -> DateTime<Utc> {
    Utc.from_utc_datetime(
        &date
            .and_hms_opt(hour as u32, minute as u32, 0)
            .expect("hour/minute invariants validates en amont"),
    )
}

fn at_year_month_day_hour_minute(
    year: i32,
    month: u32,
    day: u8,
    hour: u8,
    minute: u8,
) -> Option<DateTime<Utc>> {
    NaiveDate::from_ymd_opt(year, month, day as u32)
        .and_then(|d| d.and_hms_opt(hour as u32, minute as u32, 0))
        .map(|dt| Utc.from_utc_datetime(&dt))
}

fn last_day_of_month(year: i32, month: u32) -> u8 {
    let (next_y, next_m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_next = NaiveDate::from_ymd_opt(next_y, next_m, 1).expect("valid");
    (first_next - Duration::days(1)).day() as u8
}

// ── Interpolation des variables {date}, {day}, {week}, {month}, ... ─────

/// Variables disponibles pour interpoler dans content_text / embed_title /
/// description embed.
#[derive(Debug, Clone)]
pub struct InterpolationContext<'a> {
    pub now: DateTime<Utc>,
    pub guild_name: &'a str,
}

/// Remplace les `{var}` dans `template` par leur valeur. Les variables
/// inconnues sont laissees telles quelles (ne crash pas si l'user tape
/// `{moncoco}` par accident).
pub fn render_template(template: &str, ctx: &InterpolationContext) -> String {
    let day_name_fr = match ctx.now.weekday() {
        chrono::Weekday::Mon => "lundi",
        chrono::Weekday::Tue => "mardi",
        chrono::Weekday::Wed => "mercredi",
        chrono::Weekday::Thu => "jeudi",
        chrono::Weekday::Fri => "vendredi",
        chrono::Weekday::Sat => "samedi",
        chrono::Weekday::Sun => "dimanche",
    };
    let month_name_fr = month_name_fr(ctx.now.month());
    let mut out = template.to_string();
    let replacements: [(&str, String); 9] = [
        ("{date}", ctx.now.format("%Y-%m-%d").to_string()),
        ("{day}", format!("{:02}", ctx.now.day())),
        ("{day_name}", day_name_fr.to_string()),
        ("{week}", format!("{:02}", ctx.now.iso_week().week())),
        ("{month}", format!("{:02}", ctx.now.month())),
        ("{month_name}", month_name_fr.to_string()),
        ("{year}", format!("{}", ctx.now.year())),
        ("{time}", ctx.now.format("%H:%M").to_string()),
        ("{guild_name}", ctx.guild_name.to_string()),
    ];
    for (k, v) in replacements {
        out = out.replace(k, &v);
    }
    out
}

fn month_name_fr(m: u32) -> &'static str {
    match m {
        1 => "janvier",
        2 => "fevrier",
        3 => "mars",
        4 => "avril",
        5 => "mai",
        6 => "juin",
        7 => "juillet",
        8 => "aout",
        9 => "septembre",
        10 => "octobre",
        11 => "novembre",
        12 => "decembre",
        _ => "?",
    }
}

#[cfg(test)]
#[path = "tests/announcement.rs"]
mod tests;
