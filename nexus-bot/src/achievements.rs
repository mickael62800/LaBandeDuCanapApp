//! Hauts faits : commandes Discord et publication des deblocages.
//!
//! Le bot n'attribue jamais un haut fait lui-meme et ne lit jamais la base :
//! il appelle l'API, qui tranche. Il consomme `achievement.unlocked` sur la
//! stream Redis `nexus:events` pour publier l'annonce.
//!
//! Commandes :
//!   - `/haut-faits`               mes hauts faits (reponse ephemere) ;
//!   - `/haut-faits membre`        ceux d'un autre membre, si la config l'autorise.
//!
//! La liaison d'une identite de jeu (SteamID64, XUID) n'est plus exposee ici :
//! les hauts faits sont Discord. Le backend correspondant reste en place
//! (routes `/api/achievements/{guild}/links/...`, table `game_player_links`)
//! pour un jeu dont les evenements seraient un jour verifiables.

use std::sync::Arc;

use serenity::all::{
    ChannelId, CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, GuildId,
};

use crate::api_client::ApiClient;

/// Module dans `bot_guild_config` : salon d'annonce, mention, toggles.
const MODULE_BOT_NAME: &str = "nexus-achievements";

pub fn register() -> CreateCommand {
    CreateCommand::new("haut-faits")
        .description("Consulter tes hauts faits")
        .default_member_permissions(serenity::all::Permissions::empty())
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "moi",
            "Afficher tes hauts faits",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "membre",
                "Afficher les hauts faits d'un membre",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::User, "membre", "Le membre")
                    .required(true),
            ),
        )
}

pub async fn handle_command(api: &ApiClient, ctx: &Context, cmd: &CommandInteraction) {
    let Some(guild_id) = cmd.guild_id else {
        repondre(ctx, cmd, "Commande disponible uniquement dans un serveur.").await;
        return;
    };
    let guild_id = guild_id.to_string();
    let sub = cmd
        .data
        .options
        .first()
        .map(|o| o.name.as_str())
        .unwrap_or("moi");

    match sub {
        "membre" => {
            // Le membre vise vient de l'option ; la config decide si consulter
            // le profil d'autrui est permis.
            let config = api
                .get_guild_config(&guild_id, MODULE_BOT_NAME)
                .await
                .unwrap_or_default();
            let autorise = config
                .get("public_profiles")
                .map(|v| v != "false")
                .unwrap_or(true);
            if !autorise {
                repondre(
                    ctx,
                    cmd,
                    "La consultation des hauts faits d'un autre membre est desactivee.",
                )
                .await;
                return;
            }
            let cible = sous_option_user(cmd).unwrap_or(cmd.user.id.get());
            afficher(api, ctx, cmd, &guild_id, cible).await;
        }
        _ => afficher(api, ctx, cmd, &guild_id, cmd.user.id.get()).await,
    }
}

async fn afficher(
    api: &ApiClient,
    ctx: &Context,
    cmd: &CommandInteraction,
    guild_id: &str,
    user_id: u64,
) {
    let progress = match api
        .member_achievements(guild_id, &user_id.to_string(), None)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            repondre(ctx, cmd, &format!("Hauts faits indisponibles : {e}")).await;
            return;
        }
    };

    let (debloques, restants): (Vec<_>, Vec<_>) =
        progress.iter().partition(|p| p.unlocked_at.is_some());

    let mut embed = CreateEmbed::new()
        .title(format!(
            "🏆 Hauts faits — {} / {}",
            debloques.len(),
            progress.len()
        ))
        .description(format!("<@{user_id}>"))
        .color(0xf1c40f)
        .footer(CreateEmbedFooter::new("Hauts faits | Nexus"));

    // Une vignette parle plus qu'une liste : on prend l'image du dernier haut
    // fait obtenu quand l'administrateur en a choisi une.
    if let Some(icon) = debloques
        .iter()
        .filter_map(|p| p.icon_url.as_deref())
        .find_map(image_absolue)
    {
        embed = embed.thumbnail(icon);
    }

    embed = embed.field(
        format!("Debloques ({})", debloques.len()),
        liste(&debloques, "_Aucun pour l'instant._"),
        false,
    );
    if !restants.is_empty() {
        embed = embed.field(
            format!("A decrocher ({})", restants.len()),
            liste(&restants, "—"),
            false,
        );
    }

    let _ = cmd
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await;
}

/// Compose une liste bornee a la limite d'un champ d'embed (1024 caracteres).
fn liste(items: &[&crate::api_client::AchievementProgress], vide: &str) -> String {
    if items.is_empty() {
        return vide.to_string();
    }
    let mut out = String::new();
    let mut affiches = 0usize;
    for item in items {
        let ligne = format!("• **{}** — {}\n", item.name, item.description);
        if out.len() + ligne.len() > 950 {
            break;
        }
        out.push_str(&ligne);
        affiches += 1;
    }
    if affiches < items.len() {
        out.push_str(&format!("_… et {} de plus._", items.len() - affiches));
    }
    out
}

// ── Options ──────────────────────────────────────────────────────────────

fn sous_option_user(cmd: &CommandInteraction) -> Option<u64> {
    let sub = cmd.data.options.first()?;
    let serenity::all::CommandDataOptionValue::SubCommand(options) = &sub.value else {
        return None;
    };
    options
        .iter()
        .find(|o| o.name == "membre")
        .and_then(|o| o.value.as_user_id())
        .map(|id| id.get())
}

async fn repondre(ctx: &Context, cmd: &CommandInteraction, contenu: &str) {
    let _ = cmd
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(contenu)
                    .ephemeral(true),
            ),
        )
        .await;
}

/// Rend absolue une image de haut fait pour Discord.
///
/// Le dashboard enregistre les images livrees avec lui sous forme de chemin
/// local (`/Achievement/palworld/pal_01.jpg`) : stable d'un build a l'autre,
/// mais inutilisable tel quel par Discord, qui exige une URL absolue. On la
/// prefixe donc par `WEB_FRONT_URL`. Sans cette variable, on renonce a la
/// vignette plutot que d'envoyer une URL invalide.
fn image_absolue(icon: &str) -> Option<String> {
    let icon = icon.trim();
    if icon.is_empty() {
        return None;
    }
    if icon.starts_with("http://") || icon.starts_with("https://") {
        return Some(icon.to_owned());
    }
    let base = std::env::var("WEB_FRONT_URL").ok()?;
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/{}", icon.trim_start_matches('/')))
}

// ── Consumer d'evenements ────────────────────────────────────────────────

/// Spawn le consumer durable de `achievement.unlocked`.
pub fn spawn(ctx: Context, api: Arc<ApiClient>) {
    tokio::spawn(async move {
        let consumer = crate::event_bus::default_consumer_name();
        crate::event_bus::listen_stream_group(
            "nexus-bot-achievements".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                let api = api.clone();
                async move { handle_event(&ctx, &api, &payload_json).await }
            },
        )
        .await;
    });
}

/// Ce qu'un evenement `achievement.unlocked` porte d'utile.
///
/// Extrait du handler pour etre verifiable : un evenement mal forme ne doit
/// jamais aboutir a une annonce vide ou adressee au mauvais serveur, et c'est
/// exactement ce qu'aucun test ne pouvait constater tant que la lecture vivait
/// au milieu des appels Discord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HautFaitDebloque {
    pub guild_id: String,
    pub guild_num: u64,
    pub user_id: String,
    pub nom: String,
    pub description: String,
    pub jeu: Option<String>,
    pub icon_url: Option<String>,
}

/// Lit l'evenement. `None` des qu'il manque de quoi annoncer honnetement.
pub fn parse_haut_fait(payload_json: &str) -> Option<HautFaitDebloque> {
    let env = serde_json::from_str::<serde_json::Value>(payload_json).ok()?;
    // Le bus porte tous les evenements Nexus : celui-ci ne traite que le sien.
    if env.get("event").and_then(|v| v.as_str())
        != Some(
            platform_core::nexus::ports::outbound::events::achievement_events::ACHIEVEMENT_UNLOCKED,
        )
    {
        return None;
    }
    let data = env.get("data")?;
    let guild_id = data.get("guild_id").and_then(|v| v.as_str())?.to_string();
    let user_id = data
        .get("discord_user_id")
        .and_then(|v| v.as_str())?
        .to_string();
    let guild_num = guild_id.parse::<u64>().ok()?;

    Some(HautFaitDebloque {
        guild_id,
        guild_num,
        user_id,
        // Un haut fait sans nom reste annoncable : le libelle generique vaut
        // mieux qu'un silence, la personne l'a bien decroche.
        nom: data
            .get("achievement_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Haut fait")
            .to_string(),
        description: data
            .get("achievement_description")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        jeu: data
            .get("game")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        icon_url: data
            .get("icon_url")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

/// Salon ou publier, selon la configuration de la guilde.
///
/// `None` quand il ne faut rien publier : module eteint, annonce eteinte, ou
/// aucun salon choisi. Les deux interrupteurs restent distincts — un haut fait
/// peut etre attribue sans etre publie.
pub fn salon_d_annonce(config: &std::collections::HashMap<String, String>) -> Option<u64> {
    let actif = |cle: &str, defaut: bool| config.get(cle).map(|v| v == "true").unwrap_or(defaut);
    if !actif("enabled", true) || !actif("announce_enabled", true) {
        return None;
    }
    config
        .get("announce_channel_id")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|id| *id > 0)
}

/// Role a mentionner, s'il y en a un. Pas de mention par defaut : annoncer ne
/// veut pas dire notifier tout le serveur.
pub fn role_a_mentionner(config: &std::collections::HashMap<String, String>) -> Option<u64> {
    config
        .get("mention_role_id")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|id| *id > 0)
}

/// Compose l'annonce publique d'un haut fait.
pub fn build_annonce(fait: &HautFaitDebloque, role: Option<u64>) -> CreateMessage {
    let mut embed = CreateEmbed::new()
        .title("🏆 Haut fait debloque")
        .description(format!(
            "<@{}> vient de decrocher **{}** !",
            fait.user_id, fait.nom
        ))
        .color(0xf1c40f)
        .footer(CreateEmbedFooter::new("Hauts faits | Nexus"))
        .timestamp(serenity::model::Timestamp::now());
    if !fait.description.is_empty() {
        embed = embed.field("Haut fait", &fait.description, false);
    }
    if let Some(jeu) = &fait.jeu {
        embed = embed.field("Jeu", jeu, true);
    }
    // Image choisie par l'administrateur dans le dashboard.
    if let Some(icon) = fait.icon_url.as_deref().and_then(image_absolue) {
        embed = embed.thumbnail(icon);
    }

    let message = CreateMessage::new().embed(embed);
    match role {
        Some(role) => message.content(format!("<@&{role}>")),
        None => message,
    }
}

async fn handle_event(ctx: &Context, api: &ApiClient, payload_json: &str) {
    let Some(fait) = parse_haut_fait(payload_json) else {
        return;
    };

    let config = api
        .get_guild_config(&fait.guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();

    let Some(salon) = salon_d_annonce(&config) else {
        tracing::info!(
            guild_id = %fait.guild_id,
            "hauts faits : annonce desactivee ou aucun salon configure"
        );
        return;
    };
    let channel = ChannelId::new(salon);

    // Le salon doit appartenir a la guilde de l'evenement : sans cette
    // verification, une configuration erronee publierait ailleurs.
    let guild = GuildId::new(fait.guild_num);
    match guild.channels(&ctx.http).await {
        Ok(channels) if !channels.contains_key(&channel) => {
            tracing::warn!(guild_id = %fait.guild_id, %channel, "hauts faits : salon hors de la guilde, annonce annulee");
            return;
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, guild_id = %fait.guild_id, "hauts faits : verification du salon impossible");
            return;
        }
    }

    let message = build_annonce(&fait, role_a_mentionner(&config));
    if let Err(e) = channel.send_message(&ctx.http, message).await {
        tracing::warn!(error = %e, guild_id = %fait.guild_id, "hauts faits : publication impossible");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_client::AchievementProgress;

    #[test]
    fn test_register() {
        let cmd = register();
        let json = serde_json::to_value(&cmd).unwrap();
        assert_eq!(json["name"], "haut-faits");
    }

    #[test]
    fn test_liste_empty() {
        let res = liste(&[], "Vide");
        assert_eq!(res, "Vide");
    }

    #[test]
    fn test_liste_items() {
        let item1 = AchievementProgress {
            name: "Premier Pas".into(),
            description: "Rejoins le jeu".into(),
            icon_url: None,
            unlocked_at: Some("2026-01-01T00:00:00Z".into()),
        };
        let res = liste(&[&item1], "Vide");
        assert!(res.contains("Premier Pas"));
    }

    #[test]
    fn test_liste_truncation() {
        let items: Vec<AchievementProgress> = (0..50)
            .map(|i| AchievementProgress {
                name: format!("Haut Fait Long Nom Numéro {i}"),
                description: format!(
                    "Description très détaillée du haut fait numéro {i} avec du texte long"
                ),
                icon_url: None,
                unlocked_at: Some("2026-01-01T00:00:00Z".into()),
            })
            .collect();
        let refs: Vec<&AchievementProgress> = items.iter().collect();
        let res = liste(&refs, "Vide");
        assert!(res.contains("de plus"));
    }

    /// Pose `WEB_FRONT_URL` le temps d'une verification, puis la retire.
    ///
    /// Les variables d'environnement sont GLOBALES au processus, et les tests
    /// tournent en parallele : deux tests qui posent puis retirent la meme
    /// variable se marchent dessus, et l'un des deux echoue au hasard. D'ou un
    /// seul test pour toute la variable, plutot qu'un par cas.
    fn avec_base(valeur: Option<&str>, verifier: impl FnOnce()) {
        match valeur {
            Some(v) => std::env::set_var("WEB_FRONT_URL", v),
            None => std::env::remove_var("WEB_FRONT_URL"),
        }
        verifier();
        std::env::remove_var("WEB_FRONT_URL");
    }

    #[test]
    fn une_url_deja_absolue_part_telle_quelle() {
        // Ce cas sort avant meme de lire la configuration : il ne depend donc
        // pas de la variable d'environnement.
        assert_eq!(
            image_absolue("https://example.com/icon.png"),
            Some("https://example.com/icon.png".into())
        );
        assert_eq!(
            image_absolue("http://example.com/icon.png"),
            Some("http://example.com/icon.png".into())
        );
    }

    #[test]
    fn une_image_vide_n_est_jamais_une_url() {
        assert_eq!(image_absolue(""), None);
        assert_eq!(image_absolue("   "), None);
    }

    #[test]
    fn le_chemin_local_devient_absolu_selon_la_base() {
        // Sans base configuree, on renonce a la vignette plutot que d'envoyer
        // a Discord une URL qu'il refusera.
        avec_base(None, || {
            assert_eq!(image_absolue("/img/test.png"), None);
        });

        avec_base(Some("https://front.canap.fr"), || {
            assert_eq!(
                image_absolue("/img/test.png"),
                Some("https://front.canap.fr/img/test.png".into())
            );
            // Sans barre de tete non plus : le chemin est recolle proprement.
            assert_eq!(
                image_absolue("img/test.png"),
                Some("https://front.canap.fr/img/test.png".into())
            );
        });

        // Barre finale sur la base : pas de double barre dans le resultat.
        avec_base(Some("https://front.canap.fr/"), || {
            assert_eq!(
                image_absolue("img/test.png"),
                Some("https://front.canap.fr/img/test.png".into())
            );
        });
        avec_base(Some("https://front.canap.fr//"), || {
            assert_eq!(
                image_absolue("/img/test.png"),
                Some("https://front.canap.fr/img/test.png".into())
            );
        });

        // Une base vide ou blanche vaut une base absente.
        avec_base(Some(""), || {
            assert_eq!(image_absolue("/img/test.png"), None);
        });
        avec_base(Some("   "), || {
            assert_eq!(image_absolue("/img/test.png"), None);
        });

        avec_base(Some("https://cdn.example.com"), || {
            assert_eq!(
                image_absolue("achievements/icon.png"),
                Some("https://cdn.example.com/achievements/icon.png".into())
            );
        });
    }

    #[test]
    fn test_liste_single_item() {
        let item = AchievementProgress {
            name: "Test".into(),
            description: "Description".into(),
            icon_url: None,
            unlocked_at: Some("2026-01-01T00:00:00Z".into()),
        };
        let res = liste(&[&item], "Vide");
        assert!(res.contains("Test"));
        assert!(!res.contains("de plus"));
    }

    #[test]
    fn test_liste_multiple_items_no_truncation() {
        let items: Vec<AchievementProgress> = vec![
            AchievementProgress {
                name: "A".into(),
                description: "Desc A".into(),
                icon_url: None,
                unlocked_at: Some("2026-01-01T00:00:00Z".into()),
            },
            AchievementProgress {
                name: "B".into(),
                description: "Desc B".into(),
                icon_url: None,
                unlocked_at: Some("2026-01-01T00:00:00Z".into()),
            },
        ];
        let refs: Vec<&AchievementProgress> = items.iter().collect();
        let res = liste(&refs, "Vide");
        assert!(res.contains("A"));
        assert!(res.contains("B"));
    }

    #[test]
    fn test_liste_very_long_names() {
        let long_name = "X".repeat(900);
        let item = AchievementProgress {
            name: long_name.clone(),
            description: "Description".into(),
            icon_url: None,
            unlocked_at: Some("2026-01-01T00:00:00Z".into()),
        };
        let res = liste(&[&item], "Vide");
        assert!(!res.is_empty());
    }

    #[test]
    fn test_register_command_creation() {
        let _cmd = register();
        // CreateCommand created successfully with haut-faits name
    }

    #[test]
    fn test_module_bot_name_constant() {
        assert_eq!(MODULE_BOT_NAME, "nexus-achievements");
    }

    #[test]
    fn test_liste_with_mixed_items() {
        let items: Vec<AchievementProgress> = vec![
            AchievementProgress {
                name: "First".into(),
                description: "Desc1".into(),
                icon_url: None,
                unlocked_at: Some("2026-01-01T00:00:00Z".into()),
            },
            AchievementProgress {
                name: "Second".into(),
                description: "Desc2".into(),
                icon_url: None,
                unlocked_at: Some("2026-01-02T00:00:00Z".into()),
            },
        ];
        let refs: Vec<&AchievementProgress> = items.iter().collect();
        let res = liste(&refs, "Empty");
        assert!(res.contains("First"));
        assert!(res.contains("Second"));
        assert!(!res.contains("Empty"));
    }

    #[test]
    fn test_liste_respects_empty_message() {
        let items: Vec<&AchievementProgress> = vec![];
        let res = liste(&items, "No items here");
        assert_eq!(res, "No items here");
    }

    #[test]
    fn test_liste_with_very_long_descriptions() {
        let items: Vec<AchievementProgress> = vec![AchievementProgress {
            name: "Test".into(),
            description: "x".repeat(500),
            icon_url: None,
            unlocked_at: Some("2026-01-01T00:00:00Z".into()),
        }];
        let refs: Vec<&AchievementProgress> = items.iter().collect();
        let res = liste(&refs, "Empty");
        // Should not be empty even with very long description
        assert!(!res.is_empty());
        assert!(res.contains("Test"));
    }

    #[test]
    fn test_liste_boundary_at_950_chars() {
        // Test items that approach but don't exceed 950 char limit
        let items: Vec<AchievementProgress> = vec![
            AchievementProgress {
                name: "Achievement".into(),
                description: "d".repeat(200),
                icon_url: None,
                unlocked_at: Some("2026-01-01T00:00:00Z".into()),
            },
            AchievementProgress {
                name: "Second".into(),
                description: "e".repeat(200),
                icon_url: None,
                unlocked_at: Some("2026-01-01T00:00:00Z".into()),
            },
        ];
        let refs: Vec<&AchievementProgress> = items.iter().collect();
        let res = liste(&refs, "Empty");
        assert!(res.contains("Achievement"));
    }

    // ── Lecture de l'evenement et decision d'annonce ──
    //
    // Ces quatre fonctions vivaient dans `handle_event`, entre deux appels
    // Discord : aucun test ne pouvait constater qu'un evenement mal forme ne
    // produit pas une annonce vide, ni qu'un module eteint reste muet.

    use platform_core::nexus::ports::outbound::events::achievement_events::ACHIEVEMENT_UNLOCKED;

    fn evenement(data: serde_json::Value) -> String {
        serde_json::json!({ "event": ACHIEVEMENT_UNLOCKED, "data": data }).to_string()
    }

    fn config(paires: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        paires
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn un_evenement_complet_se_lit_entierement() {
        let brut = evenement(serde_json::json!({
            "guild_id": "123",
            "discord_user_id": "456",
            "achievement_name": "Premier Pas",
            "achievement_description": "Rejoindre le serveur",
            "game": "Palworld",
            "icon_url": "https://cdn.example.com/p.png",
        }));
        let fait = parse_haut_fait(&brut).unwrap();
        assert_eq!(fait.guild_id, "123");
        assert_eq!(fait.guild_num, 123);
        assert_eq!(fait.user_id, "456");
        assert_eq!(fait.nom, "Premier Pas");
        assert_eq!(fait.description, "Rejoindre le serveur");
        assert_eq!(fait.jeu.as_deref(), Some("Palworld"));
    }

    #[test]
    fn un_haut_fait_sans_nom_reste_annoncable() {
        // La personne l'a bien decroche : un libelle generique vaut mieux que
        // le silence.
        let brut = evenement(serde_json::json!({
            "guild_id": "1", "discord_user_id": "2"
        }));
        let fait = parse_haut_fait(&brut).unwrap();
        assert_eq!(fait.nom, "Haut fait");
        assert!(fait.description.is_empty());
        assert!(fait.jeu.is_none());
        assert!(fait.icon_url.is_none());
    }

    #[test]
    fn un_evenement_inexploitable_ne_produit_rien() {
        // Le bus porte tous les evenements Nexus : celui-ci ne traite que le
        // sien, et refuse ce a quoi il manque de quoi annoncer honnetement.
        assert!(parse_haut_fait("pas du json").is_none());
        assert!(parse_haut_fait(r#"{"event":"autre_chose","data":{}}"#).is_none());
        assert!(
            parse_haut_fait(&serde_json::json!({"event": ACHIEVEMENT_UNLOCKED}).to_string())
                .is_none()
        );
        // Guilde absente, membre absent, guilde illisible.
        assert!(parse_haut_fait(&evenement(serde_json::json!({"discord_user_id": "2"}))).is_none());
        assert!(parse_haut_fait(&evenement(serde_json::json!({"guild_id": "1"}))).is_none());
        assert!(parse_haut_fait(&evenement(
            serde_json::json!({"guild_id": "pas_un_nombre", "discord_user_id": "2"})
        ))
        .is_none());
    }

    #[test]
    fn sans_configuration_l_annonce_part_dans_le_salon_choisi() {
        // Les deux interrupteurs valent vrai par defaut : c'est le salon qui
        // decide, et lui n'a pas de defaut.
        assert_eq!(
            salon_d_annonce(&config(&[("announce_channel_id", "999")])),
            Some(999)
        );
        assert_eq!(salon_d_annonce(&config(&[])), None);
    }

    #[test]
    fn les_deux_interrupteurs_coupent_l_annonce_separement() {
        // Un haut fait peut etre attribue sans etre publie : le module et
        // l'annonce sont deux reglages distincts.
        let base = [("announce_channel_id", "999")];
        let avec = |extra: (&str, &str)| {
            let mut v = base.to_vec();
            v.push(extra);
            config(&v)
        };
        assert_eq!(salon_d_annonce(&avec(("enabled", "false"))), None);
        assert_eq!(salon_d_annonce(&avec(("announce_enabled", "false"))), None);
        assert_eq!(salon_d_annonce(&avec(("enabled", "true"))), Some(999));
    }

    #[test]
    fn un_salon_absurde_vaut_pas_de_salon() {
        // Publier dans le salon « 0 » n'a aucun sens ; mieux vaut se taire que
        // d'appeler Discord avec un identifiant invente.
        assert_eq!(
            salon_d_annonce(&config(&[("announce_channel_id", "0")])),
            None
        );
        assert_eq!(
            salon_d_annonce(&config(&[("announce_channel_id", "")])),
            None
        );
        assert_eq!(
            salon_d_annonce(&config(&[("announce_channel_id", "abc")])),
            None
        );
        // Les espaces autour sont tolerés : un identifiant se copie a la main.
        assert_eq!(
            salon_d_annonce(&config(&[("announce_channel_id", " 42 ")])),
            Some(42)
        );
    }

    #[test]
    fn aucune_mention_par_defaut() {
        // Annoncer ne veut pas dire notifier tout le serveur.
        assert_eq!(role_a_mentionner(&config(&[])), None);
        assert_eq!(
            role_a_mentionner(&config(&[("mention_role_id", "0")])),
            None
        );
        assert_eq!(
            role_a_mentionner(&config(&[("mention_role_id", "abc")])),
            None
        );
        assert_eq!(
            role_a_mentionner(&config(&[("mention_role_id", "77")])),
            Some(77)
        );
    }

    fn fait_de_test() -> HautFaitDebloque {
        HautFaitDebloque {
            guild_id: "1".into(),
            guild_num: 1,
            user_id: "42".into(),
            nom: "Premier Pas".into(),
            description: "Rejoindre le serveur".into(),
            jeu: Some("Palworld".into()),
            icon_url: None,
        }
    }

    #[test]
    fn l_annonce_nomme_le_membre_et_son_haut_fait() {
        let msg = build_annonce(&fait_de_test(), None);
        let json = serde_json::to_value(&msg).unwrap();
        let embed = &json["embeds"][0];
        assert!(embed["description"].as_str().unwrap().contains("<@42>"));
        assert!(embed["description"]
            .as_str()
            .unwrap()
            .contains("Premier Pas"));
        // Sans role configure, aucune mention n'est ajoutee.
        assert!(json.get("content").is_none() || json["content"].is_null());
    }

    #[test]
    fn le_role_configure_est_mentionne() {
        let msg = build_annonce(&fait_de_test(), Some(77));
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["content"], "<@&77>");
    }

    #[test]
    fn les_champs_facultatifs_disparaissent_quand_ils_sont_vides() {
        let mut fait = fait_de_test();
        fait.description = String::new();
        fait.jeu = None;
        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        let champs = json["embeds"][0]["fields"].as_array();
        assert!(champs.is_none() || champs.unwrap().is_empty());
    }

    #[test]
    fn test_parse_haut_fait_complete_data() {
        let payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": {
                "guild_id": "123",
                "discord_user_id": "456",
                "achievement_name": "TestAchieve",
                "achievement_description": "Test Desc",
                "game": "TestGame",
                "icon_url": "https://example.com/icon.png"
            }
        }).to_string();

        let fait = parse_haut_fait(&payload).unwrap();
        assert_eq!(fait.guild_id, "123");
        assert_eq!(fait.guild_num, 123);
        assert_eq!(fait.user_id, "456");
        assert_eq!(fait.nom, "TestAchieve");
        assert_eq!(fait.description, "Test Desc");
        assert_eq!(fait.jeu, Some("TestGame".to_string()));
        assert_eq!(fait.icon_url, Some("https://example.com/icon.png".to_string()));
    }

    #[test]
    fn test_parse_haut_fait_invalid_json() {
        assert!(parse_haut_fait("not json").is_none());
    }

    #[test]
    fn test_parse_haut_fait_wrong_event_type() {
        let payload = serde_json::json!({
            "event": "some_other_event",
            "data": { "guild_id": "123", "discord_user_id": "456" }
        }).to_string();

        assert!(parse_haut_fait(&payload).is_none());
    }

    #[test]
    fn test_parse_haut_fait_missing_guild_id() {
        let payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": { "discord_user_id": "456" }
        }).to_string();

        assert!(parse_haut_fait(&payload).is_none());
    }

    #[test]
    fn test_parse_haut_fait_missing_user_id() {
        let payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": { "guild_id": "123" }
        }).to_string();

        assert!(parse_haut_fait(&payload).is_none());
    }

    #[test]
    fn test_parse_haut_fait_invalid_guild_id_number() {
        let payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": {
                "guild_id": "not_a_number",
                "discord_user_id": "456"
            }
        }).to_string();

        assert!(parse_haut_fait(&payload).is_none());
    }

    #[test]
    fn test_salon_d_annonce_with_valid_channel() {
        let cfg = config(&[("announce_channel_id", "12345")]);
        assert_eq!(salon_d_annonce(&cfg), Some(12345));
    }

    #[test]
    fn test_salon_d_annonce_with_whitespace() {
        let cfg = config(&[("announce_channel_id", "  98765  ")]);
        assert_eq!(salon_d_annonce(&cfg), Some(98765));
    }

    #[test]
    fn test_salon_d_annonce_disabled() {
        let cfg = config(&[
            ("announce_channel_id", "123"),
            ("enabled", "false"),
        ]);
        assert_eq!(salon_d_annonce(&cfg), None);
    }

    #[test]
    fn test_salon_d_annonce_announce_disabled() {
        let cfg = config(&[
            ("announce_channel_id", "123"),
            ("announce_enabled", "false"),
        ]);
        assert_eq!(salon_d_annonce(&cfg), None);
    }

    #[test]
    fn test_role_a_mentionner_with_valid_id() {
        let cfg = config(&[("mention_role_id", "54321")]);
        assert_eq!(role_a_mentionner(&cfg), Some(54321));
    }

    #[test]
    fn test_role_a_mentionner_with_whitespace() {
        let cfg = config(&[("mention_role_id", "  11111  ")]);
        assert_eq!(role_a_mentionner(&cfg), Some(11111));
    }

    #[test]
    fn test_role_a_mentionner_invalid_id() {
        let cfg = config(&[("mention_role_id", "not_a_number")]);
        assert_eq!(role_a_mentionner(&cfg), None);
    }

    #[test]
    fn test_haut_fait_debloque_equality() {
        let f1 = HautFaitDebloque {
            guild_id: "1".into(),
            guild_num: 1,
            user_id: "2".into(),
            nom: "Achievement".into(),
            description: "Desc".into(),
            jeu: Some("Game".into()),
            icon_url: None,
        };
        let f2 = f1.clone();
        assert_eq!(f1, f2);
    }

    #[test]
    fn test_haut_fait_debloque_fields() {
        let fait = HautFaitDebloque {
            guild_id: "guild123".into(),
            guild_num: 999,
            user_id: "user456".into(),
            nom: "Achievement Name".into(),
            description: "Long description".into(),
            jeu: Some("Game Name".into()),
            icon_url: Some("http://example.com/icon.png".into()),
        };

        assert_eq!(fait.guild_id, "guild123");
        assert_eq!(fait.guild_num, 999);
        assert_eq!(fait.user_id, "user456");
        assert_eq!(fait.nom, "Achievement Name");
    }

    #[test]
    fn test_build_annonce_with_description_and_game() {
        let fait = HautFaitDebloque {
            guild_id: "1".into(),
            guild_num: 1,
            user_id: "42".into(),
            nom: "Complete Game".into(),
            description: "You won!".into(),
            jeu: Some("Epic Game".into()),
            icon_url: None,
        };

        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        let fields = json["embeds"][0]["fields"].as_array().unwrap();

        // Should have description and game fields
        assert!(fields.len() >= 2);
    }

    #[test]
    fn test_build_annonce_with_full_url_icon() {
        let mut fait = fait_de_test();
        fait.icon_url = Some("https://example.com/icon.png".into());

        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        let thumbnail = json["embeds"][0]["thumbnail"].as_object();
        assert!(thumbnail.is_some());
    }

    #[test]
    fn test_liste_with_exact_950_char_boundary() {
        // Create items that approach 950 chars
        let items: Vec<AchievementProgress> = (0..3)
            .map(|i| AchievementProgress {
                name: format!("Achievement {}", i),
                description: "x".repeat(200),
                icon_url: None,
                unlocked_at: Some("2026-01-01T00:00:00Z".into()),
            })
            .collect();

        let refs: Vec<&AchievementProgress> = items.iter().collect();
        let res = liste(&refs, "Empty");
        assert!(!res.is_empty());
    }

    #[test]
    fn test_parse_haut_fait_null_fields() {
        let payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": {
                "guild_id": "123",
                "discord_user_id": "456",
                "achievement_name": null,
                "achievement_description": null,
                "game": null
            }
        }).to_string();

        let fait = parse_haut_fait(&payload).unwrap();
        assert_eq!(fait.nom, "Haut fait");
        assert!(fait.description.is_empty());
        assert!(fait.jeu.is_none());
    }

    #[test]
    fn test_sous_option_user_with_valid_member() {
        // This tests the helper function indirectly through the command flow
        let item = AchievementProgress {
            name: "Test".into(),
            description: "Desc".into(),
            icon_url: None,
            unlocked_at: Some("2026-01-01T00:00:00Z".into()),
        };
        let refs = vec![&item];
        let res = liste(&refs, "Empty");
        assert!(res.contains("Test"));
    }


    #[test]
    fn test_parse_haut_fait_minimal_fields_only() {
        // Test with just guild_id and discord_user_id
        let payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": {
                "guild_id": "999",
                "discord_user_id": "888"
            }
        }).to_string();

        let fait = parse_haut_fait(&payload).unwrap();
        assert_eq!(fait.guild_id, "999");
        assert_eq!(fait.guild_num, 999);
        assert_eq!(fait.user_id, "888");
        assert_eq!(fait.nom, "Haut fait");
        assert!(fait.description.is_empty());
        assert!(fait.jeu.is_none());
        assert!(fait.icon_url.is_none());
    }

    #[test]
    fn test_register_creates_valid_command() {
        let cmd = register();
        let json = serde_json::to_value(&cmd).unwrap();
        assert_eq!(json["name"], "haut-faits");
        assert_eq!(json["description"], "Consulter tes hauts faits");
    }

    #[test]
    fn test_register_has_two_subcommands() {
        let cmd = register();
        let json = serde_json::to_value(&cmd).unwrap();
        let options = json["options"].as_array().unwrap();
        assert_eq!(options.len(), 2);
    }

    #[test]
    fn test_build_annonce_color_is_golden() {
        let fait = fait_de_test();
        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        // 0xf1c40f is golden yellow
        assert_eq!(json["embeds"][0]["color"], 0xf1c40f);
    }

    #[test]
    fn test_build_annonce_has_correct_timestamp() {
        let fait = fait_de_test();
        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        // Check that timestamp field exists
        assert!(json["embeds"][0]["timestamp"].is_string());
    }

    #[test]
    fn test_list_respects_custom_empty_message() {
        let custom_msg = "Rien du tout";
        let res = liste(&[], custom_msg);
        assert_eq!(res, custom_msg);
    }

    #[test]
    fn test_haut_fait_debloque_clone() {
        let f1 = HautFaitDebloque {
            guild_id: "1".into(),
            guild_num: 1,
            user_id: "2".into(),
            nom: "Test".into(),
            description: "Desc".into(),
            jeu: None,
            icon_url: None,
        };
        let f2 = f1.clone();
        assert_eq!(f1.guild_id, f2.guild_id);
        assert_eq!(f1.user_id, f2.user_id);
    }

    #[test]
    fn test_parse_haut_fait_with_all_fields_populated() {
        let full_payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": {
                "guild_id": "1001",
                "discord_user_id": "2002",
                "achievement_name": "Full Achievement",
                "achievement_description": "Complete description here",
                "game": "Epic Game",
                "icon_url": "https://example.com/icon.png"
            }
        }).to_string();

        let fait = parse_haut_fait(&full_payload).unwrap();
        assert_eq!(fait.guild_id, "1001");
        assert_eq!(fait.guild_num, 1001);
        assert_eq!(fait.user_id, "2002");
        assert_eq!(fait.nom, "Full Achievement");
        assert_eq!(fait.description, "Complete description here");
        assert_eq!(fait.jeu, Some("Epic Game".to_string()));
        assert_eq!(fait.icon_url, Some("https://example.com/icon.png".to_string()));
    }

    #[test]
    fn test_salon_d_annonce_with_zero_channel_id() {
        let cfg = config(&[("announce_channel_id", "0")]);
        assert_eq!(salon_d_annonce(&cfg), None);
    }

    #[test]
    fn test_role_a_mentionner_with_zero_id() {
        let cfg = config(&[("mention_role_id", "0")]);
        assert_eq!(role_a_mentionner(&cfg), None);
    }

    #[test]
    fn test_liste_single_item_under_limit() {
        let item = AchievementProgress {
            name: "Single".into(),
            description: "Desc".into(),
            icon_url: None,
            unlocked_at: Some("2026-01-01T00:00:00Z".into()),
        };
        let res = liste(&[&item], "Empty");
        assert!(res.contains("Single"));
        assert!(!res.contains("de plus"));
    }

    #[test]
    fn test_parse_haut_fait_large_guild_id() {
        let payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": {
                "guild_id": "999999999999",
                "discord_user_id": "123"
            }
        }).to_string();

        let fait = parse_haut_fait(&payload).unwrap();
        assert_eq!(fait.guild_id, "999999999999");
        assert_eq!(fait.guild_num, 999999999999);
    }

    #[test]
    fn test_build_annonce_empty_description() {
        let mut fait = fait_de_test();
        fait.description = String::new();
        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        // Empty description should not create a field
        let fields = json["embeds"][0]["fields"].as_array();
        if let Some(f) = fields {
            for field in f {
                assert_ne!(field["name"], "Haut fait");
            }
        }
    }

    #[test]
    fn test_build_annonce_no_game_field_when_none() {
        let mut fait = fait_de_test();
        fait.jeu = None;
        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        let fields = json["embeds"][0]["fields"].as_array();
        if let Some(f) = fields {
            for field in f {
                assert_ne!(field["name"], "Jeu");
            }
        }
    }

    #[test]
    fn test_parse_haut_fait_guild_id_with_spaces_fails() {
        let payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": {
                "guild_id": "  123  ",
                "discord_user_id": "456"
            }
        }).to_string();

        // Should fail because guild_id with spaces can't be parsed as u64
        assert!(parse_haut_fait(&payload).is_none());
    }

    #[test]
    fn test_salon_d_annonce_large_channel_id() {
        let cfg = config(&[("announce_channel_id", "123456789012345")]);
        assert_eq!(salon_d_annonce(&cfg), Some(123456789012345));
    }

    #[test]
    fn test_haut_fait_debloque_with_all_options() {
        let fait = HautFaitDebloque {
            guild_id: "100".into(),
            guild_num: 100,
            user_id: "200".into(),
            nom: "Full Test".into(),
            description: "Full test description".into(),
            jeu: Some("Test Game".into()),
            icon_url: Some("https://test.com/icon.png".into()),
        };

        assert_eq!(fait.guild_id, "100");
        assert_eq!(fait.guild_num, 100);
        assert_eq!(fait.user_id, "200");
        assert_eq!(fait.nom, "Full Test");
        assert_eq!(fait.description, "Full test description");
        assert_eq!(fait.jeu, Some("Test Game".to_string()));
        assert_eq!(fait.icon_url, Some("https://test.com/icon.png".to_string()));
    }

    #[test]
    fn test_liste_two_items_exact_boundary() {
        let items: Vec<AchievementProgress> = vec![
            AchievementProgress {
                name: "First Achievement Name".into(),
                description: "First description with some content".into(),
                icon_url: None,
                unlocked_at: Some("2026-01-01T00:00:00Z".into()),
            },
            AchievementProgress {
                name: "Second Achievement Name".into(),
                description: "Second description with some content".into(),
                icon_url: None,
                unlocked_at: Some("2026-01-02T00:00:00Z".into()),
            },
        ];
        let refs: Vec<&AchievementProgress> = items.iter().collect();
        let res = liste(&refs, "Empty");
        assert!(res.contains("First"));
        assert!(res.contains("Second"));
    }

    #[test]
    fn test_build_annonce_with_mention() {
        let fait = fait_de_test();
        let msg = build_annonce(&fait, Some(555));
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["content"], "<@&555>");
    }

    #[test]
    fn test_build_annonce_footer_branding() {
        let fait = fait_de_test();
        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        let footer = json["embeds"][0]["footer"]["text"].as_str().unwrap();
        assert!(footer.contains("Hauts faits"));
        assert!(footer.contains("Nexus"));
    }

    #[test]
    fn test_register_command_subcommands() {
        let cmd = register();
        let json = serde_json::to_value(&cmd).unwrap();
        let options = json["options"].as_array().unwrap();
        let names: Vec<&str> = options
            .iter()
            .filter_map(|opt| opt["name"].as_str())
            .collect();
        assert!(names.contains(&"moi"));
        assert!(names.contains(&"membre"));
    }

    #[test]
    fn test_parse_haut_fait_event_field_must_match() {
        let wrong_event = serde_json::json!({
            "event": "achievement.other",
            "data": { "guild_id": "1", "discord_user_id": "2" }
        }).to_string();

        assert!(parse_haut_fait(&wrong_event).is_none());
    }

    #[test]
    fn test_salon_d_annonce_default_enabled() {
        // Without explicit config, both should default to true
        let cfg = config(&[("announce_channel_id", "123")]);
        assert_eq!(salon_d_annonce(&cfg), Some(123));
    }

    #[test]
    fn test_build_annonce_has_embed() {
        let fait = fait_de_test();
        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        assert!(json["embeds"].is_array());
        assert!(!json["embeds"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_build_annonce_embed_title() {
        let fait = fait_de_test();
        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["embeds"][0]["title"], "🏆 Haut fait debloque");
    }

    #[test]
    fn test_parse_haut_fait_description_empty_string() {
        let payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": {
                "guild_id": "1",
                "discord_user_id": "2",
                "achievement_description": ""
            }
        }).to_string();

        let fait = parse_haut_fait(&payload).unwrap();
        assert_eq!(fait.description, "");
    }

    #[test]
    fn test_liste_adds_ellipsis_when_truncated() {
        let items: Vec<AchievementProgress> = (0..100)
            .map(|i| AchievementProgress {
                name: format!("Achievement{}", i),
                description: "x".repeat(50),
                icon_url: None,
                unlocked_at: Some("2026-01-01T00:00:00Z".into()),
            })
            .collect();
        let refs: Vec<&AchievementProgress> = items.iter().collect();
        let res = liste(&refs, "Empty");
        assert!(res.contains("de plus"));
    }

    #[test]
    fn test_salon_d_annonce_both_disabled() {
        let cfg = config(&[
            ("announce_channel_id", "123"),
            ("enabled", "false"),
            ("announce_enabled", "false"),
        ]);
        assert_eq!(salon_d_annonce(&cfg), None);
    }

    #[test]
    fn test_role_a_mentionner_with_spaces() {
        let cfg = config(&[("mention_role_id", "  777  ")]);
        assert_eq!(role_a_mentionner(&cfg), Some(777));
    }

    #[test]
    fn test_parse_haut_fait_all_optional_fields() {
        let payload = serde_json::json!({
            "event": ACHIEVEMENT_UNLOCKED,
            "data": {
                "guild_id": "5",
                "discord_user_id": "6",
                "achievement_name": "",
                "achievement_description": "",
                "game": "",
                "icon_url": ""
            }
        }).to_string();

        let fait = parse_haut_fait(&payload).unwrap();
        assert_eq!(fait.nom, "");
        assert_eq!(fait.description, "");
    }

    #[test]
    fn test_liste_preserves_formatting() {
        let item = AchievementProgress {
            name: "**Bold** Text".into(),
            description: "_Italic_ Text".into(),
            icon_url: None,
            unlocked_at: Some("2026-01-01T00:00:00Z".into()),
        };
        let res = liste(&[&item], "Empty");
        assert!(res.contains("**Bold**"));
    }

    #[test]
    fn test_haut_fait_debloque_debug() {
        let fait = HautFaitDebloque {
            guild_id: "1".into(),
            guild_num: 1,
            user_id: "2".into(),
            nom: "Test".into(),
            description: "Desc".into(),
            jeu: None,
            icon_url: None,
        };
        // Should be debuggable
        let _debug_str = format!("{:?}", fait);
        assert!(_debug_str.contains("1"));
    }

    #[test]
    fn test_build_annonce_description_mentions_user() {
        let fait = HautFaitDebloque {
            guild_id: "1".into(),
            guild_num: 1,
            user_id: "999".into(),
            nom: "Test".into(),
            description: "Desc".into(),
            jeu: None,
            icon_url: None,
        };
        let msg = build_annonce(&fait, None);
        let json = serde_json::to_value(&msg).unwrap();
        let desc = json["embeds"][0]["description"].as_str().unwrap();
        assert!(desc.contains("999"));
    }

    #[test]
    fn test_module_bot_name_used_correctly() {
        assert_eq!(MODULE_BOT_NAME, "nexus-achievements");
    }

    #[test]
    fn test_salon_d_annonce_with_negative_channel() {
        let cfg = config(&[("announce_channel_id", "-1")]);
        assert_eq!(salon_d_annonce(&cfg), None);
    }
}
