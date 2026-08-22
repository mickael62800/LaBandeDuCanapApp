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
        .map(|role| role_json(role.id.get(), &role.name, role.colour.0, role.mentionable))
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

    let inventory = build_inventory(&roles_json, &live_panel_messages, &unreadable_channels);

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

/// Un role, tel que le domaine l'attend dans l'inventaire.
pub fn role_json(id: u64, nom: &str, couleur: u32, mentionnable: bool) -> serde_json::Value {
    // L'identifiant part en CHAINE : au-dela de 2^53, un entier JSON perd des
    // chiffres en passant par un flottant, et un role sur deux deviendrait
    // introuvable cote domaine.
    serde_json::json!({
        "id": id.to_string(),
        "name": nom,
        "color": couleur,
        "mentionable": mentionnable,
    })
}

/// La photographie complete deposee cote API.
///
/// Les trois listes sont distinctes a dessein. Un panneau absent des
/// `live_panel_messages` sans figurer dans `unreadable_channels` a REELLEMENT
/// disparu ; s'il est declare illisible, le domaine s'abstient. Fusionner les
/// deux ferait redeployer des panneaux parfaitement vivants.
pub fn build_inventory(
    roles: &[serde_json::Value],
    live_panel_messages: &[String],
    unreadable_channels: &[String],
) -> serde_json::Value {
    serde_json::json!({
        "roles": roles,
        "live_panel_messages": live_panel_messages,
        "unreadable_channels": unreadable_channels,
    })
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
    fn un_identifiant_de_role_part_toujours_en_chaine() {
        // Au-dela de 2^53, un entier JSON perd des chiffres en passant par un
        // flottant. Les identifiants Discord depassent ce seuil : en nombre,
        // un role sur deux deviendrait introuvable cote domaine.
        let enorme = 1_234_567_890_123_456_789u64;
        let json = role_json(enorme, "Joueurs", 0x3498db, true);
        assert_eq!(json["id"], enorme.to_string());
        assert!(json["id"].is_string());
        // Relu depuis la chaine, l'identifiant est intact.
        assert_eq!(json["id"].as_str().unwrap().parse::<u64>().unwrap(), enorme);
    }

    #[test]
    fn un_role_porte_son_nom_sa_couleur_et_sa_mentionnabilite() {
        let json = role_json(42, "Minecraft", 0x2ecc71, false);
        assert_eq!(json["name"], "Minecraft");
        assert_eq!(json["color"], 0x2ecc71);
        assert_eq!(json["mentionable"], false);

        // Un nom de role Discord accepte espaces et emojis.
        let json = role_json(1, "7 Days to Die \u{1f9df}", 0, true);
        assert_eq!(json["name"], "7 Days to Die \u{1f9df}");
        assert_eq!(json["mentionable"], true);
    }

    #[test]
    fn l_inventaire_garde_les_trois_listes_separees() {
        // Le coeur du module : un panneau ABSENT est un ecart a signaler, un
        // panneau ILLISIBLE est une cecite a avouer. Les fusionner ferait
        // redeployer des panneaux parfaitement vivants.
        let roles = vec![role_json(1, "A", 0, true)];
        let vivants = vec!["100".to_string(), "200".to_string()];
        let illisibles = vec!["300".to_string()];

        let inv = build_inventory(&roles, &vivants, &illisibles);
        assert_eq!(inv["roles"].as_array().unwrap().len(), 1);
        assert_eq!(
            inv["live_panel_messages"],
            serde_json::json!(["100", "200"])
        );
        assert_eq!(inv["unreadable_channels"], serde_json::json!(["300"]));
    }

    #[test]
    fn un_inventaire_vide_reste_un_inventaire() {
        // Une guilde sans role ni panneau doit produire trois listes VIDES, pas
        // des champs absents : cote domaine, « aucun role » et « je n'ai pas
        // regarde » ne se traitent pas pareil.
        let inv = build_inventory(&[], &[], &[]);
        assert_eq!(inv["roles"], serde_json::json!([]));
        assert_eq!(inv["live_panel_messages"], serde_json::json!([]));
        assert_eq!(inv["unreadable_channels"], serde_json::json!([]));
    }

    #[test]
    fn une_erreur_qui_n_est_pas_http_ne_conclut_a_aucun_ecart() {
        // Fail closed : sans preuve que le message a disparu, on ne le declare
        // pas disparu. Le declarer ferait redeployer un panneau vivant.
        assert!(!message_is_gone(&serenity::Error::Other("coupure reseau")));
        assert!(!message_is_gone(&serenity::Error::Other("")));
    }

    #[test]
    fn test_role_json_with_various_colors() {
        let j1 = role_json(100, "Red", 0xFF0000, true);
        assert_eq!(j1["color"], 0xFF0000);
        assert_eq!(j1["id"], "100");

        let j2 = role_json(200, "Blue", 0x0000FF, false);
        assert_eq!(j2["color"], 0x0000FF);
        assert_eq!(j2["mentionable"], false);
    }

    #[test]
    fn test_role_json_preserves_all_fields() {
        let j = role_json(42, "TestRole", 0x123456, true);
        assert_eq!(j["id"], "42");
        assert_eq!(j["name"], "TestRole");
        assert_eq!(j["color"], 0x123456);
        assert_eq!(j["mentionable"], true);
    }

    #[test]
    fn test_role_json_with_unicode_name() {
        let j = role_json(1, "🎮 Minecraft", 0x2ecc71, true);
        assert_eq!(j["name"], "🎮 Minecraft");
    }

    #[test]
    fn test_build_inventory_with_all_populated() {
        let roles = vec![role_json(1, "Role1", 0, true), role_json(2, "Role2", 0, false)];
        let live = vec!["msg1".to_string(), "msg2".to_string()];
        let unreadable = vec!["ch1".to_string()];

        let inv = build_inventory(&roles, &live, &unreadable);
        assert_eq!(inv["roles"].as_array().unwrap().len(), 2);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 2);
        assert_eq!(inv["unreadable_channels"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_build_inventory_only_roles() {
        let roles = vec![role_json(1, "Role1", 0, true)];
        let inv = build_inventory(&roles, &[], &[]);
        assert_eq!(inv["roles"].as_array().unwrap().len(), 1);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 0);
        assert_eq!(inv["unreadable_channels"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_build_inventory_only_live_panels() {
        let live = vec!["msg1".to_string()];
        let inv = build_inventory(&[], &live, &[]);
        assert_eq!(inv["roles"].as_array().unwrap().len(), 0);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 1);
        assert_eq!(inv["unreadable_channels"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_build_inventory_only_unreadable() {
        let unreadable = vec!["ch1".to_string(), "ch2".to_string()];
        let inv = build_inventory(&[], &[], &unreadable);
        assert_eq!(inv["roles"].as_array().unwrap().len(), 0);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 0);
        assert_eq!(inv["unreadable_channels"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_message_is_gone_returns_false_for_non_404() {
        // Not found (404) should return true, but other HTTP errors should return false
        assert!(!message_is_gone(&serenity::Error::Other("random error")));
    }

    #[test]
    fn test_large_role_id_preserved_as_string() {
        // Test that very large role IDs are preserved correctly as strings
        let huge_id = u64::MAX;
        let j = role_json(huge_id, "Huge", 0, false);
        assert_eq!(j["id"].as_str().unwrap(), huge_id.to_string());
    }

    #[test]
    fn test_role_json_with_empty_name() {
        let j = role_json(1, "", 0, true);
        assert_eq!(j["name"], "");
    }

    #[test]
    fn test_role_json_color_zero() {
        let j = role_json(1, "Test", 0, true);
        assert_eq!(j["color"], 0);
    }

    #[test]
    fn test_role_json_with_spaces_in_name() {
        let j = role_json(1, "Role   With   Spaces", 0x123456, true);
        assert_eq!(j["name"], "Role   With   Spaces");
    }

    #[test]
    fn test_inventory_structure_consistency() {
        // Ensure inventory JSON always has all three keys
        let inv1 = build_inventory(&[], &[], &[]);
        assert!(inv1.get("roles").is_some());
        assert!(inv1.get("live_panel_messages").is_some());
        assert!(inv1.get("unreadable_channels").is_some());

        let roles = vec![role_json(1, "R", 0, true)];
        let live = vec!["1".to_string()];
        let unreadable = vec!["2".to_string()];
        let inv2 = build_inventory(&roles, &live, &unreadable);
        assert!(inv2.get("roles").is_some());
        assert!(inv2.get("live_panel_messages").is_some());
        assert!(inv2.get("unreadable_channels").is_some());
    }
}
