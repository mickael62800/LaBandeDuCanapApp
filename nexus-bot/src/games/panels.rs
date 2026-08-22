use super::*;

// ── Panels ──

pub(super) async fn handle_panel(
    ctx: &Context,
    cmd: &CommandInteraction,
    api: &ApiClient,
    guild_id: &str,
) {
    if !has_manage_guild(cmd) {
        reply(ctx, cmd, "Permission **Gerer le serveur** requise.").await;
        return;
    }

    // Le deploiement peut enchainer plusieurs appels API et jusqu'a 25 ajouts
    // de reactions Discord. Acquitter l'interaction avant ces appels evite que
    // Discord l'expire au bout de trois secondes.
    if let Err(e) = cmd.defer_ephemeral(&ctx.http).await {
        warn!(error = %e, "Defer /game-admin panel impossible");
        return;
    }

    let category = get_string_option(cmd, "category");
    let mut games = match api
        .list_games_by_category(guild_id, category.as_deref())
        .await
    {
        Ok(g) => g,
        Err(e) => {
            edit_deferred_reply(ctx, cmd, &format!("Erreur : {e}")).await;
            return;
        }
    };

    if games.is_empty() {
        edit_deferred_reply(
            ctx,
            cmd,
            "Aucun jeu dans cette categorie. Ajoute-en avec `/game-admin create`.",
        )
        .await;
        return;
    }

    // Backfill : cree un role Discord pour les jeux qui n'en ont pas encore,
    // sinon leurs boutons ne pourraient donner aucun role.
    if let Some(gid) = cmd.guild_id {
        ensure_game_roles(ctx, gid, api, guild_id, &mut games).await;
    }

    let games_slice: Vec<&Game> = games.iter().take(MAX_BUTTONS_PER_PANEL).collect();
    if games.len() > MAX_BUTTONS_PER_PANEL {
        warn!(
            total = games.len(),
            shown = MAX_BUTTONS_PER_PANEL,
            "Panel tronque : trop de jeux pour un seul message (max 25 boutons)"
        );
    }

    let embed = build_panel_embed(category.as_deref(), &games_slice);

    // 1) Envoie un message initial avec l'embed seulement (pas encore de components).
    let mut msg = match cmd
        .channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        Ok(m) => m,
        Err(e) => {
            edit_deferred_reply(ctx, cmd, &format!("Erreur envoi message : {e}")).await;
            return;
        }
    };

    // 2) Sauve le panel en DB. On a besoin de son id pour les custom_id des
    //    boutons, d'ou l'ordre : message d'abord, sauvegarde, puis boutons.
    let _panel = match api
        .save_panel(
            guild_id,
            &msg.channel_id.to_string(),
            &msg.id.to_string(),
            category.as_deref(),
        )
        .await
    {
        Ok(p) => p,
        Err(e) => {
            edit_deferred_reply(
                ctx,
                cmd,
                &format!("Panel envoye mais erreur de sauvegarde : {e}"),
            )
            .await;
            return;
        }
    };

    // 3) Attache les reactions natives. Discord affiche alors automatiquement
    //    le compteur et surligne la reaction choisie pour chaque membre.
    if let Err(e) = edit_panel_with_reactions(ctx, &mut msg, &games_slice, None).await {
        edit_deferred_reply(
            ctx,
            cmd,
            &format!("Panel envoye mais erreur d'ajout des boutons : {e}"),
        )
        .await;
        return;
    }

    edit_deferred_reply(ctx, cmd, &format_panel_deployed_message(games_slice.len())).await;
}

pub(super) async fn handle_refresh(
    ctx: &Context,
    cmd: &CommandInteraction,
    api: &ApiClient,
    guild_id: &str,
) {
    if !has_manage_guild(cmd) {
        reply(ctx, cmd, "Permission **Gerer le serveur** requise.").await;
        return;
    }

    // Une edition suivie de nombreuses reactions depasse facilement la
    // fenetre de reponse Discord si la commande n'est pas differee.
    if let Err(e) = cmd.defer_ephemeral(&ctx.http).await {
        warn!(error = %e, "Defer /game-admin refresh impossible");
        return;
    }

    let category = get_string_option(cmd, "category");

    // Trouve le panel existant via list_panels.
    let panels = match api.list_panels(guild_id).await {
        Ok(p) => p,
        Err(e) => {
            edit_deferred_reply(ctx, cmd, &format!("Erreur : {e}")).await;
            return;
        }
    };
    let cat_norm = category.as_deref().map(str::to_lowercase);
    let panel = panels
        .into_iter()
        .find(|p| p.category.as_deref().map(str::to_lowercase) == cat_norm);
    let panel = match panel {
        Some(p) => p,
        None => {
            edit_deferred_reply(
                ctx,
                cmd,
                "Aucun panneau existant pour cette categorie. Utilise `/game-admin panel` d'abord.",
            )
            .await;
            return;
        }
    };

    let mut games = match api
        .list_games_by_category(guild_id, category.as_deref())
        .await
    {
        Ok(g) => g,
        Err(e) => {
            edit_deferred_reply(ctx, cmd, &format!("Erreur : {e}")).await;
            return;
        }
    };
    // Backfill des roles manquants (idem deploiement).
    if let Some(gid) = cmd.guild_id {
        ensure_game_roles(ctx, gid, api, guild_id, &mut games).await;
    }
    let games_slice: Vec<&Game> = games.iter().take(MAX_BUTTONS_PER_PANEL).collect();

    let embed = build_panel_embed(category.as_deref(), &games_slice);
    let _gid = cmd.guild_id.unwrap_or_default();

    let channel_id: serenity::model::id::ChannelId = match panel.channel_id.parse::<u64>() {
        Ok(id) => serenity::model::id::ChannelId::new(id),
        Err(_) => {
            edit_deferred_reply(ctx, cmd, "channel_id invalide en DB.").await;
            return;
        }
    };
    let message_id: serenity::model::id::MessageId = match panel.message_id.parse::<u64>() {
        Ok(id) => serenity::model::id::MessageId::new(id),
        Err(_) => {
            edit_deferred_reply(ctx, cmd, "message_id invalide en DB.").await;
            return;
        }
    };

    let mut msg = match channel_id.message(&ctx.http, message_id).await {
        Ok(m) => m,
        Err(e) => {
            edit_deferred_reply(ctx, cmd, &format!("Message panneau introuvable : {e}")).await;
            return;
        }
    };

    // Repasse aussi les anciens panneaux à la sélection par réactions : le
    // compteur et l'état sélectionné sont gérés nativement par Discord.
    if let Err(e) = edit_panel_with_reactions(ctx, &mut msg, &games_slice, Some(embed)).await {
        edit_deferred_reply(ctx, cmd, &format!("Erreur edition : {e}")).await;
        return;
    }

    edit_deferred_reply(ctx, cmd, &format_panel_refresh_message(games_slice.len())).await;
}

/// Convertit/actualise un panneau en sélecteur par réactions natives.
///
/// Les composants sont retirés afin d'éviter deux moyens de sélection
/// concurrents. Ajouter une réaction déjà présente est idempotent côté Discord.
async fn edit_panel_with_reactions(
    ctx: &Context,
    msg: &mut serenity::model::channel::Message,
    games: &[&Game],
    embed: Option<CreateEmbed>,
) -> serenity::Result<()> {
    let mut edit = EditMessage::new().components(Vec::new());
    if let Some(embed) = embed {
        edit = edit.embed(embed);
    }
    msg.edit(&ctx.http, edit).await?;

    for reaction in panel_reactions(games) {
        msg.react(&ctx.http, reaction).await?;
    }
    Ok(())
}

/// Liste dédupliquée des réactions valides d'un panneau.
pub(super) fn panel_reactions(games: &[&Game]) -> Vec<ReactionType> {
    let mut seen = HashSet::new();
    games
        .iter()
        .filter_map(|game| game.emoji.as_deref().and_then(parse_reaction_type))
        .filter(|reaction| seen.insert(reaction.to_string()))
        .collect()
}

pub(crate) fn build_panel_embed(category: Option<&str>, games: &[&Game]) -> CreateEmbed {
    let title = match category {
        Some(c) => format!("- [ {} ] -", c),
        None => "- [ Jeux ] -".to_string(),
    };
    let desc = if games.is_empty() {
        "*Aucun jeu.*".to_string()
    } else {
        let mut lines = Vec::with_capacity(games.len());
        for g in games.iter() {
            let emoji = g.emoji.clone().unwrap_or_default();
            let prefix = if emoji.is_empty() {
                String::new()
            } else {
                format!("{} ", emoji)
            };
            // Mention du role quand il existe : le jeu s'affiche alors comme une
            // pastille coloree cliquable, et non plus un simple nom. Les jeux
            // legacy sans role_id retombent sur leur nom en gras.
            let label = match g.role_id.as_deref().and_then(|s| s.parse::<u64>().ok()) {
                Some(rid) => format!("<@&{}>", rid),
                None => format!("**{}**", g.game_name),
            };
            lines.push(format!("{}{}", prefix, label));
        }
        let mut s = lines.join("\n");
        s.push_str(
            "\n\n*Clique sur une réaction ci-dessous pour rejoindre ou quitter un jeu et recevoir (ou non) ses notifications.*",
        );
        s
    };
    info_embed(&title).description(desc)
}

/// Crée un rôle Discord pour chaque jeu qui n'en a pas encore et persiste
/// l'association côté API. Rend le panneau auto-réparateur : les jeux legacy
/// (créés avant le support des rôles) reçoivent leur rôle au déploiement —
/// sans quoi leurs boutons ne pourraient donner aucun rôle à mentionner.
///
/// Best-effort : un jeu dont le rôle n'a pas pu être créé/persisté est laissé
/// tel quel (son bouton dira « pas de rôle »), on réessaiera au prochain
/// déploiement. Nécessite la permission Discord **Gérer les rôles** pour le bot.
pub(crate) async fn ensure_game_roles(
    ctx: &Context,
    guild: GuildId,
    api: &ApiClient,
    guild_id: &str,
    games: &mut [Game],
) {
    // Une liaison non vide peut pointer vers un role supprime manuellement.
    // Charger les roles une fois permet de reparer ces liaisons sans faire un
    // appel Discord par jeu. Si Discord est indisponible, on fait confiance a
    // la DB pour ne pas creer de doublons a l'aveugle.
    let mut guild_roles = match guild.roles(&ctx.http).await {
        Ok(roles) => Some(roles),
        Err(e) => {
            warn!(error = %e, guild = %guild_id, "Verification des roles de jeux impossible");
            None
        }
    };

    for game in games.iter_mut() {
        let stored_role = game
            .role_id
            .as_deref()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .map(RoleId::new);
        let role_is_valid = is_stored_role_valid(guild_roles.as_ref(), stored_role);
        if role_is_valid {
            continue;
        }
        if let Some(role_id) = stored_role {
            warn!(game = %game.game_name, role = %role_id, "Role de jeu obsolete, recreation");
        }
        // Ne jamais rendre le panneau avec une mention vers un role supprime
        // ou un identifiant invalide si la recreation echoue ensuite.
        game.role_id = None;

        let role = match guild
            .create_role(
                &ctx.http,
                EditRole::new()
                    .name(&game.game_name)
                    .colour(Colour::new(ROLE_COLOR))
                    .mentionable(true)
                    .hoist(false),
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, game = %game.game_name, "Creation du role de jeu impossible (permission Gerer les roles ?)");
                continue;
            }
        };
        let role_id_str = role.id.get().to_string();
        match api.set_game_role(guild_id, &game.id, &role_id_str).await {
            Ok(updated) => {
                game.role_id = updated.role_id.or(Some(role_id_str));
                if let Some(roles) = &mut guild_roles {
                    roles.insert(role.id, role);
                }
                info!(game = %game.game_name, role = ?game.role_id, "Role de jeu cree et associe");
            }
            Err(e) => {
                // Rollback : role cree mais non persiste -> on le supprime pour
                // ne pas laisser un role orphelin. Reessai au prochain deploiement.
                warn!(error = %e, game = %game.game_name, "Persistance du role de jeu impossible, rollback");
                let _ = guild.delete_role(&ctx.http, role.id).await;
            }
        }
    }
}

pub fn is_stored_role_valid(
    guild_roles: Option<&std::collections::HashMap<RoleId, serenity::model::guild::Role>>,
    stored_role: Option<RoleId>,
) -> bool {
    match (guild_roles, stored_role) {
        (Some(roles), Some(role_id)) => roles.contains_key(&role_id),
        (None, Some(_)) => true,
        _ => false,
    }
}

pub fn format_panel_deployed_message(game_count: usize) -> String {
    format!("Panneau deploye ({} jeux).", game_count)
}

pub fn format_panel_refresh_message(game_count: usize) -> String {
    format!("Panneau rafraichi ({} jeux).", game_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_helpers() {
        assert_eq!(
            format_panel_deployed_message(5),
            "Panneau deploye (5 jeux)."
        );
        assert_eq!(
            format_panel_refresh_message(3),
            "Panneau rafraichi (3 jeux)."
        );

        assert!(is_stored_role_valid(None, Some(RoleId::new(1))));
        assert!(!is_stored_role_valid(None, None));

        let roles_map = std::collections::HashMap::new();
        assert!(!is_stored_role_valid(
            Some(&roles_map),
            Some(RoleId::new(1))
        ));
        assert!(!is_stored_role_valid(Some(&roles_map), None));
    }

    fn jeu(nom: &str, emoji: Option<&str>, role: Option<&str>) -> Game {
        Game {
            id: format!("id-{nom}"),
            game_name: nom.into(),
            emoji: emoji.map(str::to_string),
            category: None,
            role_id: role.map(str::to_string),
        }
    }

    #[test]
    fn le_titre_du_panneau_porte_la_categorie() {
        let vide: Vec<&Game> = vec![];
        let json = serde_json::to_value(build_panel_embed(Some("Survie"), &vide)).unwrap();
        assert_eq!(json["title"], "- [ Survie ] -");

        // Sans categorie, un titre generique plutot qu'un titre vide.
        let json = serde_json::to_value(build_panel_embed(None, &vide)).unwrap();
        assert_eq!(json["title"], "- [ Jeux ] -");
    }

    #[test]
    fn un_panneau_sans_jeu_le_dit_au_lieu_d_etre_muet() {
        let vide: Vec<&Game> = vec![];
        let json = serde_json::to_value(build_panel_embed(None, &vide)).unwrap();
        assert_eq!(json["description"], "*Aucun jeu.*");
    }

    #[test]
    fn un_jeu_avec_role_s_affiche_en_pastille_cliquable() {
        // La mention rend le jeu cliquable et colore ; le nom en gras, non.
        // C'est la seule difference visible entre un jeu configure et un jeu
        // legacy reste sans role.
        let g = jeu("Minecraft", Some("\u{26cf}"), Some("12345"));
        let json = serde_json::to_value(build_panel_embed(None, &[&g])).unwrap();
        let desc = json["description"].as_str().unwrap();
        assert!(desc.contains("<@&12345>"));
        assert!(!desc.contains("**Minecraft**"));
        assert!(desc.contains("\u{26cf}"));
    }

    #[test]
    fn un_jeu_legacy_sans_role_retombe_sur_son_nom() {
        let g = jeu("Valheim", None, None);
        let json = serde_json::to_value(build_panel_embed(None, &[&g])).unwrap();
        let desc = json["description"].as_str().unwrap();
        assert!(desc.contains("**Valheim**"));
        assert!(!desc.contains("<@&"));
    }

    #[test]
    fn un_role_illisible_vaut_un_jeu_sans_role() {
        // Fail closed : mieux vaut le nom en clair qu'une mention forgee a
        // partir d'un identifiant qu'on n'a pas su lire.
        let g = jeu("Terraria", None, Some("pas_un_nombre"));
        let json = serde_json::to_value(build_panel_embed(None, &[&g])).unwrap();
        let desc = json["description"].as_str().unwrap();
        assert!(desc.contains("**Terraria**"));
        assert!(!desc.contains("<@&"));
    }

    #[test]
    fn le_panneau_explique_toujours_comment_s_en_servir() {
        // Sans cette phrase, les reactions sous le panneau n'ont aucun sens
        // pour qui decouvre le salon.
        let g = jeu("Ark", None, None);
        let json = serde_json::to_value(build_panel_embed(None, &[&g])).unwrap();
        assert!(json["description"]
            .as_str()
            .unwrap()
            .contains("Clique sur une réaction"));
    }

    #[test]
    fn les_reactions_du_panneau_sont_dedupliquees() {
        // Discord refuse deux fois la meme reaction sur un message : deux jeux
        // qui partagent un emoji ne doivent en poser qu'une.
        let a = jeu("A", Some("\u{1f3ae}"), None);
        let b = jeu("B", Some("\u{1f3ae}"), None);
        let c = jeu("C", Some("\u{1f9df}"), None);
        assert_eq!(panel_reactions(&[&a, &b, &c]).len(), 2);
    }

    #[test]
    fn un_jeu_sans_emoji_ne_pose_aucune_reaction() {
        let a = jeu("A", None, None);
        let b = jeu("B", Some(""), None);
        assert!(panel_reactions(&[&a, &b]).is_empty());
    }

    #[test]
    fn un_role_stocke_est_valide_tant_qu_on_ne_peut_pas_prouver_le_contraire() {
        // Sans la liste des roles de la guilde, on ne conclut pas a une
        // disparition : recreer un role qui existe deja en ferait deux.
        assert!(is_stored_role_valid(None, Some(RoleId::new(1))));

        // Avec la liste, l'absence est une preuve.
        let mut roles = std::collections::HashMap::new();
        assert!(!is_stored_role_valid(Some(&roles), Some(RoleId::new(1))));

        // Et la presence aussi.
        let role = serenity::model::guild::Role::default();
        roles.insert(RoleId::new(1), role);
        assert!(is_stored_role_valid(Some(&roles), Some(RoleId::new(1))));

        // Aucun role stocke : il n'y a rien a valider.
        assert!(!is_stored_role_valid(Some(&roles), None));
        assert!(!is_stored_role_valid(None, None));
    }

    #[test]
    fn test_build_panel_embed_with_category() {
        let g = jeu("Game1", Some("🎮"), Some("123"));
        let embed = build_panel_embed(Some("Action"), &[&g]);
        let json = serde_json::to_value(embed).unwrap();
        assert_eq!(json["title"], "- [ Action ] -");
    }

    #[test]
    fn test_build_panel_embed_no_category() {
        let g = jeu("Game1", Some("🎮"), None);
        let embed = build_panel_embed(None, &[&g]);
        let json = serde_json::to_value(embed).unwrap();
        assert_eq!(json["title"], "- [ Jeux ] -");
    }

    #[test]
    fn test_build_panel_embed_multiple_games() {
        let g1 = jeu("Game1", Some("🎮"), Some("1"));
        let g2 = jeu("Game2", Some("🎯"), Some("2"));
        let embed = build_panel_embed(None, &[&g1, &g2]);
        let json = serde_json::to_value(embed).unwrap();
        let desc = json["description"].as_str().unwrap();
        assert!(desc.contains("<@&1>"));
        assert!(desc.contains("<@&2>"));
    }

    #[test]
    fn test_build_panel_embed_game_without_emoji() {
        let g = jeu("Game1", None, Some("123"));
        let embed = build_panel_embed(None, &[&g]);
        let json = serde_json::to_value(embed).unwrap();
        let desc = json["description"].as_str().unwrap();
        assert!(desc.contains("<@&123>"));
        assert!(!desc.contains("🎮"));
    }

    #[test]
    fn test_panel_reactions_deduplicates_emojis() {
        let g1 = jeu("Game1", Some("🎮"), None);
        let g2 = jeu("Game2", Some("🎮"), None);
        let g3 = jeu("Game3", Some("🎯"), None);
        let reactions = panel_reactions(&[&g1, &g2, &g3]);
        // Should have 2 unique reactions
        assert_eq!(reactions.len(), 2);
    }

    #[test]
    fn test_panel_reactions_empty_games() {
        let reactions = panel_reactions(&[]);
        assert!(reactions.is_empty());
    }

    #[test]
    fn test_format_panel_deployed_message_single_game() {
        let msg = format_panel_deployed_message(1);
        assert_eq!(msg, "Panneau deploye (1 jeux).");
    }

    #[test]
    fn test_format_panel_deployed_message_multiple_games() {
        let msg = format_panel_deployed_message(10);
        assert_eq!(msg, "Panneau deploye (10 jeux).");
    }

    #[test]
    fn test_format_panel_refresh_message_single_game() {
        let msg = format_panel_refresh_message(1);
        assert_eq!(msg, "Panneau rafraichi (1 jeux).");
    }

    #[test]
    fn test_format_panel_refresh_message_multiple_games() {
        let msg = format_panel_refresh_message(5);
        assert_eq!(msg, "Panneau rafraichi (5 jeux).");
    }

    #[test]
    fn test_is_stored_role_valid_with_empty_map() {
        let empty_map = std::collections::HashMap::new();
        assert!(!is_stored_role_valid(Some(&empty_map), Some(RoleId::new(999))));
        assert!(!is_stored_role_valid(Some(&empty_map), None));
    }

    #[test]
    fn test_panel_embed_includes_instructions() {
        let g = jeu("TestGame", Some("🎮"), None);
        let embed = build_panel_embed(None, &[&g]);
        let json = serde_json::to_value(embed).unwrap();
        let desc = json["description"].as_str().unwrap();
        assert!(desc.contains("Clique sur une réaction"));
    }
}
