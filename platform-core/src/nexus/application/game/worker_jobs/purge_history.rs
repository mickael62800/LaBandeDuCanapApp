use super::*;

// ════════════════════════════════════════════════════════════════════════
// JOB : PURGE DE L'HISTORIQUE DE SURVEILLANCE
// ════════════════════════════════════════════════════════════════════════

/// Retention par defaut de l'historique de surveillance.
///
/// Sept jours couvrent la question qu'on pose a ces courbes — « qu'est-ce qui
/// s'est passe hier soir ? », « est-ce que ca derive depuis lundi ? » — sans
/// laisser une table de series temporelles grossir sans fin. Un serveur en
/// ligne y ecrit 2 880 lignes par jour.
pub const RETENTION_JOURS_DEFAUT: i32 = 7;

/// Efface les points de surveillance trop vieux.
///
/// Volontairement separe du controle de sante : celui-ci tourne toutes les
/// trente secondes et doit rester court. Balayer la table a chaque passage
/// couterait cher pour un menage qui peut attendre la nuit.
pub async fn run_purge_history(
    ctx: &JobContext,
    retention_jours: i32,
) -> Result<JobReport, DomainError> {
    // Une retention nulle ou negative effacerait tout, y compris la mesure
    // prise il y a une seconde. On refuse plutot que d'obeir : un historique
    // vide ne se reconstruit pas.
    let jours = retention_jours.max(1);
    let effaces = ctx.server_repo.purge_history(jours).await?;

    if effaces > 0 {
        info!(effaces, jours, "historique de surveillance purge");
    }

    Ok(JobReport {
        job: "purge_history",
        processed: effaces as usize,
        errors: 0,
        details: serde_json::json!({ "retention_jours": jours }),
    })
}
