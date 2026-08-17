use super::*;

// ── Component interactions (boutons + select menus legacy des panels) ──

pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(PANEL_SELECT_PREFIX) || cid.starts_with(PANEL_BUTTON_PREFIX)
}

pub async fn on_component(api: &ApiClient, ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.as_str();

    // ACCUSE IMMEDIAT, avant le moindre appel reseau.
    //
    // S'abonner a un jeu demande de retrouver le panneau, lister les jeux,
    // lire le membre puis poser ou retirer son role : cinq allers-retours, la
    // ou Discord ferme l'interaction au bout de 3 secondes. Sans cet accuse,
    // le clic echoue en « n'a pas repondu a temps » alors que le role, lui,
    // a bien ete change.
    if !handles_component(cid) {
        return;
    }
    if let Err(error) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        warn!(%error, "games: accuse de reception impossible");
        return;
    }

    if cid.starts_with(PANEL_BUTTON_PREFIX) {
        handle_panel_button(api, ctx, component).await;
    } else if cid.starts_with(PANEL_SELECT_PREFIX) {
        handle_panel_select(api, ctx, component).await;
    }
}

/// Clic sur un bouton-icone de jeu : toggle le role (abonnement) puis met a
/// jour le panneau (compteurs). Confirmation ephemere a l'utilisateur.
async fn handle_panel_button(api: &ApiClient, ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };
    let guild_id_str = guild_id.to_string();

    // custom_id : `game_panel_btn|{panel_id}|{game_id}`.
    let rest = match component.data.custom_id.strip_prefix(PANEL_BUTTON_PREFIX) {
        Some(s) => s,
        None => return,
    };
    let (panel_id, game_id) = match rest.split_once('|') {
        Some((p, g)) => (p.to_string(), g.to_string()),
        None => return,
    };

    // Retrouve le panel (pour sa categorie) et les jeux de la categorie.
    let panel = match api.list_panels(&guild_id_str).await {
        Ok(panels) => panels.into_iter().find(|p| p.id == panel_id),
        Err(e) => {
            warn!(error = %e, "Erreur list_panels (bouton jeu)");
            None
        }
    };
    let Some(panel) = panel else {
        reply_component(
            ctx,
            component,
            "Ce panneau n'existe plus. Demande a un admin de le redeployer.",
        )
        .await;
        return;
    };
    let games = match api
        .list_games_by_category(&guild_id_str, panel.category.as_deref())
        .await
    {
        Ok(g) => g,
        Err(e) => {
            warn!(error = %e, "Erreur list_games_by_category (bouton jeu)");
            reply_component(ctx, component, "Erreur : impossible de lister les jeux.").await;
            return;
        }
    };

    let game = match games.iter().find(|g| g.id == game_id) {
        Some(g) => g,
        None => {
            reply_component(ctx, component, "Ce jeu n'existe plus.").await;
            return;
        }
    };
    let role_id = match game.role_id.as_deref().and_then(|s| s.parse::<u64>().ok()) {
        Some(id) => RoleId::new(id),
        None => {
            reply_component(
                ctx,
                component,
                "Ce jeu n'a pas de role associe. Demande a un admin de le recreer.",
            )
            .await;
            return;
        }
    };

    // Toggle du role sur le membre.
    let member = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Erreur fetch member (bouton jeu)");
            reply_component(ctx, component, "Erreur : impossible de lire ton profil.").await;
            return;
        }
    };
    let has = member.roles.contains(&role_id);
    let confirm = if has {
        match member.remove_role(&ctx.http, role_id).await {
            Ok(()) => format!("\u{274e} Tu ne suis plus **{}**.", game.game_name),
            Err(e) => {
                warn!(error = %e, "Erreur remove_role (bouton jeu)");
                "Erreur lors du desabonnement (hierarchie des roles ?).".to_string()
            }
        }
    } else {
        match member.add_role(&ctx.http, role_id).await {
            Ok(()) => format!(
                "\u{2705} Tu suis maintenant **{}** ! Tu seras notifie.",
                game.game_name
            ),
            Err(e) => {
                warn!(error = %e, "Erreur add_role (bouton jeu)");
                "Erreur lors de l'abonnement (hierarchie des roles ?).".to_string()
            }
        }
    };

    // Confirmation ephemere seulement : on ne re-edite PAS le message du panel.
    // L'ancienne version le re-rendait avec `.components(Vec::new())`, ce qui
    // EFFACAIT les boutons apres le premier clic — le panneau devenait inerte.
    // Sans compteur par bouton (impossible sans l'intent GUILD_MEMBERS), il n'y
    // a de toute facon rien a rafraichir : on laisse le panneau intact.
    reply_component(ctx, component, &confirm).await;
}

async fn handle_panel_select(api: &ApiClient, ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };
    let guild_id_str = guild_id.to_string();
    let user_id = component.user.id;

    // Extrait panel_id du custom_id : `game_panel_select_{panel_id}_{chunk_idx}`.
    let suffix = match component.data.custom_id.strip_prefix(PANEL_SELECT_PREFIX) {
        Some(s) => s,
        None => return,
    };
    let panel_id = match suffix.rsplit_once('_') {
        Some((pid, _chunk)) => pid.to_string(),
        None => suffix.to_string(),
    };

    // Valeurs selectionnees (game_id) dans ce select menu.
    let selected_values: Vec<String> = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values.clone(),
        _ => return,
    };

    // Retrouve le panel pour connaitre sa categorie.
    let panels = match api.list_panels(&guild_id_str).await {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "Erreur list_panels depuis select");
            reply_component(ctx, component, "Erreur : impossible de retrouver le panel.").await;
            return;
        }
    };
    let panel = match panels.into_iter().find(|p| p.id == panel_id) {
        Some(p) => p,
        None => {
            reply_component(
                ctx,
                component,
                "Ce panel n'existe plus. Demande a un admin de le redeployer.",
            )
            .await;
            return;
        }
    };

    let games_in_category = match api
        .list_games_by_category(&guild_id_str, panel.category.as_deref())
        .await
    {
        Ok(g) => g,
        Err(e) => {
            warn!(error = %e, "Erreur list_games_by_category depuis select");
            reply_component(ctx, component, "Erreur : impossible de lister les jeux.").await;
            return;
        }
    };

    // Chaque menu couvre un chunk des jeux (25 options max). On ne synchronise
    // que les jeux de ce chunk.
    const CHUNK_SIZE: usize = 25;
    let chunk_idx: usize = component
        .data
        .custom_id
        .rsplit_once('_')
        .and_then(|(_, n)| n.parse::<usize>().ok())
        .unwrap_or(0);

    let chunk_games: Vec<&Game> = games_in_category
        .chunks(CHUNK_SIZE)
        .nth(chunk_idx)
        .map(|c| c.iter().collect())
        .unwrap_or_default();

    if chunk_games.is_empty() {
        reply_component(ctx, component, "Ce panel est vide ou obsolete.").await;
        return;
    }

    let chunk_game_ids: HashSet<String> = chunk_games.iter().map(|g| g.id.clone()).collect();
    let selected_set: HashSet<String> = selected_values
        .into_iter()
        .filter(|id| chunk_game_ids.contains(id))
        .collect();

    // Recupere le membre pour lire/muter ses roles.
    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Erreur fetch member depuis select");
            reply_component(ctx, component, "Erreur : impossible de lire ton profil.").await;
            return;
        }
    };
    let current_role_ids: HashSet<RoleId> = member.roles.iter().copied().collect();

    let mut added_names: Vec<String> = Vec::new();
    let mut removed_names: Vec<String> = Vec::new();
    let mut skipped_legacy = 0usize;

    // On track aussi l'etat final attendu pour pouvoir afficher la liste
    // complete des jeux actifs apres l'operation (sans re-fetch member).
    let mut active_role_ids: HashSet<RoleId> = current_role_ids.clone();

    for g in &chunk_games {
        let role_id = match g.role_id.as_deref().and_then(|s| s.parse::<u64>().ok()) {
            Some(id) => RoleId::new(id),
            None => {
                skipped_legacy += 1;
                warn!(game = %g.game_name, "Jeu sans role_id : skip (legacy)");
                continue;
            }
        };
        let wants = selected_set.contains(&g.id);
        let has = current_role_ids.contains(&role_id);

        if wants && !has {
            match member.add_role(&ctx.http, role_id).await {
                Ok(()) => {
                    added_names.push(g.game_name.clone());
                    active_role_ids.insert(role_id);
                }
                Err(e) => warn!(error = %e, game = %g.game_name, "Erreur add_role"),
            }
        } else if !wants && has {
            match member.remove_role(&ctx.http, role_id).await {
                Ok(()) => {
                    removed_names.push(g.game_name.clone());
                    active_role_ids.remove(&role_id);
                }
                Err(e) => warn!(error = %e, game = %g.game_name, "Erreur remove_role"),
            }
        }
    }

    // Liste complete des jeux actuellement actifs pour cet user (toutes
    // categories confondues, pas juste le chunk courant).
    let all_games = api
        .list_games_by_category(&guild_id_str, None)
        .await
        .unwrap_or_default();
    let active_games: Vec<String> = all_games
        .iter()
        .filter_map(|g| {
            let rid = g.role_id.as_deref().and_then(|s| s.parse::<u64>().ok())?;
            if active_role_ids.contains(&RoleId::new(rid)) {
                Some(g.game_name.clone())
            } else {
                None
            }
        })
        .collect();

    let response = build_sync_response(&added_names, &removed_names, skipped_legacy, &active_games);
    reply_component(ctx, component, &response).await;
}

fn build_sync_response(
    added: &[String],
    removed: &[String],
    skipped_legacy: usize,
    active_games: &[String],
) -> String {
    let mut lines = Vec::new();

    if !added.is_empty() || !removed.is_empty() {
        lines.push("**Abonnements mis a jour :**".to_string());
        if !added.is_empty() {
            let shown: Vec<&String> = added.iter().take(10).collect();
            let extra = added.len().saturating_sub(shown.len());
            let names = shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            if extra > 0 {
                lines.push(format!("+ {} (+{} autres)", names, extra));
            } else {
                lines.push(format!("+ {}", names));
            }
        }
        if !removed.is_empty() {
            let shown: Vec<&String> = removed.iter().take(10).collect();
            let extra = removed.len().saturating_sub(shown.len());
            let names = shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            if extra > 0 {
                lines.push(format!("- {} (+{} autres)", names, extra));
            } else {
                lines.push(format!("- {}", names));
            }
        }
    } else if skipped_legacy == 0 {
        lines.push("Aucun changement.".to_string());
    }

    if skipped_legacy > 0 {
        lines.push(format!(
            "*{} jeu(x) ignore(s) : pas encore de role Discord associe (recree-les via `/game-admin create`).*",
            skipped_legacy
        ));
    }

    if active_games.is_empty() {
        lines.push("\n**Tu ne suis aucun jeu actuellement.**".to_string());
    } else {
        let shown: Vec<&String> = active_games.iter().take(20).collect();
        let extra = active_games.len().saturating_sub(shown.len());
        let names = shown
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let suffix = if extra > 0 {
            format!(" (+{} autres)", extra)
        } else {
            String::new()
        };
        lines.push(format!(
            "\n**Tu suis actuellement ({}) :** {}{}",
            active_games.len(),
            names,
            suffix
        ));
    }

    lines.join("\n")
}
