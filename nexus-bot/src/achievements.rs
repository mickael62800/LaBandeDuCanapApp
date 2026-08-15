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

async fn handle_event(ctx: &Context, api: &ApiClient, payload_json: &str) {
    let Ok(env) = serde_json::from_str::<serde_json::Value>(payload_json) else {
        return;
    };
    if env.get("event").and_then(|v| v.as_str())
        != Some(
            platform_core::nexus::ports::outbound::events::achievement_events::ACHIEVEMENT_UNLOCKED,
        )
    {
        return;
    }
    let Some(data) = env.get("data") else { return };

    let (Some(guild_id), Some(user_id)) = (
        data.get("guild_id").and_then(|v| v.as_str()),
        data.get("discord_user_id").and_then(|v| v.as_str()),
    ) else {
        return;
    };
    let Ok(guild_num) = guild_id.parse::<u64>() else {
        return;
    };

    let config = api
        .get_guild_config(guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let actif = |cle: &str, defaut: bool| config.get(cle).map(|v| v == "true").unwrap_or(defaut);
    // Fail closed sur le module, et l'annonce reste un interrupteur distinct :
    // un haut fait peut etre attribue sans etre publie.
    if !actif("enabled", true) || !actif("announce_enabled", true) {
        return;
    }

    let Some(channel) = config
        .get("announce_channel_id")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|id| *id > 0)
        .map(ChannelId::new)
    else {
        tracing::info!(guild_id, "hauts faits : aucun salon d'annonce configure");
        return;
    };

    // Le salon doit appartenir a la guilde de l'evenement : sans cette
    // verification, une configuration erronee publierait ailleurs.
    let guild = GuildId::new(guild_num);
    match guild.channels(&ctx.http).await {
        Ok(channels) if !channels.contains_key(&channel) => {
            tracing::warn!(guild_id, %channel, "hauts faits : salon hors de la guilde, annonce annulee");
            return;
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, guild_id, "hauts faits : verification du salon impossible");
            return;
        }
    }

    let nom = data
        .get("achievement_name")
        .and_then(|v| v.as_str())
        .unwrap_or("Haut fait");
    let description = data
        .get("achievement_description")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let jeu = data.get("game").and_then(|v| v.as_str());

    let mut embed = CreateEmbed::new()
        .title("🏆 Haut fait debloque")
        .description(format!("<@{user_id}> vient de decrocher **{nom}** !"))
        .color(0xf1c40f)
        .footer(CreateEmbedFooter::new("Hauts faits | Nexus"))
        .timestamp(serenity::model::Timestamp::now());
    if !description.is_empty() {
        embed = embed.field("Haut fait", description, false);
    }
    if let Some(jeu) = jeu {
        embed = embed.field("Jeu", jeu, true);
    }
    // Image choisie par l'administrateur dans le dashboard.
    if let Some(icon) = data
        .get("icon_url")
        .and_then(|v| v.as_str())
        .and_then(image_absolue)
    {
        embed = embed.thumbnail(icon);
    }

    let mut message = CreateMessage::new().embed(embed);
    // Mention desactivee par defaut : elle n'existe que si un role est configure.
    if let Some(role) = config
        .get("mention_role_id")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|id| *id > 0)
    {
        message = message.content(format!("<@&{role}>"));
    }

    if let Err(e) = channel.send_message(&ctx.http, message).await {
        tracing::warn!(error = %e, guild_id, "hauts faits : publication impossible");
    }
}
