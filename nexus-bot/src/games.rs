//! Module games — /game, /game-admin, panneaux d'abonnement par boutons.
//!
//! Porte depuis sentinel-bot (modules/games). Le ping des joueurs est fait
//! nativement via un role Discord par jeu : chaque jeu cree par
//! `/game-admin create` genere un role Discord mentionnable (`<@&role_id>`).
//! S'abonner = recevoir le role, se desabonner = perdre le role.

use std::collections::{HashMap, HashSet};

use serenity::all::{
    ButtonStyle, Colour, CommandDataOptionValue, CommandInteraction, CommandOptionType,
    ComponentInteraction, ComponentInteractionDataKind, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditMessage, EditRole, GuildId,
    ReactionType, RoleId, UserId,
};
use serenity::builder::CreateEmbed;
use tracing::{info, warn};

use crate::api_client::{ApiClient, Game};

/// Prefix du custom_id des select menus de panel de jeux (LEGACY : anciens
/// panels deployes avant la bascule boutons ; le handler reste pour compat).
/// Format : `game_panel_select_{panel_id}_{chunk_index}`.
pub const PANEL_SELECT_PREFIX: &str = "game_panel_select_";

/// Prefix du custom_id des boutons-icones de panel de jeux.
/// Format : `game_panel_btn|{panel_id}|{game_id}`. Cliquer toggle le role du
/// jeu (abonnement aux notifs) et met a jour le compteur d'abonnes du bouton.
pub const PANEL_BUTTON_PREFIX: &str = "game_panel_btn|";

/// Max jeux affiches dans un panel a boutons (5 boutons x 5 rangees).
pub const MAX_BUTTONS_PER_PANEL: usize = 25;

/// Couleur par defaut des roles de jeu (l'original la lisait dans la config
/// guild du module ; nexus n'a pas encore de config par guild -> constante).
const ROLE_COLOR: u32 = 0x3498db;

// ── Enregistrement des commandes ──

pub fn register_commands() -> Vec<CreateCommand> {
    vec![register_public(), register_admin()]
}

fn register_public() -> CreateCommand {
    CreateCommand::new("game")
        .description("Consulter et s'inscrire aux jeux")
        .default_member_permissions(serenity::all::Permissions::empty())
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "Lister les jeux disponibles",
        ))
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "join", "S'inscrire a un jeu")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "name", "Nom du jeu")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "leave",
                "Se desinscrire d'un jeu",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "name", "Nom du jeu")
                    .required(true),
            ),
        )
}

fn register_admin() -> CreateCommand {
    CreateCommand::new("game-admin")
        .description("Gerer les jeux (admin)")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "create", "Creer un jeu")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "name", "Nom du jeu")
                        .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "emoji",
                        "Emoji optionnel (unicode ou <:name:id>)",
                    )
                    .required(false),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "category",
                        "Categorie (ex: RPG)",
                    )
                    .required(false),
                ),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "delete", "Supprimer un jeu")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::String, "name", "Nom du jeu")
                        .required(true),
                ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "panel",
                "Deployer le panneau d'une categorie",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "category",
                    "Categorie (vide = jeux sans categorie)",
                )
                .required(false),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "refresh",
                "Rafraichir le panneau d'une categorie",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "category",
                    "Categorie (vide = jeux sans categorie)",
                )
                .required(false),
            ),
        )
}

// ── Dispatch ──

pub async fn handle_command(api: &ApiClient, ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(g) => g.to_string(),
        None => {
            reply(
                ctx,
                command,
                "Cette commande ne fonctionne que dans un serveur.",
            )
            .await;
            return;
        }
    };

    let sub_name = command
        .data
        .options
        .first()
        .map(|o| o.name.as_str())
        .unwrap_or("");
    let top_name = command.data.name.as_str();

    match (top_name, sub_name) {
        ("game-admin", "create") => handle_create(ctx, command, api, &guild_id).await,
        ("game-admin", "delete") => handle_delete(ctx, command, api, &guild_id).await,
        ("game-admin", "panel") => handle_panel(ctx, command, api, &guild_id).await,
        ("game-admin", "refresh") => handle_refresh(ctx, command, api, &guild_id).await,
        ("game", "list") => handle_list(ctx, command, api, &guild_id).await,
        ("game", "join") => handle_join(ctx, command, api, &guild_id).await,
        ("game", "leave") => handle_leave(ctx, command, api, &guild_id).await,
        _ => reply(ctx, command, "Sous-commande inconnue.").await,
    }
}

// ── Sub-commands ──

async fn handle_create(ctx: &Context, cmd: &CommandInteraction, api: &ApiClient, guild_id: &str) {
    if !has_manage_guild(cmd) {
        reply(
            ctx,
            cmd,
            "Tu as besoin de la permission **Gerer le serveur** pour creer un jeu.",
        )
        .await;
        return;
    }

    let name = get_string_option(cmd, "name").unwrap_or_default();
    let emoji_raw = get_string_option(cmd, "emoji");
    let category = get_string_option(cmd, "category");

    // Emoji optionnel : on valide seulement s'il est fourni.
    let emoji_clean: Option<String> = match emoji_raw.as_deref().map(str::trim) {
        Some(e) if !e.is_empty() => {
            if parse_reaction_type(e).is_none() {
                reply(ctx, cmd, "Emoji invalide. Utilise un emoji unicode (ex. 🎮) ou un emoji serveur (ex. `<:name:123456>`).").await;
                return;
            }
            Some(e.to_string())
        }
        _ => None,
    };

    let guild_id_obj = match cmd.guild_id {
        Some(g) => g,
        None => {
            reply(ctx, cmd, "Commande disponible uniquement dans un serveur.").await;
            return;
        }
    };

    // 1) Cree le role Discord.
    let role = match guild_id_obj
        .create_role(
            &ctx.http,
            EditRole::new()
                .name(&name)
                .colour(Colour::new(ROLE_COLOR))
                .mentionable(true)
                .hoist(false),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, game = %name, "Erreur create_role Discord");
            reply(ctx, cmd, &format!(
                "Erreur creation du role Discord : {e}. Verifie que le bot a la permission **Gerer les roles**."
            )).await;
            return;
        }
    };
    let role_id_str = role.id.get().to_string();

    // 2) Insere en DB avec le role_id. Si ca echoue, rollback du role Discord.
    match api
        .create_game(
            guild_id,
            &name,
            &cmd.user.id.to_string(),
            Some(&role_id_str),
            emoji_clean.as_deref(),
            category.as_deref(),
        )
        .await
    {
        Ok(game) => {
            let desc = format!(
                "**{}** {} est maintenant disponible.\nCategorie : {}\nRole : <@&{}>\nLes joueurs peuvent s'inscrire avec `/game join {}` ou via le panneau.",
                game.game_name,
                game.emoji.clone().unwrap_or_default(),
                game.category.clone().unwrap_or_else(|| "(aucune)".into()),
                role_id_str,
                game.game_name,
            );
            let embed = success_embed("Jeu cree !").description(desc);
            reply_embed(ctx, cmd, embed).await;
            info!(game = %game.game_name, role = %role_id_str, guild = %guild_id, "Jeu cree (avec role)");
        }
        Err(e) => {
            // Rollback : le jeu n'a pas ete cree, on supprime le role pour
            // eviter de laisser un role orphelin.
            if let Err(del_err) = guild_id_obj.delete_role(&ctx.http, role.id).await {
                warn!(error = %del_err, role = %role_id_str, "Rollback delete_role a echoue");
            }
            reply(ctx, cmd, &format!("Erreur : {e}")).await;
        }
    }
}

async fn handle_delete(ctx: &Context, cmd: &CommandInteraction, api: &ApiClient, guild_id: &str) {
    if !has_manage_guild(cmd) {
        reply(
            ctx,
            cmd,
            "Tu as besoin de la permission **Gerer le serveur** pour supprimer un jeu.",
        )
        .await;
        return;
    }

    let name = get_string_option(cmd, "name").unwrap_or_default();
    let game = match api.get_game_by_name(guild_id, &name).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            reply(ctx, cmd, &format!("Jeu **{}** introuvable.", name)).await;
            return;
        }
        Err(e) => {
            reply(ctx, cmd, &format!("Erreur : {e}")).await;
            return;
        }
    };

    match api.delete_game(guild_id, &game.id).await {
        Ok(()) => {
            // Supprime le role Discord associe (best-effort : si l'admin
            // l'a deja supprime a la main, on ignore).
            if let (Some(role_id_str), Some(guild_id_obj)) = (game.role_id.as_deref(), cmd.guild_id)
            {
                if let Ok(rid) = role_id_str.parse::<u64>() {
                    if let Err(e) = guild_id_obj.delete_role(&ctx.http, RoleId::new(rid)).await {
                        warn!(error = %e, role = %role_id_str, game = %game.game_name, "Erreur delete_role (le role a peut-etre deja ete supprime manuellement)");
                    }
                }
            }
            reply(ctx, cmd, &format!("Jeu **{}** supprime.", game.game_name)).await;
            info!(game = %game.game_name, guild = %guild_id, "Jeu supprime");
        }
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

async fn handle_list(ctx: &Context, cmd: &CommandInteraction, api: &ApiClient, guild_id: &str) {
    match api.list_games(guild_id).await {
        Ok(games) => {
            if games.is_empty() {
                reply(
                    ctx,
                    cmd,
                    "Aucun jeu configure. Un admin peut en creer avec `/game-admin create`.",
                )
                .await;
            } else {
                let list: String = games
                    .iter()
                    .map(|g| {
                        format!(
                            "- {} **{}**",
                            g.emoji.clone().unwrap_or_default(),
                            g.game_name
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let embed = info_embed("Jeux disponibles")
                    .description(format!("{}\n\n*Inscris-toi avec `/game join <nom>`*", list));
                reply_embed(ctx, cmd, embed).await;
            }
        }
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

async fn handle_join(ctx: &Context, cmd: &CommandInteraction, api: &ApiClient, guild_id: &str) {
    let name = get_string_option(cmd, "name").unwrap_or_default();
    let game = match api.get_game_by_name(guild_id, &name).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            reply(
                ctx,
                cmd,
                &format!(
                    "Jeu **{}** introuvable. Utilise `/game list` pour voir les jeux.",
                    name
                ),
            )
            .await;
            return;
        }
        Err(e) => {
            reply(ctx, cmd, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let (guild_id_obj, role_id) = match (
        cmd.guild_id,
        game.role_id.as_deref().and_then(|s| s.parse::<u64>().ok()),
    ) {
        (Some(g), Some(rid)) => (g, RoleId::new(rid)),
        (Some(_), None) => {
            reply(ctx, cmd, &format!(
                "Le jeu **{}** n'a pas de role Discord associe (jeu legacy). Demande a un admin de le recreer.",
                game.game_name
            )).await;
            return;
        }
        _ => {
            reply(ctx, cmd, "Commande disponible uniquement dans un serveur.").await;
            return;
        }
    };

    let member = match guild_id_obj.member(&ctx.http, cmd.user.id).await {
        Ok(m) => m,
        Err(e) => {
            reply(ctx, cmd, &format!("Impossible de lire ton profil : {e}")).await;
            return;
        }
    };
    match member.add_role(&ctx.http, role_id).await {
        Ok(()) => {
            reply(
                ctx,
                cmd,
                &format!(
                    "Tu es inscrit a **{}** ! Utilise <@&{}> pour pinger les joueurs.",
                    game.game_name,
                    role_id.get()
                ),
            )
            .await
        }
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

async fn handle_leave(ctx: &Context, cmd: &CommandInteraction, api: &ApiClient, guild_id: &str) {
    let name = get_string_option(cmd, "name").unwrap_or_default();
    let game = match api.get_game_by_name(guild_id, &name).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            reply(ctx, cmd, &format!("Jeu **{}** introuvable.", name)).await;
            return;
        }
        Err(e) => {
            reply(ctx, cmd, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let (guild_id_obj, role_id) = match (
        cmd.guild_id,
        game.role_id.as_deref().and_then(|s| s.parse::<u64>().ok()),
    ) {
        (Some(g), Some(rid)) => (g, RoleId::new(rid)),
        (Some(_), None) => {
            reply(
                ctx,
                cmd,
                &format!(
                    "Le jeu **{}** n'a pas de role Discord associe.",
                    game.game_name
                ),
            )
            .await;
            return;
        }
        _ => {
            reply(ctx, cmd, "Commande disponible uniquement dans un serveur.").await;
            return;
        }
    };

    let member = match guild_id_obj.member(&ctx.http, cmd.user.id).await {
        Ok(m) => m,
        Err(e) => {
            reply(ctx, cmd, &format!("Impossible de lire ton profil : {e}")).await;
            return;
        }
    };
    match member.remove_role(&ctx.http, role_id).await {
        Ok(()) => {
            reply(
                ctx,
                cmd,
                &format!("Tu es desinscrit de **{}**.", game.game_name),
            )
            .await
        }
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

// ── Panels ──

async fn handle_panel(ctx: &Context, cmd: &CommandInteraction, api: &ApiClient, guild_id: &str) {
    if !has_manage_guild(cmd) {
        reply(ctx, cmd, "Permission **Gerer le serveur** requise.").await;
        return;
    }

    let category = get_string_option(cmd, "category");
    let games = match api
        .list_games_by_category(guild_id, category.as_deref())
        .await
    {
        Ok(g) => g,
        Err(e) => {
            reply(ctx, cmd, &format!("Erreur : {e}")).await;
            return;
        }
    };

    if games.is_empty() {
        reply(
            ctx,
            cmd,
            "Aucun jeu dans cette categorie. Ajoute-en avec `/game-admin create`.",
        )
        .await;
        return;
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
            reply(ctx, cmd, &format!("Erreur envoi message : {e}")).await;
            return;
        }
    };

    // 2) Sauve le panel en DB. On a besoin de son id pour les custom_id des
    //    boutons, d'ou l'ordre : message d'abord, sauvegarde, puis boutons.
    let panel = match api
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
            reply(
                ctx,
                cmd,
                &format!("Panel envoye mais erreur de sauvegarde : {e}"),
            )
            .await;
            return;
        }
    };

    // 3) Attache les boutons (un par jeu). Cliquer bascule le role d'abonnement.
    let counts = match cmd.guild_id {
        Some(gid) => count_subscribers(ctx, gid, &game_role_ids(&games_slice)).await,
        None => HashMap::new(),
    };
    let buttons = build_panel_buttons(&panel.id, &games_slice, &counts);
    if let Err(e) = msg
        .edit(&ctx.http, EditMessage::new().components(buttons))
        .await
    {
        reply(
            ctx,
            cmd,
            &format!("Panel envoye mais erreur d'ajout des boutons : {e}"),
        )
        .await;
        return;
    }

    reply(
        ctx,
        cmd,
        &format!("Panneau deploye ({} jeux).", games_slice.len()),
    )
    .await;
}

async fn handle_refresh(ctx: &Context, cmd: &CommandInteraction, api: &ApiClient, guild_id: &str) {
    if !has_manage_guild(cmd) {
        reply(ctx, cmd, "Permission **Gerer le serveur** requise.").await;
        return;
    }

    let category = get_string_option(cmd, "category");

    // Trouve le panel existant via list_panels.
    let panels = match api.list_panels(guild_id).await {
        Ok(p) => p,
        Err(e) => {
            reply(ctx, cmd, &format!("Erreur : {e}")).await;
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
            reply(
                ctx,
                cmd,
                "Aucun panneau existant pour cette categorie. Utilise `/game-admin panel` d'abord.",
            )
            .await;
            return;
        }
    };

    let games = match api
        .list_games_by_category(guild_id, category.as_deref())
        .await
    {
        Ok(g) => g,
        Err(e) => {
            reply(ctx, cmd, &format!("Erreur : {e}")).await;
            return;
        }
    };
    let games_slice: Vec<&Game> = games.iter().take(MAX_BUTTONS_PER_PANEL).collect();

    let embed = build_panel_embed(category.as_deref(), &games_slice);
    let _gid = cmd.guild_id.unwrap_or_default();

    let channel_id: serenity::model::id::ChannelId = match panel.channel_id.parse::<u64>() {
        Ok(id) => serenity::model::id::ChannelId::new(id),
        Err(_) => {
            reply(ctx, cmd, "channel_id invalide en DB.").await;
            return;
        }
    };
    let message_id: serenity::model::id::MessageId = match panel.message_id.parse::<u64>() {
        Ok(id) => serenity::model::id::MessageId::new(id),
        Err(_) => {
            reply(ctx, cmd, "message_id invalide en DB.").await;
            return;
        }
    };

    let mut msg = match channel_id.message(&ctx.http, message_id).await {
        Ok(m) => m,
        Err(e) => {
            reply(ctx, cmd, &format!("Message panneau introuvable : {e}")).await;
            return;
        }
    };

    // Met a jour l'embed ET les boutons (le catalogue et les compteurs ont pu changer).
    let counts = match cmd.guild_id {
        Some(gid) => count_subscribers(ctx, gid, &game_role_ids(&games_slice)).await,
        None => HashMap::new(),
    };
    let buttons = build_panel_buttons(&panel.id, &games_slice, &counts);
    if let Err(e) = msg
        .edit(&ctx.http, EditMessage::new().embed(embed).components(buttons))
        .await
    {
        reply(ctx, cmd, &format!("Erreur edition : {e}")).await;
        return;
    }

    reply(
        ctx,
        cmd,
        &format!("Panneau rafraichi ({} jeux).", games_slice.len()),
    )
    .await;
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
            "\n\n*Clique sur un bouton ci-dessous pour rejoindre ou quitter un jeu et recevoir (ou non) ses notifications.*",
        );
        s
    };
    info_embed(&title).description(desc)
}

/// Rangées de boutons du panneau : un bouton par jeu (emoji + nom + nombre
/// d'abonnés), cliquer bascule le rôle d'abonnement. Discord limite à 5 boutons
/// par rangée et 5 rangées, soit `MAX_BUTTONS_PER_PANEL` jeux.
///
/// `counts` associe chaque `RoleId` à son nombre de porteurs (cf.
/// [`count_subscribers`]) ; un jeu absent de la map (ou sans rôle) n'affiche
/// pas de compteur.
pub(crate) fn build_panel_buttons(
    panel_id: &str,
    games: &[&Game],
    counts: &HashMap<RoleId, usize>,
) -> Vec<CreateActionRow> {
    let mut rows = Vec::new();
    for chunk in games.chunks(5) {
        let mut buttons = Vec::with_capacity(chunk.len());
        for g in chunk {
            let count = g
                .role_id
                .as_deref()
                .and_then(|s| s.parse::<u64>().ok())
                .and_then(|rid| counts.get(&RoleId::new(rid)))
                .copied();
            // Le nom sert de libellé (les icônes seules déroutaient), suivi du
            // nombre d'abonnés quand il est connu. Discord borne à 80 caractères.
            let label: String = match count {
                Some(n) => format!("{} · {}", g.game_name, n),
                None => g.game_name.clone(),
            }
            .chars()
            .take(80)
            .collect();
            let mut btn = CreateButton::new(format!("{PANEL_BUTTON_PREFIX}{panel_id}|{}", g.id))
                .label(label)
                .style(ButtonStyle::Secondary);
            if let Some(rt) = g.emoji.as_deref().and_then(parse_reaction_type) {
                btn = btn.emoji(rt);
            }
            buttons.push(btn);
        }
        rows.push(CreateActionRow::Buttons(buttons));
    }
    rows
}

/// Compte les porteurs de chaque rôle de jeu en énumérant les membres de la
/// guilde (pagination REST, 1000 par page). Nécessite l'intent GUILD_MEMBERS.
///
/// Appelé au déploiement et au rafraîchissement d'un panneau — pas à chaque
/// clic — donc le coût de l'énumération reste marginal. En cas d'erreur (intent
/// non activé, réseau), renvoie une map vide : les boutons s'affichent alors
/// sans compteur plutôt que de bloquer le panneau.
async fn count_subscribers(
    ctx: &Context,
    guild_id: GuildId,
    roles: &HashSet<RoleId>,
) -> HashMap<RoleId, usize> {
    let mut counts: HashMap<RoleId, usize> = HashMap::new();
    if roles.is_empty() {
        return counts;
    }
    let mut after: Option<UserId> = None;
    loop {
        let batch = match guild_id.members(&ctx.http, Some(1000), after).await {
            Ok(b) => b,
            Err(e) => {
                warn!(error = %e, "Comptage des abonnes impossible (intent GUILD_MEMBERS ?)");
                break;
            }
        };
        if batch.is_empty() {
            break;
        }
        for member in &batch {
            for role in &member.roles {
                if roles.contains(role) {
                    *counts.entry(*role).or_default() += 1;
                }
            }
        }
        // Page pleine = il reste probablement des membres : on pagine via le
        // dernier id. Page partielle = fin de la liste.
        if batch.len() < 1000 {
            break;
        }
        after = batch.last().map(|m| m.user.id);
    }
    counts
}

/// Extrait les `RoleId` des jeux (ceux qui en ont un), pour le comptage.
fn game_role_ids(games: &[&Game]) -> HashSet<RoleId> {
    games
        .iter()
        .filter_map(|g| g.role_id.as_deref())
        .filter_map(|s| s.parse::<u64>().ok())
        .map(RoleId::new)
        .collect()
}





// ── Component interactions (boutons + select menus legacy des panels) ──

pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(PANEL_SELECT_PREFIX) || cid.starts_with(PANEL_BUTTON_PREFIX)
}

pub async fn on_component(api: &ApiClient, ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.as_str();
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

// ── Helpers ──


fn get_string_option(cmd: &CommandInteraction, name: &str) -> Option<String> {
    let sub = cmd.data.options.first()?;
    if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
        opts.iter().find(|o| o.name == name).and_then(|o| {
            if let CommandDataOptionValue::String(s) = &o.value {
                Some(s.clone())
            } else {
                None
            }
        })
    } else {
        None
    }
}

/// Permission admin lue directement sur l'interaction (permissions du membre
/// calculees par Discord) — plus fiable et plus simple que le fetch member +
/// cache de l'original.
fn has_manage_guild(cmd: &CommandInteraction) -> bool {
    cmd.member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.manage_guild() || p.administrator())
        .unwrap_or(false)
}

fn info_embed(title: &str) -> CreateEmbed {
    CreateEmbed::new().title(title.to_string()).color(0x3498db)
}

fn success_embed(title: &str) -> CreateEmbed {
    CreateEmbed::new().title(title.to_string()).color(0x2ecc71)
}

async fn reply(ctx: &Context, cmd: &CommandInteraction, content: &str) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    if let Err(e) = cmd.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur reponse commande game");
    }
}

async fn reply_embed(ctx: &Context, cmd: &CommandInteraction, embed: CreateEmbed) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .ephemeral(true),
    );
    if let Err(e) = cmd.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur reponse embed commande game");
    }
}

async fn reply_component(ctx: &Context, component: &ComponentInteraction, content: &str) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    );
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Erreur reponse ephemeral games panel");
    }
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
                        if let (Some(event), Some(data)) = (value.get("event").and_then(|v| v.as_str()), value.get("data")) {
                            if event == "games_panel_deploy" {
                                if let (Some(guild_id), Some(channel_id)) = (
                                    data.get("guild_id").and_then(|v| v.as_str()),
                                    data.get("channel_id").and_then(|v| v.as_str())
                                ) {
                                    let category = data.get("category").and_then(|v| v.as_str());
                                    deploy_panel_from_event(&ctx_clone, &api_clone, guild_id, channel_id, category).await;
                                }
                            }
                        }
                    }
                }
            }
        ).await;
    });
}

async fn deploy_panel_from_event(ctx: &Context, api: &ApiClient, guild_id: &str, channel_id: &str, category: Option<&str>) {
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

    let msg = match chan_id.send_message(&ctx.http, CreateMessage::new().embed(embed)).await {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Erreur envoi message panel");
            return;
        }
    };

    let _panel = match api.save_panel(guild_id, channel_id, &msg.id.to_string(), category).await {
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

pub async fn handle_reaction(api: &ApiClient, ctx: &Context, reaction: &serenity::all::Reaction, is_add: bool) {
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

    // Cherche si l'emoji correspond a un jeu
    let games = match api.list_games(&guild_id.to_string()).await {
        Ok(g) => g,
        Err(_) => return,
    };
    
    let game = match games.iter().find(|g| {
        if let Some(emoji) = &g.emoji {
            emoji == &reaction_str || parse_reaction_type(emoji).map(|rt| {
                match rt {
                    ReactionType::Custom { id, .. } => id.to_string() == reaction_str,
                    ReactionType::Unicode(u) => u == reaction_str,
                    _ => false
                }
            }).unwrap_or(false)
        } else {
            false
        }
    }) {
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
