//! Consumer des evenements du delai d'acceptation du reglement.
//!
//! L'API decide (`sentinel::jobs::welcome::rules_deadline`) et publie ; le bot
//! execute. Seul lui parle a Discord : l'API ne connait ni les messages prives
//! ni les expulsions.
//!
//! A ne pas confondre avec `security::quarantine_expired_consumer`, qui traite
//! les comptes SUSPECTS. Ici la population est celle des arrivants ordinaires,
//! et le ton des messages n'est pas le meme : personne n'est soupconne de rien.

use serenity::all::{Context, CreateMessage, GuildId, UserId};
use std::str::FromStr;
use tracing::{info, warn};

pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "welcome-bot-rules-deadline".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move {
                    handle_event(&ctx, &payload_json).await;
                }
            },
        )
        .await;
    });
}

/// Identifiants d'un evenement, une fois valides.
fn cibles(data: &serde_json::Value) -> Option<(GuildId, UserId)> {
    let guild = data.get("guild_id").and_then(|v| v.as_str())?;
    let user = data.get("user_id").and_then(|v| v.as_str())?;
    Some((
        GuildId::new(u64::from_str(guild).ok()?),
        UserId::new(u64::from_str(user).ok()?),
    ))
}

/// Texte de la relance.
///
/// Il dit QUAND le delai expire, pas seulement qu'il expire : « bientot » ne
/// permet a personne de s'organiser. L'horodatage Discord se lit dans le fuseau
/// de chacun, ce qu'une heure ecrite en clair ne permet pas.
pub fn build_reminder_content(guild_name: &str, expires_at: Option<&str>) -> String {
    let quand = expires_at
        .and_then(|iso| chrono::DateTime::parse_from_rfc3339(iso).ok())
        .map(|t| {
            format!(
                " Il te reste jusqu'au <t:{}:F> (<t:{}:R>).",
                t.timestamp(),
                t.timestamp()
            )
        })
        .unwrap_or_default();
    format!(
        "Bonjour ! Tu n'as pas encore accepte le reglement de **{guild_name}**, \
         et tu n'as donc pas encore acces au serveur.{quand}\n\n\
         Il suffit d'un clic sur le bouton du salon de reglement. \
         Sans reponse, tu seras retire du serveur — tu pourras revenir avec une \
         nouvelle invitation."
    )
}

/// Texte envoye au moment de l'expulsion.
///
/// Envoye AVANT le kick : une fois la personne partie, le bot ne partage plus
/// de serveur avec elle et Discord refuse le message prive. Elle serait
/// expulsee sans jamais savoir pourquoi.
pub fn build_kick_notice_content(guild_name: &str) -> String {
    format!(
        "Tu as ete retire de **{guild_name}** faute d'avoir accepte le reglement \
         dans le delai imparti.\n\n\
         Ce n'est pas un bannissement : tu peux revenir a tout moment avec une \
         nouvelle invitation."
    )
}

async fn nom_guilde(ctx: &Context, guild_id: GuildId) -> String {
    guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| "le serveur".to_string())
}

/// Envoie un message prive, sans jamais faire echouer l'appelant.
///
/// Des MP fermes sont le cas ORDINAIRE, pas une anomalie : beaucoup de gens les
/// desactivent. Ne pas pouvoir prevenir ne doit ni annuler l'expulsion, ni
/// remplir le journal d'erreurs.
async fn message_prive(ctx: &Context, user_id: UserId, contenu: String) {
    match user_id.create_dm_channel(&ctx.http).await {
        Ok(canal) => {
            if let Err(e) = canal
                .id
                .send_message(&ctx.http, CreateMessage::new().content(contenu))
                .await
            {
                info!(error = %e, user = %user_id, "reglement : message prive non delivre");
            }
        }
        Err(e) => info!(error = %e, user = %user_id, "reglement : messages prives fermes"),
    }
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let Ok(event) = serde_json::from_str::<serde_json::Value>(payload_json) else {
        return;
    };
    let nom_event = event.get("event").and_then(|v| v.as_str()).unwrap_or("");
    let Some(data) = event.get("data") else {
        return;
    };
    let Some((guild_id, user_id)) = cibles(data) else {
        return;
    };

    match nom_event {
        "welcome_rules_reminder" => {
            let expires_at = data.get("expires_at").and_then(|v| v.as_str());
            let nom = nom_guilde(ctx, guild_id).await;
            message_prive(ctx, user_id, build_reminder_content(&nom, expires_at)).await;
            info!(guild = %guild_id, user = %user_id, "reglement : relance envoyee");
        }

        "welcome_rules_expired" => {
            // Le membre a-t-il quitte de lui-meme entre-temps ? Le cas est
            // frequent, et tenter de l'expulser remplirait le journal
            // d'erreurs pour rien.
            if guild_id.member(&ctx.http, user_id).await.is_err() {
                info!(guild = %guild_id, user = %user_id, "reglement : membre deja parti");
                return;
            }

            let nom = nom_guilde(ctx, guild_id).await;
            // Prevenir AVANT de retirer : apres le kick, le bot ne partage plus
            // de serveur avec la personne et Discord refuse le message.
            message_prive(ctx, user_id, build_kick_notice_content(&nom)).await;

            match guild_id
                .kick_with_reason(&ctx.http, user_id, "Reglement non accepte dans le delai")
                .await
            {
                Ok(()) => {
                    info!(guild = %guild_id, user = %user_id, "reglement : membre retire faute d'acceptation")
                }
                Err(e) => {
                    warn!(error = %e, guild = %guild_id, user = %user_id, "reglement : expulsion impossible")
                }
            }
        }

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_cibles_se_lisent_ou_l_evenement_est_ignore() {
        let bon = serde_json::json!({"guild_id": "123", "user_id": "456"});
        let (g, u) = cibles(&bon).unwrap();
        assert_eq!(g, GuildId::new(123));
        assert_eq!(u, UserId::new(456));

        // Un identifiant illisible ne doit pas faire expulser au hasard.
        assert!(cibles(&serde_json::json!({"guild_id": "abc", "user_id": "456"})).is_none());
        assert!(cibles(&serde_json::json!({"guild_id": "123"})).is_none());
        assert!(cibles(&serde_json::json!({})).is_none());
    }

    #[test]
    fn la_relance_dit_quand_le_delai_expire() {
        let avec = build_reminder_content("La Bande du Canape", Some("2026-08-25T12:00:00Z"));
        assert!(avec.contains("La Bande du Canape"));
        // Horodatage Discord : chacun le lit dans son fuseau.
        assert!(avec.contains("<t:"));
        assert!(avec.contains("reglement"));

        // Sans date exploitable, la relance part quand meme — sans promesse
        // d'horaire qu'on ne pourrait pas tenir.
        let sans = build_reminder_content("Serveur", None);
        assert!(!sans.contains("<t:"));
        assert!(sans.contains("Serveur"));
    }

    #[test]
    fn une_date_illisible_est_simplement_omise() {
        let contenu = build_reminder_content("Serveur", Some("pas une date"));
        assert!(!contenu.contains("<t:"));
    }

    #[test]
    fn l_avis_d_expulsion_dit_que_ce_n_est_pas_un_bannissement() {
        // Quelqu'un qui a simplement tarde a cliquer ne doit pas croire qu'il
        // est banni : il peut revenir.
        let contenu = build_kick_notice_content("La Bande du Canape");
        assert!(contenu.contains("La Bande du Canape"));
        assert!(contenu.contains("bannissement"));
        assert!(contenu.contains("revenir"));
    }
}
