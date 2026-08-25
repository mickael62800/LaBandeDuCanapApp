use super::*;
use platform_core::nexus::ports::outbound::events::game_events;

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
    // Si c'est au format <:name:id> ou <a:name:id>
    if (s.starts_with("<:") || s.starts_with("<a:")) && s.ends_with('>') {
        let content = &s[2..s.len() - 1]; // Enlever <: et >
        let is_animated = s.starts_with("<a:");
        let content = if is_animated { &content[1..] } else { content }; // Enlever 'a'
        if let Some((name, id_str)) = content.split_once(':') {
            if let Ok(id) = id_str.parse::<u64>() {
                return Some(ReactionType::Custom {
                    animated: is_animated,
                    id: serenity::all::EmojiId::new(id),
                    name: Some(name.to_string()),
                });
            }
        }
    }
    // Serenity parse nativement les emojis custom ("name:id", "a:name:id")
    // et les emojis unicode.
    match ReactionType::try_from(s) {
        Ok(rt) => Some(rt),
        Err(_) => Some(ReactionType::Unicode(s.to_string())),
    }
}

/// Cle d'une reaction, telle qu'elle sert a retrouver le jeu correspondant.
///
/// Un emoji de SERVEUR s'identifie par son identifiant numerique, jamais par
/// son nom : renommer l'emoji ne doit pas detacher les abonnes du jeu. Un emoji
/// unicode, lui, EST sa propre cle.
///
/// `None` pour les formes que Discord peut envoyer sans qu'on sache quoi en
/// faire : mieux vaut ignorer la reaction que de la rattacher au mauvais jeu.
pub(super) fn reaction_key(emoji: &ReactionType) -> Option<String> {
    match emoji {
        ReactionType::Custom { id, .. } => Some(id.to_string()),
        ReactionType::Unicode(valeur) => Some(valeur.clone()),
        _ => None,
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
                            match event {
                                game_events::GAMES_PANEL_DEPLOY => {
                                    if let (Some(guild_id), Some(channel_id)) = (
                                        data.get("guild_id").and_then(|v| v.as_str()),
                                        data.get("channel_id").and_then(|v| v.as_str()),
                                    ) {
                                        let category =
                                            data.get("category").and_then(|v| v.as_str());
                                        deploy_panel_from_event(
                                            &ctx_clone, &api_clone, guild_id, channel_id, category,
                                        )
                                        .await;
                                    }
                                }
                                game_events::GAMES_ROLES_ENSURE => {
                                    if let Some(guild_id) =
                                        data.get("guild_id").and_then(|v| v.as_str())
                                    {
                                        ensure_roles_from_event(&ctx_clone, &api_clone, guild_id)
                                            .await;
                                    }
                                }
                                game_events::GAMES_SYNC_REQUESTED => {
                                    if let Some(guild_id) =
                                        data.get("guild_id").and_then(|v| v.as_str())
                                    {
                                        super::sync::report_inventory(
                                            &ctx_clone, &api_clone, guild_id,
                                        )
                                        .await;
                                        // Meme evenement, deux moitiés : le
                                        // rapport dit a l'API ce que le bot
                                        // voit, ce nettoyage supprime ce que
                                        // le bot voit et que la base ne
                                        // connait plus.
                                        crate::game_portal::reconcilier_salons_orphelins(
                                            &ctx_clone, &api_clone, guild_id,
                                        )
                                        .await;
                                    }
                                }
                                game_events::GAME_ROLE_DELETE => {
                                    if let (Some(guild_id), Some(role_id)) = (
                                        data.get("guild_id").and_then(|v| v.as_str()),
                                        data.get("role_id").and_then(|v| v.as_str()),
                                    ) {
                                        delete_role_from_event(&ctx_clone, guild_id, role_id).await;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            },
        )
        .await;
    });
}

async fn ensure_roles_from_event(ctx: &Context, api: &ApiClient, guild_id: &str) {
    let guild_id_obj = match guild_id.parse::<u64>() {
        Ok(id) => serenity::all::GuildId::new(id),
        Err(_) => {
            warn!(
                guild_id,
                "Guild ID invalide pour la creation des roles de jeux"
            );
            return;
        }
    };
    let mut games = match api.list_games_by_category(guild_id, None).await {
        Ok(games) => games,
        Err(error) => {
            warn!(%error, guild_id, "Impossible de lister les jeux a provisionner");
            return;
        }
    };
    ensure_game_roles(ctx, guild_id_obj, api, guild_id, &mut games).await;
}

async fn delete_role_from_event(ctx: &Context, guild_id: &str, role_id: &str) {
    let (Ok(guild_id), Ok(role_id)) = (guild_id.parse::<u64>(), role_id.parse::<u64>()) else {
        warn!(
            guild_id,
            role_id, "Identifiants invalides pour la suppression du role de jeu"
        );
        return;
    };
    if let Err(error) = serenity::all::GuildId::new(guild_id)
        .delete_role(&ctx.http, RoleId::new(role_id))
        .await
    {
        warn!(%error, guild_id, role_id, "Suppression du role de jeu impossible");
    }
}

async fn deploy_panel_from_event(
    ctx: &Context,
    api: &ApiClient,
    guild_id: &str,
    channel_id: &str,
    category: Option<&str>,
) {
    let mut games = match api.list_games_by_category(guild_id, category).await {
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

    let guild_id_obj = match guild_id.parse::<u64>() {
        Ok(id) => serenity::all::GuildId::new(id),
        Err(_) => {
            warn!(
                guild_id,
                "Guild ID invalide pour le deploiement du panel web"
            );
            return;
        }
    };

    // Parite avec `/game-admin panel` : un deploiement demande depuis le Web
    // doit lui aussi creer/reparer les roles Discord avant de rendre le panel.
    ensure_game_roles(ctx, guild_id_obj, api, guild_id, &mut games).await;

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

    let Some(reaction_str) = reaction_key(&reaction.emoji) else {
        return;
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
        Err(error) => {
            warn!(%error, %guild_id, %user_id, "Impossible de charger le membre pour la reaction de jeu");
            return;
        }
    };

    let result = if is_add {
        member.add_role(&ctx.http, role_id).await
    } else {
        member.remove_role(&ctx.http, role_id).await
    };
    if let Err(error) = result {
        warn!(
            %error,
            %guild_id,
            %user_id,
            game = %game.game_name,
            %role_id,
            action = if is_add { "add" } else { "remove" },
            "Modification du role de jeu impossible"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_game_for_reaction() {
        let games = vec![
            Game {
                id: "g1".into(),
                game_name: "Minecraft".into(),
                role_id: Some("111".into()),
                emoji: Some("⛏️".into()),
                category: None,
            },
            Game {
                id: "g2".into(),
                game_name: "CustomGame".into(),
                role_id: Some("222".into()),
                emoji: Some("<:custom:333333>".into()),
                category: None,
            },
            Game {
                id: "g3".into(),
                game_name: "NoEmoji".into(),
                role_id: None,
                emoji: None,
                category: None,
            },
        ];

        let found_unicode = find_game_for_reaction(&games, "⛏️");
        assert_eq!(
            found_unicode.map(|g| g.game_name.as_str()),
            Some("Minecraft")
        );

        let found_custom = find_game_for_reaction(&games, "333333");
        assert_eq!(
            found_custom.map(|g| g.game_name.as_str()),
            Some("CustomGame")
        );

        let not_found = find_game_for_reaction(&games, "🎮");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_parse_reaction_type_cases() {
        assert!(parse_reaction_type("").is_none());
        assert!(parse_reaction_type("   ").is_none());

        let animated = parse_reaction_type("<a:fire:99999>").unwrap();
        match animated {
            ReactionType::Custom { animated, id, name } => {
                assert!(animated);
                assert_eq!(id.get(), 99999);
                assert_eq!(name.as_deref(), Some("fire"));
            }
            _ => panic!("Expected animated custom emoji"),
        }
    }

    fn jeu(nom: &str, emoji: Option<&str>) -> Game {
        Game {
            id: format!("id-{nom}"),
            game_name: nom.into(),
            emoji: emoji.map(str::to_string),
            category: None,
            role_id: None,
        }
    }

    #[test]
    fn un_emoji_de_serveur_est_identifie_par_son_numero() {
        // Renommer l'emoji ne doit pas detacher les abonnes du jeu : c'est
        // l'identifiant qui fait foi, jamais le nom.
        let custom = ReactionType::Custom {
            animated: false,
            id: serenity::all::EmojiId::new(987654321),
            name: Some("minecraft".into()),
        };
        assert_eq!(reaction_key(&custom).as_deref(), Some("987654321"));

        // Meme identifiant sous un autre nom : meme cle.
        let renomme = ReactionType::Custom {
            animated: true,
            id: serenity::all::EmojiId::new(987654321),
            name: Some("mc_nouveau_nom".into()),
        };
        assert_eq!(reaction_key(&renomme), reaction_key(&custom));
    }

    #[test]
    fn un_emoji_unicode_est_sa_propre_cle() {
        let uni = ReactionType::Unicode("\u{1f3ae}".into());
        assert_eq!(reaction_key(&uni).as_deref(), Some("\u{1f3ae}"));
    }

    #[test]
    fn une_reaction_inconnue_ne_rattache_aucun_jeu() {
        // Fail closed : rattacher au hasard donnerait un role qui n'a rien a
        // voir avec ce sur quoi la personne a clique.
        let jeux = vec![jeu("Minecraft", Some("\u{26cf}"))];
        assert!(find_game_for_reaction(&jeux, "\u{274c}").is_none());
        assert!(find_game_for_reaction(&jeux, "").is_none());
        assert!(find_game_for_reaction(&[], "\u{26cf}").is_none());
    }

    #[test]
    fn un_jeu_sans_emoji_n_est_jamais_retrouve() {
        let jeux = vec![jeu("SansEmoji", None)];
        assert!(find_game_for_reaction(&jeux, "").is_none());
        assert!(find_game_for_reaction(&jeux, "\u{26cf}").is_none());
    }

    #[test]
    fn un_emoji_vide_en_configuration_repond_a_une_reaction_vide() {
        // Constat, pas souhait : la comparaison est une egalite de chaines, et
        // `"" == ""` est vrai. En pratique Discord n'envoie jamais de reaction
        // vide, et le panneau ne pose aucune reaction pour un tel jeu — il
        // resterait donc inatteignable. Le test fige le comportement plutot que
        // de laisser croire a une garde qui n'existe pas.
        let jeux = vec![jeu("Vide", Some(""))];
        assert_eq!(
            find_game_for_reaction(&jeux, "").map(|g| g.game_name.as_str()),
            Some("Vide")
        );
        assert!(find_game_for_reaction(&jeux, "\u{26cf}").is_none());
    }

    #[test]
    fn le_premier_jeu_qui_correspond_l_emporte() {
        // Deux jeux ne devraient pas partager un emoji — le panneau les
        // deduplique — mais si la configuration l'a laisse passer, le resultat
        // doit rester previsible plutot qu'arbitraire.
        let jeux = vec![
            jeu("Premier", Some("\u{1f3ae}")),
            jeu("Second", Some("\u{1f3ae}")),
        ];
        assert_eq!(
            find_game_for_reaction(&jeux, "\u{1f3ae}").map(|g| g.game_name.as_str()),
            Some("Premier")
        );
    }

    #[test]
    fn une_saisie_vide_ne_donne_aucune_reaction() {
        assert!(parse_reaction_type("").is_none());
        assert!(parse_reaction_type("   ").is_none());
    }

    #[test]
    fn test_parse_reaction_type_custom_emoji() {
        let custom = parse_reaction_type("<:smiling_face:123456>").unwrap();
        match custom {
            ReactionType::Custom { animated, id, name } => {
                assert!(!animated);
                assert_eq!(id.get(), 123456);
                assert_eq!(name.as_deref(), Some("smiling_face"));
            }
            _ => panic!("Expected custom emoji"),
        }
    }

    #[test]
    fn test_parse_reaction_type_animated_emoji() {
        let animated = parse_reaction_type("<a:star:654321>").unwrap();
        match animated {
            ReactionType::Custom { animated, id, name } => {
                assert!(animated);
                assert_eq!(id.get(), 654321);
                assert_eq!(name.as_deref(), Some("star"));
            }
            _ => panic!("Expected animated custom emoji"),
        }
    }

    #[test]
    fn test_parse_reaction_type_unicode_emoji() {
        let unicode = parse_reaction_type("😀").unwrap();
        match unicode {
            ReactionType::Unicode(val) => {
                assert_eq!(val, "😀");
            }
            _ => panic!("Expected unicode emoji"),
        }
    }

    #[test]
    fn test_parse_reaction_type_invalid_custom_format() {
        // Malformed custom emoji formats fall back to unicode parsing
        let result = parse_reaction_type("<:invalid");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_game_for_reaction_with_custom_emoji() {
        let games = vec![Game {
            id: "g1".into(),
            game_name: "Test".into(),
            role_id: None,
            emoji: Some("<:custom:111111>".into()),
            category: None,
        }];

        // Should match by ID
        let found = find_game_for_reaction(&games, "111111");
        assert_eq!(found.map(|g| g.game_name.as_str()), Some("Test"));
    }

    #[test]
    fn test_find_game_for_reaction_no_emoji() {
        let games = vec![Game {
            id: "g1".into(),
            game_name: "NoEmoji".into(),
            role_id: None,
            emoji: None,
            category: None,
        }];

        assert!(find_game_for_reaction(&games, "🎮").is_none());
    }

    #[test]
    fn test_parse_reaction_type_whitespace_trimming() {
        // Extra whitespace should be trimmed
        let result = parse_reaction_type("  😀  ");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_reaction_type_custom_with_spaces_in_name() {
        // Names with no spaces should parse fine
        let custom = parse_reaction_type("<:name_test:123>").unwrap();
        match custom {
            ReactionType::Custom { .. } => {}
            _ => panic!("Should be custom"),
        }
    }

    #[test]
    fn test_parse_reaction_type_malformed_custom() {
        // Malformed custom emoji formats
        assert!(parse_reaction_type("<:no_closing").is_some());
        assert!(parse_reaction_type("<:>").is_some());
        assert!(parse_reaction_type("<:name:").is_some());
    }

    #[test]
    fn test_parse_reaction_type_emoji_with_variant_selector() {
        let result = parse_reaction_type("👍\u{fe0f}");
        assert!(result.is_some());
    }

    #[test]
    fn test_reaction_key_custom_emoji_different_names() {
        let emoji1 = ReactionType::Custom {
            animated: false,
            id: serenity::all::EmojiId::new(123),
            name: Some("name1".into()),
        };
        let emoji2 = ReactionType::Custom {
            animated: false,
            id: serenity::all::EmojiId::new(123),
            name: Some("name2".into()),
        };
        // Same ID = same key, regardless of name
        assert_eq!(reaction_key(&emoji1), reaction_key(&emoji2));
    }

    #[test]
    fn test_find_game_for_reaction_multiple_custom_emojis() {
        let games = vec![
            Game {
                id: "1".into(),
                game_name: "Game1".into(),
                emoji: Some("<:emoji1:111>".into()),
                category: None,
                role_id: None,
            },
            Game {
                id: "2".into(),
                game_name: "Game2".into(),
                emoji: Some("<:emoji2:222>".into()),
                category: None,
                role_id: None,
            },
        ];

        assert_eq!(
            find_game_for_reaction(&games, "111").map(|g| g.game_name.as_str()),
            Some("Game1")
        );
        assert_eq!(
            find_game_for_reaction(&games, "222").map(|g| g.game_name.as_str()),
            Some("Game2")
        );
    }

    #[test]
    fn test_parse_reaction_type_emoji_sequences() {
        // Multiple emoji in sequence
        let result = parse_reaction_type("👨‍👩‍👧");
        assert!(result.is_some());
    }

    #[test]
    fn test_reaction_key_consistency() {
        let emoji1 = ReactionType::Unicode("🎮".into());
        let emoji2 = ReactionType::Unicode("🎮".into());
        assert_eq!(reaction_key(&emoji1), reaction_key(&emoji2));
    }

    #[test]
    fn test_find_game_for_reaction_case_sensitivity() {
        let games = vec![Game {
            id: "1".into(),
            game_name: "Test".into(),
            emoji: Some("<:EMOJI:123>".into()),
            category: None,
            role_id: None,
        }];

        // Emoji comparison should handle ID matching
        assert_eq!(
            find_game_for_reaction(&games, "123").map(|g| g.game_name.as_str()),
            Some("Test")
        );
    }

    #[test]
    fn test_parse_reaction_type_flag_emoji() {
        // Flag emojis (two regional indicator symbols)
        let result = parse_reaction_type("🇫🇷");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_reaction_type_animated_name_parsing() {
        let animated = parse_reaction_type("<a:anim_name:555>").unwrap();
        match animated {
            ReactionType::Custom {
                animated: is_anim,
                id,
                name,
            } => {
                assert!(is_anim);
                assert_eq!(id.get(), 555);
                assert_eq!(name.as_deref(), Some("anim_name"));
            }
            _ => panic!("Expected animated custom"),
        }
    }

    #[test]
    fn test_find_game_for_reaction_first_match_wins() {
        let games = vec![
            Game {
                id: "1".into(),
                game_name: "First".into(),
                emoji: Some("🎮".into()),
                category: None,
                role_id: None,
            },
            Game {
                id: "2".into(),
                game_name: "Second".into(),
                emoji: Some("🎮".into()),
                category: None,
                role_id: None,
            },
        ];

        let found = find_game_for_reaction(&games, "🎮");
        assert_eq!(found.map(|g| g.game_name.as_str()), Some("First"));
    }

    #[test]
    fn test_parse_reaction_type_long_custom_id() {
        let custom = parse_reaction_type("<:name:999999999999>").unwrap();
        match custom {
            ReactionType::Custom { id, .. } => {
                assert_eq!(id.get(), 999999999999);
            }
            _ => panic!("Expected custom"),
        }
    }

    #[test]
    fn test_reaction_key_unicode_emoji_variants() {
        let e1 = ReactionType::Unicode("❤️".into());
        let e2 = ReactionType::Unicode("❤".into());
        // Different variants should have different keys
        let k1 = reaction_key(&e1);
        let k2 = reaction_key(&e2);
        assert_eq!(k1, Some("❤️".into()));
        assert_eq!(k2, Some("❤".into()));
    }

    #[test]
    fn test_find_game_for_reaction_exact_match() {
        let games = vec![Game {
            id: "1".into(),
            game_name: "Game".into(),
            emoji: Some("<:exact:123>".into()),
            category: None,
            role_id: None,
        }];

        assert!(find_game_for_reaction(&games, "123").is_some());
        assert!(find_game_for_reaction(&games, "124").is_none());
    }

    #[test]
    fn test_parse_reaction_type_max_emoji_id() {
        let custom = parse_reaction_type("<:name:18446744073709551615>").unwrap();
        match custom {
            ReactionType::Custom { id, .. } => {
                // Should parse the large number
                assert_eq!(id.get(), 18446744073709551615);
            }
            _ => panic!("Expected custom"),
        }
    }

    #[test]
    fn test_reaction_key_empty_unicode_string() {
        // Empty string should still be a valid key
        let e = ReactionType::Unicode("".into());
        assert_eq!(reaction_key(&e), Some("".into()));
    }

    #[test]
    fn test_find_game_for_reaction_whitespace_handling() {
        let games = vec![Game {
            id: "1".into(),
            game_name: "Game".into(),
            emoji: Some("  🎮  ".into()),
            category: None,
            role_id: None,
        }];

        // The emoji field has spaces, but parse_reaction_type trims them
        let found = find_game_for_reaction(&games, "🎮");
        assert!(found.is_some());
    }

    #[test]
    fn test_parse_reaction_type_numbers_as_emoji() {
        let result = parse_reaction_type("123");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_game_for_reaction_empty_games_list() {
        let found = find_game_for_reaction(&[], "🎮");
        assert!(found.is_none());
    }

    #[test]
    fn test_parse_reaction_type_custom_no_id() {
        let result = parse_reaction_type("<:name:notanumber>");
        // Should still return something (fallback to unicode)
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_reaction_type_colon_variations() {
        // Various colon formats
        assert!(parse_reaction_type(":").is_some());
        assert!(parse_reaction_type("::").is_some());
        assert!(parse_reaction_type(":::").is_some());
    }

    #[test]
    fn test_parse_reaction_type_brackets_only() {
        assert!(parse_reaction_type("<>").is_some());
        assert!(parse_reaction_type("<:>").is_some());
    }

    #[test]
    fn test_reaction_key_custom_multiple_ids() {
        for id in 1..=5u64 {
            let emoji = ReactionType::Custom {
                animated: false,
                id: serenity::all::EmojiId::new(id),
                name: Some("test".into()),
            };
            let key = reaction_key(&emoji);
            assert_eq!(key, Some(id.to_string()));
        }
    }

    #[test]
    fn test_find_game_for_reaction_partial_emoji_match() {
        let games = vec![Game {
            id: "1".into(),
            game_name: "Game".into(),
            emoji: Some("<:test:123>".into()),
            category: None,
            role_id: None,
        }];

        // Should match by custom ID
        assert!(find_game_for_reaction(&games, "123").is_some());
        // Should not match partial ID
        assert!(find_game_for_reaction(&games, "12").is_none());
    }

    #[test]
    fn test_parse_reaction_type_special_unicode() {
        let special_chars = vec!["🔥", "💯", "⚡", "👑", "🎯"];
        for char in special_chars {
            let result = parse_reaction_type(char);
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_reaction_key_multiple_unicode() {
        let emojis = vec!["🎮", "⚔️", "🏰", "🎯"];
        for emoji_str in emojis {
            let emoji = ReactionType::Unicode(emoji_str.into());
            assert_eq!(reaction_key(&emoji), Some(emoji_str.into()));
        }
    }

    #[test]
    fn test_find_game_for_reaction_all_game_types() {
        let games = vec![
            Game {
                id: "1".into(),
                game_name: "Unicode".into(),
                emoji: Some("🎮".into()),
                category: None,
                role_id: None,
            },
            Game {
                id: "2".into(),
                game_name: "Custom".into(),
                emoji: Some("<:custom:999>".into()),
                category: None,
                role_id: None,
            },
            Game {
                id: "3".into(),
                game_name: "Animated".into(),
                emoji: Some("<a:anim:888>".into()),
                category: None,
                role_id: None,
            },
        ];

        // Test all three types
        assert_eq!(
            find_game_for_reaction(&games, "🎮").map(|g| g.game_name.as_str()),
            Some("Unicode")
        );
        assert_eq!(
            find_game_for_reaction(&games, "999").map(|g| g.game_name.as_str()),
            Some("Custom")
        );
        assert_eq!(
            find_game_for_reaction(&games, "888").map(|g| g.game_name.as_str()),
            Some("Animated")
        );
    }

    #[test]
    fn test_parse_reaction_type_very_long_emoji() {
        let long = "🎮".repeat(100);
        let result = parse_reaction_type(&long);
        assert!(result.is_some());
    }

    #[test]
    fn test_reaction_key_boundary_values() {
        // Test boundary ID values (skip 0 as EmojiId::new doesn't accept it)
        for id in [1u64, 100, 1000, u64::MAX] {
            let emoji = ReactionType::Custom {
                animated: false,
                id: serenity::all::EmojiId::new(id),
                name: Some("test".into()),
            };
            let key = reaction_key(&emoji);
            assert_eq!(key, Some(id.to_string()));
        }
    }

    #[test]
    fn test_find_game_for_reaction_same_emoji_different_games() {
        let games = vec![
            Game {
                id: "1".into(),
                game_name: "First".into(),
                emoji: Some("🎮".into()),
                category: None,
                role_id: None,
            },
            Game {
                id: "2".into(),
                game_name: "Second".into(),
                emoji: Some("🎮".into()),
                category: None,
                role_id: None,
            },
        ];

        // Should return first match
        let found = find_game_for_reaction(&games, "🎮");
        assert_eq!(found.map(|g| g.game_name.as_str()), Some("First"));
    }
}
