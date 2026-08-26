//! Reprise des annonces d'ouverture qui n'ont pas pu etre publiees.
//!
//! POURQUOI CE JOB EXISTE. L'annonce redigee par Atrium precede le panneau
//! d'inscription : quand Atrium ne peut rien ecrire, le bot ne publie NI
//! l'annonce NI le panneau. Sans reprise, une panne de quelques minutes
//! laisserait la session muette pour toujours — salons crees, personne capable
//! de s'inscrire, et aucune trace expliquant pourquoi.
//!
//! CE JOB NE REDIGE RIEN ET NE PUBLIE RIEN. Il se contente de rappeler au bot
//! les sessions en souffrance : seul le bot voit Discord, et seule l'API sait a
//! qui confier la plume. Le bot rappelle alors la meme sequence que a
//! l'ouverture — une seule implementation, donc aucune divergence possible.
//!
//! LE PLAFOND EST DANS LA REQUETE, pas ici. Une session qui a epuise ses
//! tentatives cesse d'apparaitre : le job n'a rien a filtrer, et le plafond
//! reste vrai meme si un autre appelant interroge la meme liste.

use platform_core::nexus::application::game::session_announcement::session_announcement_service::TENTATIVES_MAX;
use platform_core::nexus::ports::outbound::events::game_events;

use crate::nexus::bootstrap::AppState;

#[derive(Debug, Default)]
pub struct Rapport {
    /// Sessions rappelees au bot.
    pub relancees: usize,
    /// Sessions abandonnees, signalees dans le salon de logs.
    pub abandons: usize,
    pub errors: usize,
}

pub async fn run(
    state: &AppState,
) -> Result<Rapport, platform_core::nexus::domain::errors::DomainError> {
    let en_attente = state
        .game_server_repo
        .annonces_en_attente(TENTATIVES_MAX)
        .await?;

    let mut rapport = Rapport::default();
    for serveur in en_attente {
        // Le bot refera la sequence complete : demander le texte, publier,
        // marquer, puis poser le panneau. On ne lui transmet que l'identite de
        // la session — lui redemandera le reste, et travaillera donc sur un
        // etat frais plutot que sur celui d'il y a cinq minutes.
        state
            .events
            .publish(
                game_events::SESSION_ANNOUNCEMENT_RETRY,
                serde_json::json!({
                    "server_id": serveur.id.to_string(),
                    "guild_id": serveur.guild_id,
                }),
            )
            .await;
        rapport.relancees += 1;
    }

    // ── Abandons a signaler ──
    //
    // Une session qui a epuise ses tentatives ne sera plus reprise. Le taire
    // laisserait une soiree sans panneau d'inscription, et personne ne
    // l'apprendrait autrement que par les joueurs.
    //
    // Le marquage a lieu APRES la publication de l'evenement : marque avant,
    // une panne entre les deux ferait disparaitre l'alerte pour toujours. Dans
    // l'autre sens, le pire est une alerte publiee deux fois.
    let abandonnees = state
        .game_server_repo
        .annonces_abandonnees(TENTATIVES_MAX)
        .await?;
    for serveur in abandonnees {
        state
            .events
            .publish(
                game_events::SESSION_ANNOUNCEMENT_ABANDONED,
                serde_json::json!({
                    "server_id": serveur.id.to_string(),
                    "guild_id": serveur.guild_id,
                    "nom": serveur.name,
                    "tentatives": serveur.announcement_attempts,
                }),
            )
            .await;
        if let Err(erreur) = state
            .game_server_repo
            .marquer_abandon_signale(serveur.id)
            .await
        {
            tracing::warn!(%erreur, server_id = %serveur.id, "abandon signale mais non marque");
            rapport.errors += 1;
        }
        rapport.abandons += 1;
    }
    if rapport.relancees > 0 {
        tracing::info!(
            relancees = rapport.relancees,
            "annonces de session : reprise demandee"
        );
    }
    Ok(rapport)
}
