//! Module games — /game, /game-admin, panneaux d'abonnement par boutons.
//!
//! Porte depuis sentinel-bot (modules/games). Le ping des joueurs est fait
//! nativement via un role Discord par jeu : chaque jeu cree par
//! `/game-admin create` genere un role Discord mentionnable (`<@&role_id>`).
//! S'abonner = recevoir le role, se desabonner = perdre le role.

use std::collections::HashSet;

use serenity::all::{
    Colour, CommandDataOptionValue, CommandInteraction, CommandOptionType, ComponentInteraction,
    ComponentInteractionDataKind, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    EditInteractionResponse, EditMessage, EditRole, GuildId, ReactionType, RoleId,
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
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "parametres",
            "Voir les reglages du serveur de ce salon (reponse privee)",
        ))
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
        ("game", "parametres") => handle_show_params(ctx, command, api).await,
        _ => reply(ctx, command, "Sous-commande inconnue.").await,
    }
}

// ── Sub-commands ──

/// `/game parametres` — accessible a TOUS les joueurs. Affiche les reglages du
/// serveur de session correspondant au salon courant, en reponse EPHEMERE
/// (visible du seul demandeur). Contextuel : dans le salon prive des inscrits le
/// mot de passe est inclus ; dans le salon d'inscription il est masque.
async fn handle_show_params(ctx: &Context, cmd: &CommandInteraction, api: &ApiClient) {
    match crate::game_portal::params_embeds_for_channel(ctx, api, cmd.channel_id).await {
        Ok(embeds) => {
            let _ = cmd
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .embeds(embeds)
                            .ephemeral(true),
                    ),
                )
                .await;
        }
        Err(message) => reply(ctx, cmd, message).await,
    }
}

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

    let emoji_clean = match clean_emoji(emoji_raw.as_deref()) {
        Ok(e) => e,
        Err(msg) => {
            reply(ctx, cmd, msg).await;
            return;
        }
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
            let desc = format_game_created_description(&game, &role_id_str);
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
                let content = format_game_list_content(&games);
                let embed = info_embed("Jeux disponibles").description(content);
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
                &format_game_join_success(&game.game_name, role_id.get()),
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
                &format_game_leave_success(&game.game_name),
            )
            .await
        }
        Err(e) => reply(ctx, cmd, &format!("Erreur : {e}")).await,
    }
}

pub fn format_game_created_description(game: &Game, role_id_str: &str) -> String {
    format!(
        "**{}** {} est maintenant disponible.\nCategorie : {}\nRole : <@&{}>\nLes joueurs peuvent s'inscrire avec `/game join {}` ou via le panneau.",
        game.game_name,
        game.emoji.clone().unwrap_or_default(),
        game.category.clone().unwrap_or_else(|| "(aucune)".into()),
        role_id_str,
        game.game_name,
    )
}

pub fn format_game_list_content(games: &[Game]) -> String {
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
    format!("{}\n\n*Inscris-toi avec `/game join <nom>`*", list)
}

pub fn format_game_join_success(game_name: &str, role_id: u64) -> String {
    format!(
        "Tu es inscrit a **{}** ! Utilise <@&{}> pour pinger les joueurs.",
        game_name,
        role_id
    )
}

pub fn format_game_leave_success(game_name: &str) -> String {
    format!("Tu es desinscrit de **{}**.", game_name)
}

pub fn clean_emoji(emoji_raw: Option<&str>) -> Result<Option<String>, &'static str> {
    match emoji_raw.map(str::trim) {
        Some(e) if !e.is_empty() => {
            if parse_reaction_type(e).is_none() {
                Err("Emoji invalide. Utilise un emoji unicode (ex. 🎮) ou un emoji serveur (ex. `<:name:123456>`).")
            } else {
                Ok(Some(e.to_string()))
            }
        }
        _ => Ok(None),
    }
}

mod interactions;
mod panels;
mod reactions;
pub(crate) mod sync;

pub(crate) use interactions::{handles_component, on_component};
pub(crate) use panels::{build_panel_embed, ensure_game_roles};
use panels::{handle_panel, handle_refresh};
pub(crate) use reactions::{handle_reaction, parse_reaction_type, spawn_listener};

#[cfg(test)]
use panels::panel_reactions;
#[cfg(test)]
use reactions::find_game_for_reaction;

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

pub fn build_game_reply(content: &str) -> CreateInteractionResponse {
    CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true),
    )
}

pub fn build_game_embed_reply(embed: CreateEmbed) -> CreateInteractionResponse {
    CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .ephemeral(true),
    )
}

pub fn build_game_edit_reply(content: &str) -> EditInteractionResponse {
    EditInteractionResponse::new().content(content)
}

pub fn build_game_component_followup(content: &str) -> serenity::all::CreateInteractionResponseFollowup {
    serenity::all::CreateInteractionResponseFollowup::new()
        .content(content)
        .ephemeral(true)
}

async fn reply(ctx: &Context, cmd: &CommandInteraction, content: &str) {
    if let Err(e) = cmd.create_response(&ctx.http, build_game_reply(content)).await {
        warn!(error = %e, "Erreur reponse commande game");
    }
}

async fn edit_deferred_reply(ctx: &Context, cmd: &CommandInteraction, content: &str) {
    if let Err(e) = cmd
        .edit_response(&ctx.http, build_game_edit_reply(content))
        .await
    {
        warn!(error = %e, "Erreur edition reponse differee commande game");
    }
}

async fn reply_embed(ctx: &Context, cmd: &CommandInteraction, embed: CreateEmbed) {
    if let Err(e) = cmd.create_response(&ctx.http, build_game_embed_reply(embed)).await {
        warn!(error = %e, "Erreur reponse embed commande game");
    }
}

/// Reponse ephemere a un clic de panneau.
///
/// Un followup, et non une reponse initiale : l'interaction a deja ete
/// acquittee des le premier instant du handler (cf. `on_component`), sans quoi
/// les appels API et Discord qui suivent depassent les 3 s accordees par
/// Discord et le clic echoue en « n'a pas repondu a temps ».
async fn reply_component(ctx: &Context, component: &ComponentInteraction, content: &str) {
    if let Err(e) = component
        .create_followup(&ctx.http, build_game_component_followup(content))
        .await
    {
        warn!(error = %e, "Erreur reponse component");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: &str, emoji: Option<&str>) -> Game {
        Game {
            id: id.to_string(),
            game_name: format!("Jeu {id}"),
            emoji: emoji.map(str::to_string),
            category: None,
            role_id: None,
        }
    }

    #[test]
    fn test_register_commands() {
        let cmds = register_commands();
        assert_eq!(cmds.len(), 2);
        let j0 = serde_json::to_value(&cmds[0]).unwrap();
        let j1 = serde_json::to_value(&cmds[1]).unwrap();
        assert_eq!(j0["name"], "game");
        assert_eq!(j1["name"], "game-admin");
    }

    #[test]
    fn panel_reactions_keeps_valid_unique_emojis_in_catalog_order() {
        let games = [
            game("1", Some(" 🎮 ")),
            game("2", Some("🎯")),
            game("3", Some("🎮")),
            game("4", None),
            game("5", Some("   ")),
        ];
        let refs = games.iter().collect::<Vec<_>>();

        let reactions = panel_reactions(&refs);

        assert_eq!(reactions.len(), 2);
        assert!(matches!(&reactions[0], ReactionType::Unicode(value) if value == "🎮"));
        assert!(matches!(&reactions[1], ReactionType::Unicode(value) if value == "🎯"));
    }

    #[test]
    fn panel_reactions_supports_discord_custom_emojis() {
        let games = [game("1", Some("<:battlefield:123456789012345678>"))];
        let refs = games.iter().collect::<Vec<_>>();

        let reactions = panel_reactions(&refs);

        assert_eq!(reactions.len(), 1);
        assert!(matches!(
            &reactions[0],
            ReactionType::Custom { id, animated: false, .. }
                if id.get() == 123_456_789_012_345_678
        ));
    }

    #[test]
    fn reaction_lookup_supports_unicode_and_custom_emojis() {
        let games = [
            game("unicode", Some("  🎮 ")),
            game("custom", Some("<:battlefield:123456789012345678>")),
        ];

        assert_eq!(
            find_game_for_reaction(&games, "🎮").map(|game| game.id.as_str()),
            Some("unicode")
        );
        assert_eq!(
            find_game_for_reaction(&games, "123456789012345678").map(|game| game.id.as_str()),
            Some("custom")
        );
        assert!(find_game_for_reaction(&games, "❌").is_none());
    }

    #[test]
    fn test_parse_reaction_type() {
        assert_eq!(parse_reaction_type(""), None);
        assert_eq!(parse_reaction_type("   "), None);

        let custom = parse_reaction_type("<:mon_emoji:9876543210>").unwrap();
        assert!(matches!(custom, ReactionType::Custom { id, animated: false, .. } if id.get() == 9876543210));

        let custom_anim = parse_reaction_type("<a:mon_anim:1122334455>").unwrap();
        assert!(matches!(custom_anim, ReactionType::Custom { id, animated: true, .. } if id.get() == 1122334455));

        let unicode = parse_reaction_type("🎲").unwrap();
        assert!(matches!(unicode, ReactionType::Unicode(u) if u == "🎲"));
    }

    #[test]
    fn test_build_panel_embed() {
        let embed_empty = build_panel_embed(None, &[]);
        let j_empty = serde_json::to_value(&embed_empty).unwrap();
        assert_eq!(j_empty["title"], "- [ Jeux ] -");

        let g1 = Game {
            id: "g1".into(),
            game_name: "Minecraft".into(),
            emoji: Some("⛏️".into()),
            category: Some("Survie".into()),
            role_id: Some("12345".into()),
        };
        let embed_with_games = build_panel_embed(Some("Survie"), &[&g1]);
        let j_games = serde_json::to_value(&embed_with_games).unwrap();
        assert_eq!(j_games["title"], "- [ Survie ] -");
        assert!(j_games["description"].as_str().unwrap().contains("<@&12345>"));
    }

    #[test]
    fn test_embed_builders() {
        let info = info_embed("Info Title");
        let j_info = serde_json::to_value(&info).unwrap();
        assert_eq!(j_info["title"], "Info Title");

        let success = success_embed("Success Title");
        let j_succ = serde_json::to_value(&success).unwrap();
        assert_eq!(j_succ["title"], "Success Title");
    }

    #[test]
    fn test_game_formatting_helpers() {
        let g = Game {
            id: "g1".into(),
            game_name: "Palworld".into(),
            emoji: Some("🦖".into()),
            category: Some("Aventure".into()),
            role_id: Some("999".into()),
        };
        let desc = format_game_created_description(&g, "999");
        assert!(desc.contains("Palworld"));
        assert!(desc.contains("<@&999>"));

        let list_txt = format_game_list_content(&[g]);
        assert!(list_txt.contains("Palworld"));
        assert!(list_txt.contains("/game join"));

        let join_txt = format_game_join_success("Palworld", 999);
        assert!(join_txt.contains("<@&999>"));

        let leave_txt = format_game_leave_success("Palworld");
        assert!(leave_txt.contains("desinscrit"));

        let rep = build_game_reply("Message test");
        let j_rep = serde_json::to_value(&rep).unwrap();
        assert_eq!(j_rep["data"]["content"], "Message test");

        let emb_rep = build_game_embed_reply(info_embed("Info Title"));
        let j_emb_rep = serde_json::to_value(&emb_rep).unwrap();
        assert!(j_emb_rep["data"]["embeds"].as_array().is_some());

        let edit_rep = build_game_edit_reply("Edit test");
        let j_edit = serde_json::to_value(&edit_rep).unwrap();
        assert_eq!(j_edit["content"], "Edit test");

        let comp_fup = build_game_component_followup("Followup test");
        let j_fup = serde_json::to_value(&comp_fup).unwrap();
        assert_eq!(j_fup["content"], "Followup test");

        assert_eq!(clean_emoji(None), Ok(None));
        assert_eq!(clean_emoji(Some("  ")), Ok(None));
        assert_eq!(clean_emoji(Some("🎮")), Ok(Some("🎮".into())));
    }
}
