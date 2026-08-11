use super::*;

// ── Emoji ──

/// Parse une chaine emoji :
/// - `<:name:123456>` → custom
/// - `<a:name:123456>` → custom anime
/// - sinon → unicode (ex. "🎮")
/// Retourne None si la chaine est vide.
pub(crate) fn parse_reaction_type(raw: &str) -> Option<ReactionType> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    // Serenity parse nativement les emojis custom ("<:name:id>", "<a:name:id>")
    // et les emojis unicode.
    match ReactionType::try_from(s) {
        Ok(rt) => Some(rt),
        Err(_) => Some(ReactionType::Unicode(s.to_string())),
    }
}

pub(super) fn find_game_for_reaction<'a>(games: &'a [Game], reaction: &str) -> Option<&'a Game> {
    games.iter().find(|game| {
        game.emoji.as_deref().is_some_and(|emoji| {
            emoji == reaction
                || parse_reaction_type(emoji).is_some_and(|parsed| match parsed {
                    ReactionType::Custom { id, .. } => id.to_string() == reaction,
                    ReactionType::Unicode(value) => value == reaction,
                    _ => false,
                })
        })
    })
}

pub fn spawn_listener(ctx: Context, api: std::sync::Arc<ApiClient>) {
    let ctx = ctx.clone();
    let api = api.clone();

    // On ecoute la queue Redis de la meme maniere que game_portal.rs
    tokio::spawn(async move {
        crate::event_bus::listen_stream_group(
            "nexus-bot-games".to_string(),
            crate::event_bus::default_consumer_name(),
            move |raw_event| {
                let ctx_clone = ctx.clone();
                let api_clone = api.clone();
                async move {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw_event) {
                        if let (Some(event), Some(data)) = (
                            value.get("event").and_then(|v| v.as_str()),
                            value.get("data"),
                        ) {
                            if event == "games_panel_deploy" {
                                if let (Some(guild_id), Some(channel_id)) = (
                                    data.get("guild_id").and_then(|v| v.as_str()),
                                    data.get("channel_id").and_then(|v| v.as_str()),
                                ) {
                                    let category = data.get("category").and_then(|v| v.as_str());
                                    deploy_panel_from_event(
                                        &ctx_clone, &api_clone, guild_id, channel_id, category,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                }
            },
        )
        .await;
    });
}

async fn deploy_panel_from_event(
    ctx: &Context,
    api: &ApiClient,
    guild_id: &str,
    channel_id: &str,
    category: Option<&str>,
) {
    let games = match api.list_games_by_category(guild_id, category).await {
        Ok(g) => g,
        Err(e) => {
            warn!(error = %e, "Impossible de lister les jeux pour le panel web");
            return;
        }
    };

    if games.is_empty() {
        warn!("Aucun jeu dans cette categorie. Panel non deploye.");
        return;
    }

    let games_slice: Vec<&Game> = games.iter().take(MAX_BUTTONS_PER_PANEL).collect();
    let embed = build_panel_embed(category, &games_slice);

    let chan_id: serenity::model::id::ChannelId = match channel_id.parse::<u64>() {
        Ok(id) => serenity::model::id::ChannelId::new(id),
        Err(_) => return,
    };

    let msg = match chan_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Erreur envoi message panel");
            return;
        }
    };

    let _panel = match api
        .save_panel(guild_id, channel_id, &msg.id.to_string(), category)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "Panel envoye mais erreur sauvegarde");
            return;
        }
    };

    let _guild_id_obj = match guild_id.parse::<u64>() {
        Ok(id) => serenity::all::GuildId::new(id),
        Err(_) => return,
    };
    // Ajoute les reactions
    for game in &games_slice {
        if let Some(emoji_str) = &game.emoji {
            if let Some(rt) = parse_reaction_type(emoji_str) {
                if let Err(e) = msg.react(&ctx.http, rt).await {
                    tracing::warn!(error = %e, "Erreur ajout reaction pour {}", game.game_name);
                }
            }
        }
    }
}

pub async fn handle_reaction(
    api: &ApiClient,
    ctx: &Context,
    reaction: &serenity::all::Reaction,
    is_add: bool,
) {
    let guild_id = match reaction.guild_id {
        Some(g) => g,
        None => return,
    };

    // On ignore les bots
    if reaction.user_id == Some(ctx.cache.current_user().id) {
        return;
    }
    let user_id = match reaction.user_id {
        Some(id) => id,
        None => return,
    };

    // Obtenir la reaction textuelle
    let reaction_str = match &reaction.emoji {
        ReactionType::Custom { id, .. } => id.to_string(),
        ReactionType::Unicode(u) => u.clone(),
        _ => return,
    };

    let guild_id_str = guild_id.to_string();
    let message_id = reaction.message_id.to_string();
    let panel = match api.find_panel_by_message(&guild_id_str, &message_id).await {
        Ok(Some(panel)) => panel,
        Ok(None) => return,
        Err(error) => {
            warn!(%error, guild_id = %guild_id_str, %message_id, "Impossible de verifier le panel de jeux");
            return;
        }
    };

    let channel_id = reaction.channel_id.to_string();
    if panel.channel_id != channel_id {
        warn!(
            guild_id = %guild_id_str,
            %message_id,
            expected_channel_id = %panel.channel_id,
            actual_channel_id = %channel_id,
            "Panel de jeux associe a un autre salon"
        );
        return;
    }

    // Limite le catalogue aux jeux affiches par ce panel.
    let games = match api
        .list_games_by_category(&guild_id_str, panel.category.as_deref())
        .await
    {
        Ok(g) => g,

        Err(error) => {
            warn!(%error, guild_id = %guild_id_str, %message_id, "Impossible de lister les jeux du panel");
            return;
        }
    };

    let game = match find_game_for_reaction(&games, &reaction_str) {
        Some(g) => g,
        None => return,
    };

    let role_id = match game.role_id.as_deref().and_then(|s| s.parse::<u64>().ok()) {
        Some(id) => RoleId::new(id),
        None => return,
    };

    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(_) => return,
    };

    if is_add {
        let _ = member.add_role(&ctx.http, role_id).await;
    } else {
        let _ = member.remove_role(&ctx.http, role_id).await;
    }
}
