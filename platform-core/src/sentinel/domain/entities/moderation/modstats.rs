//! Entites d'agregation des statistiques de moderation (lecture seule).
//! Alimentees depuis `audit_logs` (event_type `mod_*`).

/// Agregation par moderateur sur une fenetre glissante (top N).
#[derive(Debug, Clone)]
pub struct ModeratorBreakdown {
    pub moderator_id: String,
    pub moderator_name: String,
    pub total: i64,
    pub warns: i64,
    pub mutes: i64,
    pub bans: i64,
    pub kicks: i64,
}

/// Comptes d'actions agreges pour un jour donne (courbe de tendance).
#[derive(Debug, Clone)]
pub struct ModstatsTrendDay {
    pub day: chrono::NaiveDate,
    pub warns: i64,
    pub mutes: i64,
    pub bans: i64,
    pub kicks: i64,
}
