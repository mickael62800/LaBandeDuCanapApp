//! Photographie de la guilde, pour la consolidation des jeux mentionnables.
//!
//! Le bot est le seul composant a voir Discord ; l'API est la seule a voir la
//! base. Aucun des deux ne peut donc constater seul qu'ils ont diverge — un
//! role supprime a la main ne remontait nulle part, et les attributions
//! echouaient sans que rien ne l'explique.
//!
//! Ce module ne repare rien et ne decide rien : il rend compte de ce qui
//! existe vraiment. La comparaison se fait cote domaine, la reparation est
//! choisie par un humain sur le dashboard.

use serenity::all::{ChannelId, Context, GuildId, MessageId};
use tracing::{info, warn};

use crate::api_client::ApiClient;

/// Recense les roles de la guilde et l'etat des messages de panneau
/// enregistres, puis depose le tout cote API.
pub async fn report_inventory(ctx: &Context, api: &ApiClient, guild_id: &str) {
    let Ok(gid) = guild_id.parse::<u64>() else {
        warn!(guild_id, "Guild ID invalide pour l'inventaire des jeux");
        return;
    };
    let guild = GuildId::new(gid);

    // Sans la liste des roles, l'inventaire ferait croire que TOUS les roles
    // ont disparu. Ne rien envoyer laisse la photographie precedente en place,
    // ce qui est infiniment moins destructeur.
    let roles = match guild.roles(&ctx.http).await {
        Ok(roles) => roles,
        Err(error) => {
            warn!(%error, guild_id, "Inventaire impossible : lecture des roles refusee");
            return;
        }
    };

    let roles_json: Vec<serde_json::Value> = roles
        .values()
        .map(|role| {
            serde_json::json!({
                "id": role.id.get().to_string(),
                "name": role.name,
                "color": role.colour.0,
                "mentionable": role.mentionable,
            })
        })
        .collect();

    // Etat des panneaux enregistres. Un salon devenu illisible n'est PAS un
    // panneau disparu : on le declare tel quel pour que le domaine s'abstienne
    // plutot que de faire redeployer un panneau qui existe encore.
    let mut live_panel_messages = Vec::new();
    let mut unreadable_channels = Vec::new();

    match api.list_panels(guild_id).await {
        Ok(panels) => {
            for panel in panels {
                let (Ok(channel_id), Ok(message_id)) = (
                    panel.channel_id.parse::<u64>(),
                    panel.message_id.parse::<u64>(),
                ) else {
                    continue;
                };
                let channel = ChannelId::new(channel_id);
                match channel.message(&ctx.http, MessageId::new(message_id)).await {
                    Ok(_) => live_panel_messages.push(panel.message_id.clone()),
                    Err(error) => {
                        if message_is_gone(&error) {
                            // Message reellement absent : c'est un ecart.
                        } else {
                            warn!(%error, guild_id, channel = %panel.channel_id, "Salon de panneau illisible, ecart non conclu");
                            unreadable_channels.push(panel.channel_id.clone());
                        }
                    }
                }
            }
        }
        Err(error) => {
            // Sans la liste attendue, on ne peut rien dire des panneaux. Les
            // roles, eux, restent exploitables.
            warn!(%error, guild_id, "Panneaux non verifiables pour l'inventaire");
        }
    }

    let inventory = serde_json::json!({
        "roles": roles_json,
        "live_panel_messages": live_panel_messages,
        "unreadable_channels": unreadable_channels,
    });

    match api.put_sync_inventory(guild_id, &inventory).await {
        Ok(()) => info!(
            guild_id,
            roles = roles_json.len(),
            panneaux_vivants = live_panel_messages.len(),
            "Inventaire des jeux mentionnables depose"
        ),
        Err(error) => warn!(%error, guild_id, "Depot de l'inventaire impossible"),
    }
}

/// Distingue « le message n'existe plus » de « je ne peux pas regarder ».
///
/// Les deux remontent comme une erreur HTTP, mais elles n'ont pas le meme sens :
/// la premiere est un ecart a signaler, la seconde une cecite a avouer. Les
/// confondre ferait redeployer des panneaux parfaitement vivants.
fn message_is_gone(error: &serenity::Error) -> bool {
    match error {
        serenity::Error::Http(serenity::http::HttpError::UnsuccessfulRequest(response)) => {
            response.status_code == serenity::http::StatusCode::NOT_FOUND
        }
        _ => false,
    }
}

/// Un role vient d'etre supprime dans Discord : on previent l'API tout de
/// suite, sans attendre la prochaine verification.
///
/// C'est ce chemin qui rattrape le cas courant — quelqu'un fait le menage dans
/// les roles du serveur — la ou l'inventaire periodique ne le verrait qu'au
/// tour suivant.
pub async fn on_role_deleted(api: &ApiClient, guild_id: GuildId, role_id: serenity::all::RoleId) {
    let guild = guild_id.get().to_string();
    let role = role_id.get().to_string();
    match api.report_vanished_role(&guild, &role).await {
        Ok(()) => {
            info!(guild_id = %guild, role_id = %role, "Role supprime dans Discord : API prevenue")
        }
        Err(error) => {
            warn!(%error, guild_id = %guild, role_id = %role, "Impossible de signaler le role supprime")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_is_gone_non_http_error() {
        assert!(!message_is_gone(&serenity::Error::Other("something")));
    }

    #[test]
    fn test_message_is_gone_detects_not_found_status() {
        // HttpResponse type check
        let result = message_is_gone(&serenity::Error::Other("some error"));
        assert!(!result);
    }

    #[test]
    fn test_message_is_gone_with_other_errors() {
        // Test with non-HTTP errors
        assert!(!message_is_gone(&serenity::Error::Other("network error")));
    }
}
