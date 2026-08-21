//! Embeds Discord pour la Roue du Destin (repris de l'ancien module
//! `sentinel-bot/src/modules/wheel/embeds.rs`).

use serenity::all::CreateEmbed;
use serenity::all::CreateEmbedFooter;

use crate::api_client::WheelSpinResponse;

/// Couleur en fonction du type de resultat.
fn color_for(payout: i64, is_memorable: bool) -> u32 {
    if is_memorable && payout > 0 {
        return 0xf1c40f; // or
    }
    if is_memorable && payout < 0 {
        return 0x8b0000; // rouge sombre apocalypse
    }
    if payout > 0 {
        return 0x2ecc71; // vert
    }
    if payout < 0 {
        return 0xe74c3c; // rouge
    }
    0x95a5a6 // gris (blanche)
}

/// Embed d'attente, affiche pendant que la roue « tourne ».
///
/// Il est poste APRES que l'API a valide le tirage : annoncer un tirage qui
/// se fera refuser laisserait un message mensonger dans le salon.
pub fn build_spinning_embed(username: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("\u{1f300} La Roue du Destin tourne...")
        .description(format!(
            "\u{1f3b2} La roue tourne pour **{username}** !\n\n\
             # \u{1fa99} . . . \u{1fa99} . . . \u{1fa99}\n\n*Tic... tic... tic...*"
        ))
        .color(0xf1c40f)
        .footer(CreateEmbedFooter::new("Le destin se decide..."))
}

/// Embed final avec le resultat.
pub fn build_result_embed(resp: &WheelSpinResponse, username: &str) -> CreateEmbed {
    let net_str = if resp.payout > 0 {
        format!("+{}", resp.payout)
    } else {
        resp.payout.to_string()
    };
    let title = if resp.is_memorable {
        format!("\u{1f300} {} a tire... LE DESTIN PARLE !", username)
    } else {
        format!("\u{1f300} {} a tire la Roue", username)
    };

    let mut embed = CreateEmbed::new()
        .title(title)
        .description(format!("# {}", resp.case_label))
        .color(color_for(resp.payout, resp.is_memorable))
        .field("Gain", format!("{} coins", net_str), true)
        .field("Solde", format!("{} coins", resp.balance_after), true);

    if resp.is_memorable && resp.payout > 0 {
        embed = embed.footer(CreateEmbedFooter::new(format!(
            "🎉 GROS COUP pour {} ! Reviens demain pour ton prochain spin.",
            username
        )));
    } else if resp.is_memorable && resp.payout < 0 {
        embed = embed.footer(CreateEmbedFooter::new(format!(
            "💀 Le destin a frappe fort. Reviens demain {}.",
            username
        )));
    } else {
        embed = embed.footer(CreateEmbedFooter::new(format!(
            "Reviens demain pour ton prochain spin, {}",
            username
        )));
    }
    embed
}

/// Embed du solde (`/solde`).
pub fn build_wallet_embed(
    w: &crate::api_client::WalletResponse,
    display_name: &str,
) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("\u{1fa99} Portefeuille de {display_name}"))
        .color(0xf1c40f)
        .field("Solde", format!("**{}** coins", w.coins), true)
        .field("Total gagne", format!("{} coins", w.total_earned), true)
        .field("Total depense", format!("{} coins", w.total_spent), true)
        .timestamp(serenity::model::Timestamp::now())
}

/// Embed d'un don reussi (`/donner`), style historique du don de coins.
pub fn build_transfer_embed(
    from_user_id: u64,
    to_user_id: u64,
    amount: i64,
    from_balance: i64,
    reason: Option<&str>,
) -> CreateEmbed {
    let mut description = format!(
        "<@{from_user_id}> a donne **{amount} coins** a <@{to_user_id}> !\n\n\
         \u{1f4b0} Nouveau solde du donateur : {from_balance} coins"
    );
    if let Some(r) = reason {
        description.push_str(&format!("\n\u{1f4dd} Raison : {r}"));
    }
    CreateEmbed::new()
        .title("\u{1f381} Don de coins !")
        .description(description)
        .color(0x57F287)
        .timestamp(serenity::model::Timestamp::now())
}

/// Embed du classement (`/classement`), style medailles des anciens
/// leaderboards (`format_leaderboard` du module coussin).
pub fn build_leaderboard_embed(entries: &[crate::api_client::WalletResponse]) -> CreateEmbed {
    let body = if entries.is_empty() {
        "Aucun joueur pour le moment.".to_string()
    } else {
        let medals = ["\u{1f947}", "\u{1f948}", "\u{1f949}"];
        entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let rank = medals
                    .get(i)
                    .map(|m| (*m).to_string())
                    .unwrap_or_else(|| format!("{}.", i + 1));
                format!("{} <@{}> — **{}** coins", rank, e.user_id, e.coins)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    CreateEmbed::new()
        .title("\u{1f3c6} Classement — Les plus riches")
        .description(body)
        .color(0xE67E22)
        .timestamp(serenity::model::Timestamp::now())
}

/// Embed pour signaler une erreur (daily deja claim, API down...).
pub fn build_error_embed(message: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("\u{1f300} Roue du Destin")
        .description(message)
        .color(0xed4245)
}

pub fn build_coussin_challenge_embed(attacker_id: u64, defender_id: u64, mise: i64) -> CreateEmbed {
    CreateEmbed::new()
        .title("🛋️💣 Coussin Piégé !")
        .description(format!(
            "<@{attacker_id}> vient de glisser un coussin sous <@{defender_id}> — **{mise} coins** sur la table.\n\n\
             <@{defender_id}>, tu t'assois ou tu restes debout ?"
        ))
        .color(0xF39C12)
        .footer(CreateEmbedFooter::new("Rester debout devant tout le monde, ca se remarque."))
}

/// Nom lisible d'un objet du coffre a coussins.
///
/// Le catalogue est celui du domaine : dupliquer les libelles ici les aurait
/// laisses diverger de la boutique des la premiere retouche.
fn nom_objet(cle: &str) -> String {
    platform_core::nexus::domain::entities::coussin_shop::item(cle)
        .map(|i| i.name.to_string())
        .unwrap_or_else(|| cle.to_string())
}

/// Libelle de la classe, emoji compris. L'API renvoie la cle technique
/// (`ecraseur`) ; l'afficher telle quelle donnerait « Maniere : ecraseur ».
fn nom_classe(cle: &str) -> String {
    platform_core::nexus::domain::entities::coussin::PlayerClass::parse(cle)
        .map(|c| c.label().to_string())
        .unwrap_or_else(|| "🧍 Debout".to_string())
}

/// Le profil parle de CONFORT et de place sur le canape, pas de points de vie :
/// c'est le meme chiffre, mais il raconte enfin quelque chose.
pub fn build_coussin_profile_embed(p: &crate::api_client::CoussinProfileResponse) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("🛋️ Place de {} sur le canape", p.username))
        .color(0x5865F2)
        .description(format!(
            "**{}** · Niveau **{}** — {} XP\n🛋️ Confort {}/{} · 🪙 {} coins",
            p.title, p.level, p.xp, p.hp_current, p.hp_max, p.coins
        ))
        .field("Maniere de s'asseoir", nom_classe(&p.class), true)
        .field("Impact / Moelleux", format!("{} / {}", p.atk, p.def), true)
        .field("Points a placer", p.stat_points.to_string(), true)
        .field(
            "Palmares",
            format!(
                "{} assis · {} leves · {} match nuls",
                p.total_wins, p.total_losses, p.total_draws
            ),
            false,
        )
        .field(
            "Trouve sous les coussins",
            format!("{} coins", p.total_stolen),
            true,
        )
        .field("Fois reste debout", p.cowardice_count.to_string(), true)
        .field("Bazar declenche", p.chaos_events.to_string(), true)
}

/// Achat au coffre a coussins. Reponse privee : c'est une transaction
/// personnelle, et surtout annoncer publiquement qu'on vient d'acheter une
/// Punaise dans le Coussin ruinerait l'objet avant meme de s'en servir.
pub fn build_coussin_purchase_embed(item_key: &str, balance: i64) -> CreateEmbed {
    CreateEmbed::new()
        .title("🛋️ Planque effectuee")
        .description(format!(
            "**{}** est maintenant sous ton coussin.",
            nom_objet(item_key)
        ))
        .field("Solde", format!("{balance} coins"), true)
        .color(0xF39C12)
}

/// Garantie anti-tache. Le caractere douteux du contrat fait partie du jeu :
/// on le signale sans dire ce qu'il changera, sinon il n'y aurait plus de
/// mauvaise surprise a avoir.
pub fn build_coussin_insurance_embed(is_scam: bool, expires_at: &str) -> CreateEmbed {
    if is_scam {
        CreateEmbed::new()
            .title("⚠️ Garantie signee")
            .description(format!(
                "Le vendeur a souri un peu trop vite et les petites lignes sont illisibles.\n\
                 Couverture annoncee jusqu'a **{expires_at}**."
            ))
            .color(0xE67E22)
    } else {
        CreateEmbed::new()
            .title("🧼 Garantie anti-tache active")
            .description(format!(
                "Tes pertes sont couvertes jusqu'a **{expires_at}**."
            ))
            .color(0x2ECC71)
    }
}

/// Ce qu'on planque sous son coussin. Prive par nature : la moitie de ces
/// objets ne valent que par l'effet de surprise.
pub fn build_coussin_inventory_embed(
    items: &[crate::api_client::CoussinInventoryItem],
) -> CreateEmbed {
    let body = if items.is_empty() {
        "Rien sous ton coussin. Le coffre s'ouvre avec `/shop`.".to_string()
    } else {
        items
            .iter()
            .map(|i| format!("• **{}** ×{}", nom_objet(&i.item_key), i.quantity))
            .collect::<Vec<_>>()
            .join("\n")
    };
    CreateEmbed::new()
        .title("🛋️ Sous ton coussin")
        .description(body)
        .color(0x5865F2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_for_positive_memorable() {
        assert_eq!(color_for(100, true), 0xf1c40f); // or
    }

    #[test]
    fn test_color_for_negative_memorable() {
        assert_eq!(color_for(-100, true), 0x8b0000); // rouge sombre
    }

    #[test]
    fn test_color_for_positive_normal() {
        assert_eq!(color_for(50, false), 0x2ecc71); // vert
    }

    #[test]
    fn test_color_for_negative_normal() {
        assert_eq!(color_for(-50, false), 0xe74c3c); // rouge
    }

    #[test]
    fn test_color_for_zero() {
        assert_eq!(color_for(0, false), 0x95a5a6); // gris
        assert_eq!(color_for(0, true), 0x95a5a6); // gris
    }

    #[test]
    fn test_build_spinning_embed() {
        let embed = build_spinning_embed("Alice");
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Roue"));
        assert!(json["description"].as_str().unwrap().contains("Alice"));
        assert_eq!(json["color"], 0xf1c40f);
    }

    #[test]
    fn test_build_result_embed_positive() {
        let resp = WheelSpinResponse {
            case_label: "Jackpot".into(),
            payout: 100,
            balance_after: 500,
            is_memorable: false,
        };
        let embed = build_result_embed(&resp, "Bob");
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Bob"));
        assert!(json["description"].as_str().unwrap().contains("Jackpot"));
        assert_eq!(json["color"], 0x2ecc71);
    }

    #[test]
    fn test_build_result_embed_negative_memorable() {
        let resp = WheelSpinResponse {
            case_label: "Perte".into(),
            payout: -50,
            balance_after: 100,
            is_memorable: true,
        };
        let embed = build_result_embed(&resp, "Charlie");
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["title"].as_str().unwrap().contains("DESTIN PARLE"));
        assert_eq!(json["color"], 0x8b0000);
    }

    #[test]
    fn test_build_wallet_embed() {
        let wallet = crate::api_client::WalletResponse {
            user_id: "u1".into(),
            coins: 500,
            total_earned: 1000,
            total_spent: 500,
        };
        let embed = build_wallet_embed(&wallet, "Alice");
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Alice"));
        assert_eq!(json["color"], 0xf1c40f);
    }

    #[test]
    fn test_build_transfer_embed_with_reason() {
        let embed = build_transfer_embed(100, 200, 50, 450, Some("Gift"));
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["description"].as_str().unwrap().contains("Gift"));
        assert_eq!(json["color"], 0x57F287);
    }

    #[test]
    fn test_build_transfer_embed_without_reason() {
        let embed = build_transfer_embed(100, 200, 50, 450, None);
        let json = serde_json::to_value(&embed).unwrap();
        let desc = json["description"].as_str().unwrap();
        assert!(!desc.contains("Raison :"));
    }

    #[test]
    fn test_build_leaderboard_embed_empty() {
        let embed = build_leaderboard_embed(&[]);
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["description"].as_str().unwrap().contains("Aucun joueur"));
    }

    #[test]
    fn test_build_leaderboard_embed_with_entries() {
        let entries = vec![
            crate::api_client::WalletResponse {
                user_id: "u1".into(),
                coins: 1000,
                total_earned: 2000,
                total_spent: 1000,
            }
        ];
        let embed = build_leaderboard_embed(&entries);
        let json = serde_json::to_value(&embed).unwrap();
        assert_eq!(json["color"], 0xE67E22);
    }

    #[test]
    fn test_build_error_embed() {
        let embed = build_error_embed("API down");
        let json = serde_json::to_value(&embed).unwrap();
        assert_eq!(json["description"], "API down");
        assert_eq!(json["color"], 0xed4245);
    }

    #[test]
    fn test_build_coussin_profile_embed() {
        let profile = crate::api_client::CoussinProfileResponse {
            username: "Alice".into(),
            class: "ecraseur".into(),
            level: 5,
            xp: 100,
            atk: 10,
            def: 8,
            hp_current: 50,
            hp_max: 100,
            coins: 200,
            stat_points: 2,
            title: "Squatteur".into(),
            total_wins: 10,
            total_losses: 3,
            total_draws: 1,
            total_stolen: 50,
            cowardice_count: 0,
            chaos_events: 2,
        };
        let embed = build_coussin_profile_embed(&profile);
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Alice"));
        assert_eq!(json["color"], 0x5865F2);
    }

    #[test]
    fn test_build_coussin_purchase_embed() {
        let embed = build_coussin_purchase_embed("plume", 450);
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Planque"));
        assert_eq!(json["color"], 0xF39C12);
    }

    #[test]
    fn test_build_coussin_insurance_embed_legitimate() {
        let embed = build_coussin_insurance_embed(false, "demain");
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["title"].as_str().unwrap().contains("anti-tache"));
        assert_eq!(json["color"], 0x2ECC71);
    }

    #[test]
    fn test_build_coussin_insurance_embed_scam() {
        let embed = build_coussin_insurance_embed(true, "demain");
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["title"].as_str().unwrap().contains("Garantie signee"));
        assert_eq!(json["color"], 0xE67E22);
    }

    #[test]
    fn test_build_coussin_inventory_embed_empty() {
        let embed = build_coussin_inventory_embed(&[]);
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["description"].as_str().unwrap().contains("Rien"));
    }

    #[test]
    fn test_build_coussin_inventory_embed_with_items() {
        let items = vec![
            crate::api_client::CoussinInventoryItem {
                item_key: "plume".into(),
                quantity: 2,
            }
        ];
        let embed = build_coussin_inventory_embed(&items);
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["title"].as_str().unwrap().contains("coussin"));
    }
}
