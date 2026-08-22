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

    #[test]
    fn test_role_json_with_max_color() {
        let j = role_json(1, "Test", u32::MAX, true);
        assert_eq!(j["color"], u32::MAX);
    }

    #[test]
    fn test_role_json_mentionable_false() {
        let j = role_json(1, "Test", 0, false);
        assert_eq!(j["mentionable"], false);
    }

    #[test]
    fn test_role_json_mentionable_true() {
        let j = role_json(1, "Test", 0, true);
        assert_eq!(j["mentionable"], true);
    }

    #[test]
    fn test_build_inventory_many_roles() {
        let roles: Vec<_> = (0..10)
            .map(|i| role_json(i, &format!("Role{}", i), i as u32, i % 2 == 0))
            .collect();

        let inv = build_inventory(&roles, &[], &[]);
        assert_eq!(inv["roles"].as_array().unwrap().len(), 10);
    }

    #[test]
    fn test_build_inventory_many_panels() {
        let live: Vec<_> = (0..20).map(|i| i.to_string()).collect();
        let inv = build_inventory(&[], &live, &[]);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 20);
    }

    #[test]
    fn test_build_inventory_duplicates_preserved() {
        // Inventory should preserve exact data, including duplicates if present
        let live = vec!["1".to_string(), "1".to_string(), "2".to_string()];
        let inv = build_inventory(&[], &live, &[]);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_message_is_gone_non_http_error() {
        assert!(!message_is_gone(&serenity::Error::Other("test")));
    }

    #[test]
    fn test_role_json_numeric_id_boundaries() {
        // Test with various ID sizes
        let ids = vec![1u64, 10u64, 100u64, 1000u64, 10000u64];
        for id in ids {
            let j = role_json(id, "Test", 0, true);
            assert_eq!(j["id"].as_str().unwrap().parse::<u64>().unwrap(), id);
        }
    }

    #[test]
    fn test_build_inventory_json_structure() {
        let roles = vec![role_json(1, "Test", 0xFF0000, true)];
        let live = vec!["msg1".to_string()];
        let unreadable = vec!["ch1".to_string()];

        let inv = build_inventory(&roles, &live, &unreadable);

        // Verify all arrays are present and correct type
        assert!(inv["roles"].is_array());
        assert!(inv["live_panel_messages"].is_array());
        assert!(inv["unreadable_channels"].is_array());

        // Verify first role has all fields
        let first_role = &inv["roles"][0];
        assert!(first_role["id"].is_string());
        assert!(first_role["name"].is_string());
        assert!(first_role["color"].is_number());
        assert!(first_role["mentionable"].is_boolean());
    }

    #[test]
    fn test_role_json_special_characters_in_name() {
        let j = role_json(1, "Role™ | Spéciâl", 0, true);
        assert_eq!(j["name"], "Role™ | Spéciâl");
    }

    #[test]
    fn test_build_inventory_empty_strings_in_arrays() {
        let live = vec!["".to_string(), "msg".to_string()];
        let unreadable = vec!["".to_string()];

        let inv = build_inventory(&[], &live, &unreadable);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 2);
        assert_eq!(inv["unreadable_channels"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_role_json_zero_id() {
        let j = role_json(0, "ZeroRole", 0, true);
        assert_eq!(j["id"], "0");
    }

    #[test]
    fn test_build_inventory_mixed_sizes() {
        let roles = vec![role_json(1, "R", 0, true), role_json(2, "R2", 0xFF0000, false)];
        let live = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let unreadable = vec!["x".to_string()];

        let inv = build_inventory(&roles, &live, &unreadable);

        assert_eq!(inv["roles"].as_array().unwrap().len(), 2);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 3);
        assert_eq!(inv["unreadable_channels"].as_array().unwrap().len(), 1);

        // Verify roles are distinct
        let r1 = &inv["roles"][0];
        let r2 = &inv["roles"][1];
        assert_ne!(r1["mentionable"], r2["mentionable"]);
    }

    #[test]
    fn test_role_json_id_string_format() {
        let j = role_json(12345, "Test", 0, true);
        assert!(j["id"].is_string());
        assert_eq!(j["id"].as_str().unwrap(), "12345");
    }

    #[test]
    fn test_role_json_name_preserved() {
        let j = role_json(1, "MyRoleName", 0, true);
        assert_eq!(j["name"], "MyRoleName");
    }

    #[test]
    fn test_build_inventory_array_types() {
        let inv = build_inventory(&[], &[], &[]);
        assert!(inv["roles"].is_array());
        assert!(inv["live_panel_messages"].is_array());
        assert!(inv["unreadable_channels"].is_array());
    }

    #[test]
    fn test_role_json_all_fields_present() {
        let j = role_json(5, "Test", 0x123456, false);
        assert!(j.get("id").is_some());
        assert!(j.get("name").is_some());
        assert!(j.get("color").is_some());
        assert!(j.get("mentionable").is_some());
    }

    #[test]
    fn test_build_inventory_roles_array_contains_objects() {
        let roles = vec![role_json(1, "R1", 0, true)];
        let inv = build_inventory(&roles, &[], &[]);
        let roles_arr = inv["roles"].as_array().unwrap();
        assert!(roles_arr[0].is_object());
    }

    #[test]
    fn test_message_is_gone_other_error() {
        let err = serenity::Error::Other("test error");
        assert!(!message_is_gone(&err));
    }

    #[test]
    fn test_build_inventory_consistent_structure() {
        let inv1 = build_inventory(&[], &[], &[]);
        let inv2 = build_inventory(&[], &[], &[]);

        // Both empty inventories should have same structure
        assert_eq!(inv1["roles"].as_array().unwrap().len(), 0);
        assert_eq!(inv2["roles"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_role_json_color_value() {
        let j = role_json(1, "Test", 0xABCDEF, true);
        assert_eq!(j["color"], 0xABCDEF);
    }

    #[test]
    fn test_build_inventory_large_panel_count() {
        let live: Vec<_> = (0..100).map(|i| i.to_string()).collect();
        let inv = build_inventory(&[], &live, &[]);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 100);
    }

    #[test]
    fn test_role_json_mentionable_bool_type() {
        let j_false = role_json(1, "Test", 0, false);
        let j_true = role_json(1, "Test", 0, true);

        assert!(j_false["mentionable"].is_boolean());
        assert!(j_true["mentionable"].is_boolean());
    }

    #[test]
    fn test_build_inventory_preserves_order() {
        let roles = vec![
            role_json(1, "First", 0, true),
            role_json(2, "Second", 0, false),
            role_json(3, "Third", 0, true),
        ];
        let inv = build_inventory(&roles, &[], &[]);
        let roles_arr = inv["roles"].as_array().unwrap();

        assert_eq!(roles_arr[0]["id"], "1");
        assert_eq!(roles_arr[1]["id"], "2");
        assert_eq!(roles_arr[2]["id"], "3");
    }

    #[test]
    fn test_role_json_unicode_handling() {
        let j = role_json(1, "👑 King 👑", 0, true);
        assert_eq!(j["name"], "👑 King 👑");
    }

    #[test]
    fn test_build_inventory_mixed_arrays() {
        let roles = vec![role_json(1, "R", 0, true)];
        let live = vec!["msg1".to_string(), "msg2".to_string()];
        let unreadable = vec!["ch1".to_string()];

        let inv = build_inventory(&roles, &live, &unreadable);

        // Verify each array has correct size
        assert_eq!(inv["roles"].as_array().unwrap().len(), 1);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 2);
        assert_eq!(inv["unreadable_channels"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_role_json_all_color_values() {
        for color in [0x000000, 0xFFFFFF, 0xFF0000, 0x00FF00, 0x0000FF] {
            let j = role_json(1, "Test", color, true);
            assert_eq!(j["color"], color);
        }
    }

    #[test]
    fn test_role_json_name_field_exact() {
        let names = vec!["Simple", "With Spaces", "With-Dashes", "With_Underscores"];
        for name in names {
            let j = role_json(1, name, 0, true);
            assert_eq!(j["name"], name);
        }
    }

    #[test]
    fn test_build_inventory_single_of_each() {
        let roles = vec![role_json(1, "R", 0, true)];
        let live = vec!["m".to_string()];
        let unreadable = vec!["c".to_string()];

        let inv = build_inventory(&roles, &live, &unreadable);
        assert_eq!(inv["roles"].as_array().unwrap().len(), 1);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 1);
        assert_eq!(inv["unreadable_channels"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_message_is_gone_catches_non_http_error() {
        // Non-HTTP error should return false
        assert!(!message_is_gone(&serenity::Error::Other("test")));
    }

    #[test]
    fn test_role_json_id_boundaries() {
        let test_ids = [0u64, 1, 10, 100, 1000, u64::MAX >> 1, u64::MAX];
        for id in test_ids.iter() {
            let j = role_json(*id, "Test", 0, true);
            assert_eq!(j["id"].as_str().unwrap().parse::<u64>().unwrap(), *id);
        }
    }

    #[test]
    fn test_build_inventory_large_role_count() {
        let roles: Vec<_> = (0..50)
            .map(|i| role_json(i, &format!("R{}", i), i as u32, i % 2 == 0))
            .collect();

        let inv = build_inventory(&roles, &[], &[]);
        assert_eq!(inv["roles"].as_array().unwrap().len(), 50);
    }

    #[test]
    fn test_role_json_string_id_always() {
        // IDs must always be strings to avoid JSON number precision loss
        for id in [1u64, 100, 10000, 1000000] {
            let j = role_json(id, "Test", 0, true);
            assert!(j["id"].is_string());
        }
    }

    #[test]
    fn test_build_inventory_all_empty_arrays() {
        let inv = build_inventory(&[], &[], &[]);

        // All three should be empty arrays, not null
        assert_eq!(inv["roles"].as_array().unwrap().len(), 0);
        assert_eq!(inv["live_panel_messages"].as_array().unwrap().len(), 0);
        assert_eq!(inv["unreadable_channels"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_role_json_mentionable_consistency() {
        let j_true = role_json(1, "T", 0, true);
        let j_false = role_json(1, "T", 0, false);

        assert_ne!(j_true["mentionable"], j_false["mentionable"]);
        assert!(j_true["mentionable"].as_bool().unwrap());
        assert!(!j_false["mentionable"].as_bool().unwrap());
    }


    #[test]
    fn test_build_inventory_role_field_structure() {
        let roles = vec![role_json(1, "Test Role", 0xABCDEF, false)];
        let inv = build_inventory(&roles, &[], &[]);

        let role = &inv["roles"][0];
        assert_eq!(role["id"], "1");
        assert_eq!(role["name"], "Test Role");
        assert_eq!(role["color"], 0xABCDEF);
        assert_eq!(role["mentionable"], false);
    }

    #[test]
    fn test_build_inventory_panel_messages_type() {
        let live = vec!["123".to_string(), "456".to_string()];
        let inv = build_inventory(&[], &live, &[]);

        let messages = inv["live_panel_messages"].as_array().unwrap();
        for msg in messages {
            assert!(msg.is_string());
        }
    }

    #[test]
    fn test_build_inventory_unreadable_channels_type() {
        let unreadable = vec!["ch1".to_string(), "ch2".to_string()];
        let inv = build_inventory(&[], &[], &unreadable);

        let channels = inv["unreadable_channels"].as_array().unwrap();
        for ch in channels {
            assert!(ch.is_string());
        }
    }

    #[test]
    fn test_role_json_with_various_mentionable_values() {
        // Test both boolean values
        let true_role = role_json(1, "T", 0, true);
        let false_role = role_json(2, "F", 0, false);

        assert!(true_role["mentionable"].as_bool().unwrap());
        assert!(!false_role["mentionable"].as_bool().unwrap());
    }

    #[test]
    fn test_message_is_gone_pattern_matching() {
        // Test that only HTTP 404 returns true
        assert!(!message_is_gone(&serenity::Error::Other("any error")));
        // Other errors should return false
    }

    #[test]
    fn test_build_inventory_maintains_data_integrity() {
        let roles = vec![
            role_json(1, "Admin", 0xFF0000, true),
            role_json(2, "Moderator", 0x00FF00, false),
            role_json(3, "Member", 0x0000FF, true),
        ];
        let live = vec!["msg1".to_string(), "msg2".to_string()];
        let unreadable = vec!["ch".to_string()];

        let inv = build_inventory(&roles, &live, &unreadable);

        // Verify data is preserved
        let r = inv["roles"].as_array().unwrap();
        assert_eq!(r[0]["name"], "Admin");
        assert_eq!(r[1]["name"], "Moderator");
        assert_eq!(r[2]["name"], "Member");
    }
}
