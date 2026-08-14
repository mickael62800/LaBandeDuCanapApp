//! Hauts faits : commandes Discord et publication des deblocages.
//!
//! Le bot n'attribue jamais un haut fait lui-meme et ne lit jamais la base :
//! il appelle l'API, qui tranche. Il consomme `achievement.unlocked` sur la
//! stream Redis `nexus:events` pour publier l'annonce.
//!
//! Commandes :
//!   - `/haut-faits`               mes hauts faits (reponse ephemere) ;
//!   - `/haut-faits membre`        ceux d'un autre membre, si la config l'autorise ;
//!   - `/haut-faits lier`          declarer son identite de jeu (SteamID64 Palworld) ;
//!   - `/haut-faits delier`        retirer cette identite.

use std::sync::Arc;

use serenity::all::{
    ChannelId, CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage, GuildId,
};

use crate::api_client::ApiClient;

/// Module dans `bot_guild_config` : salon d'annonce, mention, toggles.
const MODULE_BOT_NAME: &str = "nexus-achievements";

/// Jeux pour lesquels une identite peut etre liee. Palworld est le premier
/// adaptateur ; la liste s'etendra avec les autres jeux du portail.
const JEUX_LIABLES: &[(&str, &str)] = &[("Palworld", "palworld")];

pub fn register() -> CreateCommand {
    let mut lier = CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "lier",
        "Lier ton compte de jeu (SteamID64 pour Palworld)",
    );
    let mut jeu_option =
        CreateCommandOption::new(CommandOptionType::String, "jeu", "Le jeu").required(true);
    let mut jeu_option_delier =
        CreateCommandOption::new(CommandOptionType::String, "jeu", "Le jeu").required(true);
    for (label, value) in JEUX_LIABLES {
        jeu_option = jeu_option.add_string_choice(*label, *value);
        jeu_option_delier = jeu_option_delier.add_string_choice(*label, *value);
    }
    lier = lier
        .add_sub_option(jeu_option)
        .add_sub_option(
            CreateCommandOption::new(CommandOptionType::String, "plateforme", "Ou tu joues")
                .required(true)
                .add_string_choice("Steam", "steam")
                .add_string_choice("Xbox / Microsoft Store", "xbox"),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "identifiant",
                "SteamID64 (17 chiffres) ou XUID / Gamertag Xbox",
            )
            .required(true),
        );

    CreateCommand::new("haut-faits")
        .description("Consulter tes hauts faits et lier ton compte de jeu")
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
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "compte",
            "Voir le compte de jeu que tu as lie",
        ))
        .add_option(lier)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "delier",
                "Retirer ton identite de jeu",
            )
            .add_sub_option(jeu_option_delier),
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
        "compte" => compte(api, ctx, cmd, &guild_id).await,
        "lier" => lier(api, ctx, cmd, &guild_id).await,
        "delier" => delier(api, ctx, cmd, &guild_id).await,
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

/// Rappelle au membre l'identite qu'il a liee pour chaque jeu supporte.
async fn compte(api: &ApiClient, ctx: &Context, cmd: &CommandInteraction, guild_id: &str) {
    let user_id = cmd.user.id.to_string();
    let mut lignes = Vec::new();
    for (label, slug) in JEUX_LIABLES {
        let ligne = match api.get_player_link(guild_id, &user_id, slug).await {
            Ok(Some(link)) => format!(
                "**{label}** : `{}` ({})",
                link.game_player_id, link.platform
            ),
            Ok(None) => format!("**{label}** : _aucun compte lie_"),
            Err(e) => format!("**{label}** : indisponible ({e})"),
        };
        lignes.push(ligne);
    }
    repondre(
        ctx,
        cmd,
        &format!(
            "{}

Pour lier ou changer : `/haut-faits lier`.",
            lignes.join(
                "
"
            )
        ),
    )
    .await;
}

async fn lier(api: &ApiClient, ctx: &Context, cmd: &CommandInteraction, guild_id: &str) {
    let (Some(jeu), Some(identifiant)) = (
        sous_option_str(cmd, "jeu"),
        sous_option_str(cmd, "identifiant"),
    ) else {
        repondre(ctx, cmd, "Jeu et identifiant requis.").await;
        return;
    };
    let plateforme = sous_option_str(cmd, "plateforme").unwrap_or_else(|| "steam".to_string());

    match api
        .link_player(
            guild_id,
            &cmd.user.id.to_string(),
            &jeu,
            &plateforme,
            &identifiant,
        )
        .await
    {
        Ok(link) => {
            repondre(
                ctx,
                cmd,
                &format!(
                    "✅ Compte lie pour **{}** : `{}`.\nTes hauts faits {} pourront maintenant t'etre attribues.",
                    link.game, link.game_player_id, link.game
                ),
            )
            .await;
        }
        // L'API porte le message utile (format du SteamID, identite deja
        // prise) : on le relaie tel quel plutot que de le reformuler.
        Err(e) => repondre(ctx, cmd, &format!("❌ Liaison impossible : {e}")).await,
    }
}

async fn delier(api: &ApiClient, ctx: &Context, cmd: &CommandInteraction, guild_id: &str) {
    let Some(jeu) = sous_option_str(cmd, "jeu") else {
        repondre(ctx, cmd, "Jeu requis.").await;
        return;
    };
    match api
        .unlink_player(guild_id, &cmd.user.id.to_string(), &jeu)
        .await
    {
        Ok(()) => repondre(ctx, cmd, &format!("✅ Identite **{jeu}** retiree.")).await,
        Err(e) => repondre(ctx, cmd, &format!("❌ Suppression impossible : {e}")).await,
    }
}

// ── Options ──────────────────────────────────────────────────────────────

fn sous_option_str(cmd: &CommandInteraction, nom: &str) -> Option<String> {
    let sub = cmd.data.options.first()?;
    let serenity::all::CommandDataOptionValue::SubCommand(options) = &sub.value else {
        return None;
    };
    options
        .iter()
        .find(|o| o.name == nom)
        .and_then(|o| o.value.as_str())
        .map(str::to_owned)
}

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
