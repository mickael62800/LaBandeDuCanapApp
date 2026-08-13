//! Entites du domaine snapshots analytics : rapport de job et publication
//! "Top users" prete a poster. Le calcul metier vit dans le use case, le SQL
//! dans l'adapter Postgres ; ces structs transportent le resultat.

use serde::Serialize;

/// Rapport standard renvoye par les jobs analytics (snapshot/cleanup/publish).
#[derive(Debug, Clone, Serialize)]
pub struct JobReport {
    pub guilds_processed: usize,
    pub guilds_skipped: usize,
    pub status: &'static str,
}

impl JobReport {
    pub fn ok(guilds_processed: usize, guilds_skipped: usize) -> Self {
        Self {
            guilds_processed,
            guilds_skipped,
            status: "ok",
        }
    }
}

/// Publication "Top infracteurs" calculee par le use case, prete a etre postee
/// par le handler (le POST Discord reste un concern inbound). `published_at` est
/// l'horodatage a persister apres un post reussi.
#[derive(Debug, Clone)]
pub struct TopPublication {
    pub guild_id: String,
    pub channel_id: String,
    pub title: String,
    pub description: String,
    pub color: u32,
    pub published_at: String,
}

/// Plan de publication : les guilds dues + le nombre skip (module off, non
/// active, salon absent, intervalle non ecoule).
#[derive(Debug, Clone)]
pub struct TopPublishPlan {
    pub publications: Vec<TopPublication>,
    pub skipped: usize,
}
