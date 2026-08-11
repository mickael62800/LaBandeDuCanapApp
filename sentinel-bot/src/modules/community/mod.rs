//! Module community — panels de roles, auto-roles, sponsorship, temp roles
//! (ex community-bot + roles-bot).

pub const MODULE_BOT_NAME: &str = "community-bot";

pub mod api_client;
pub mod cooldown;
pub mod exclusive_groups;
pub mod roles_panel;
pub mod sponsor;
pub mod sponsorship;
pub mod temp_roles;

use std::sync::Arc;

use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use serenity::model::application::ButtonStyle;
use serenity::model::channel::Message;
use serenity::model::guild::Member;
use serenity::model::id::RoleId;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};
use crate::shared::embeds::{neutral_embed, success_embed};
use crate::shared::heartbeat::ApiClientKey;

use api_client::{ApiClient, RolePanelDetail, SyncRole};
use cooldown::InteractionCooldown;
use sponsorship::SponsorshipTracker;
use temp_roles::TempRoleTracker;

// ── TypeMapKeys ──

pub struct RolesApiKey;
impl TypeMapKey for RolesApiKey {
    type Value = ApiClient;
}

pub struct CooldownKey;
impl TypeMapKey for CooldownKey {
    type Value = Arc<InteractionCooldown>;
}

pub struct TempRoleKey;
impl TypeMapKey for TempRoleKey {
    type Value = TempRoleTracker;
}

pub struct SponsorshipKey;
impl TypeMapKey for SponsorshipKey {
    type Value = SponsorshipTracker;
}

// ── Slash commands ──

// ── Init TypeMapKeys ──

pub fn init_typemap(
    data: &mut serenity::prelude::TypeMap,
    grpc: &Arc<crate::shared::grpc_client::SentinelGrpcClient>,
) {
    data.insert::<RolesApiKey>(ApiClient::new(Arc::clone(grpc)));
    data.insert::<CooldownKey>(Arc::new(InteractionCooldown::new()));
    data.insert::<TempRoleKey>(TempRoleTracker::new());
    data.insert::<SponsorshipKey>(SponsorshipTracker::new());
}

pub fn register_commands() -> Vec<CreateCommand> {
    vec![roles_panel::register(), sponsor::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    match command.data.name.as_str() {
        "roles-panel" => roles_panel::handle(ctx, command).await,
        "parrain" => sponsor::handle(ctx, command).await,
        _ => {}
    }
}

// ── Component interactions ──

/// Retourne true si ce custom_id est gere par le module community.
pub fn handles_component(cid: &str) -> bool {
    cid.starts_with("role_")
        || cid.starts_with("sponsor_accept:")
        || cid.starts_with("sponsor_refuse:")
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    let cid = component.data.custom_id.as_str();
    if cid.starts_with("role_") {
        handle_role_button(ctx, component).await;
    } else if cid.starts_with("sponsor_accept:") || cid.starts_with("sponsor_refuse:") {
        sponsor::handle_button(ctx, component).await;
    }
}

// ── Event handlers ──

/// Auto-roles quand un nouveau membre rejoint.
pub async fn on_member_add(ctx: &Context, new_member: &Member) {
    let guild_id = new_member.guild_id;

    // Si la verification d'age (module welcome) est active, on NE pose AUCUN
    // auto-role a l'arrivee : le membre ne doit obtenir ses roles qu'apres
    // avoir saisi un age suffisant via le formulaire du reglement. Sinon le
    // role Membre configure en auto-role court-circuiterait la verification.
    if crate::modules::welcome::age_check_active(ctx, guild_id).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<RolesApiKey>() {
        Some(a) => a,
        None => return,
    };

    let auto_roles = match api.get_auto_roles(&guild_id.to_string()).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Erreur chargement auto-roles");
            return;
        }
    };

    for ar in &auto_roles {
        if !ar.enabled {
            continue;
        }

        if ar.delay_secs > 0 {
            let ctx_clone = ctx.clone();
            let guild = guild_id;
            let user = new_member.user.id;
            let role_id: u64 = match ar.role_id.parse() {
                Ok(r) => r,
                Err(_) => continue,
            };
            let delay = ar.delay_secs as u64;

            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                if let Ok(member) = guild.member(&ctx_clone.http, user).await {
                    if let Err(e) = member.add_role(&ctx_clone.http, RoleId::new(role_id)).await {
                        warn!(error = %e, "Failed to add delayed auto-role");
                    }
                }
            });
        } else if let Ok(role_id) = ar.role_id.parse::<u64>() {
            if let Ok(member) = guild_id.member(&ctx.http, new_member.user.id).await {
                if let Err(e) = member.add_role(&ctx.http, RoleId::new(role_id)).await {
                    warn!(error = %e, "Failed to add auto-role");
                }
            }
        }
    }
}

/// Charge les roles temporaires actifs depuis l'API au demarrage.
pub async fn load_temp_roles(ctx: &Context, guild_id: serenity::model::id::GuildId) {
    let data = ctx.data.read().await;
    if let (Some(api), Some(tracker)) = (data.get::<RolesApiKey>(), data.get::<TempRoleKey>()) {
        let gid = guild_id.to_string();
        match api.list_temp_roles(&gid).await {
            Ok(entries) => {
                let mut loaded = 0u32;
                for entry in entries {
                    let g = entry.guild_id.parse::<u64>().unwrap_or(0);
                    let u = entry.user_id.parse::<u64>().unwrap_or(0);
                    let r = entry.role_id.parse::<u64>().unwrap_or(0);
                    if g > 0 && u > 0 && r > 0 {
                        tracker.add_with_expiry_timestamp(g, u, r, &entry.expires_at);
                        loaded += 1;
                    }
                }
                if loaded > 0 {
                    info!(guild = %gid, count = loaded, "Roles temporaires recharges");
                }
            }
            Err(e) => {
                warn!(error = %e, guild = %gid, "Echec chargement roles temporaires");
            }
        }
    }
}

/// Phase 5D — Consumer Redis pour les events `temp_role_expire` publies
/// par le worker `temp_roles::expire_temp_roles` (sentinel-worker).
///
/// Avant : ce module avait sa propre boucle 60s qui scannait un
/// `TempRoleTracker` in-memory. Probleme : si le bot redemarrait, le
/// tracker etait reconstruit depuis l'API mais les expirations en cours
/// pendant le restart etaient ratees.
///
/// Maintenant : le worker scanne la DB (source de verite) et publie un
/// event a chaque expiration. Le bot consume et execute le retrait
/// Discord. Resilient aux redemarrages.
pub fn spawn_temp_role_cleanup(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "community-bot-temp-role-expire".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move {
                    handle_temp_role_expire(&ctx, &payload_json).await;
                }
            },
        )
        .await;
    });
}

async fn handle_temp_role_expire(ctx: &Context, payload_json: &str) {
    let event: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if event.get("event").and_then(|v| v.as_str()) != Some("temp_role_expire") {
        return;
    }
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };
    let guild_id_str = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let user_id_str = data.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
    let role_id_str = data.get("role_id").and_then(|v| v.as_str()).unwrap_or("");
    if guild_id_str.is_empty() || user_id_str.is_empty() || role_id_str.is_empty() {
        return;
    }

    let guild_id_u64: u64 = match guild_id_str.parse() {
        Ok(v) => v,
        Err(_) => return,
    };
    let user_id_u64: u64 = match user_id_str.parse() {
        Ok(v) => v,
        Err(_) => return,
    };
    let role_id_u64: u64 = match role_id_str.parse() {
        Ok(v) => v,
        Err(_) => return,
    };

    let guild_id = serenity::model::id::GuildId::new(guild_id_u64);
    let user_id = serenity::model::id::UserId::new(user_id_u64);
    let role_id = RoleId::new(role_id_u64);

    if let Ok(member) = guild_id.member(&ctx.http, user_id).await {
        if member.remove_role(&ctx.http, role_id).await.is_ok() {
            info!(guild = %guild_id_str, user = %user_id_str, role = %role_id_str, "Role temporaire retire (event)");
        }
    }

    // Cleanup tracker in-memory (defensif, plus la source de verite).
    let bot_data = ctx.data.read().await;
    if let Some(tracker) = bot_data.get::<TempRoleKey>() {
        tracker.remove(guild_id_u64, user_id_u64, role_id_u64);
    }
    if let Some(api) = bot_data.get::<RolesApiKey>() {
        api.delete_temp_role(guild_id_str, user_id_str, role_id_str)
            .await;
    }
}

/// Synchronise les roles Discord de toutes les guilds vers l'API.
pub async fn sync_all_guild_roles(ctx: &Context) {
    let data = ctx.data.read().await;
    let api = match data.get::<RolesApiKey>() {
        Some(a) => a,
        None => return,
    };

    let guilds = ctx.cache.guilds();
    for guild_id in guilds {
        let roles = match guild_id.roles(&ctx.http).await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, guild = %guild_id, "Erreur recuperation roles Discord");
                continue;
            }
        };

        let sync_roles: Vec<SyncRole> = roles
            .values()
            .map(|r| SyncRole {
                id: r.id.to_string(),
                name: r.name.clone(),
                color: r.colour.0 as i32,
                position: r.position as i32,
                permissions: r.permissions.bits().to_string(),
                mentionable: r.mentionable,
                managed: r.managed,
                icon: r.icon.as_ref().map(|i| i.to_string()),
                member_count: 0,
            })
            .collect();

        let count = sync_roles.len();
        if let Err(e) = api
            .sync_discord_roles(&guild_id.to_string(), sync_roles)
            .await
        {
            warn!(error = %e, guild = %guild_id, "Erreur sync roles vers API");
        } else {
            info!(guild = %guild_id, roles = count, "Roles Discord synchronises");
        }
    }
}

// ── Helpers ──

/// Envoie un panel de roles dans un channel avec des boutons.
pub async fn send_role_panel(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    panel: &RolePanelDetail,
) -> Result<Message, serenity::Error> {
    let mut embed = CreateEmbed::new().title(&panel.panel.title).color(0x5865F2);

    if !panel.panel.description.is_empty() {
        embed = embed.description(&panel.panel.description);
    }

    let mut desc_parts = Vec::new();
    for entry in &panel.entries {
        let emoji = entry.emoji.as_deref().unwrap_or("");
        desc_parts.push(format!("{} **{}**", emoji, entry.label));
    }
    if !desc_parts.is_empty() {
        embed = embed.description(desc_parts.join("\n"));
    }

    let buttons: Vec<CreateButton> = panel
        .entries
        .iter()
        .map(|entry| {
            let style = match entry.style.as_str() {
                "secondary" => ButtonStyle::Secondary,
                "success" => ButtonStyle::Success,
                "danger" => ButtonStyle::Danger,
                _ => ButtonStyle::Primary,
            };
            let mut btn = CreateButton::new(format!("role_{}", entry.role_id))
                .label(&entry.label)
                .style(style);
            if let Some(ref emoji) = entry.emoji {
                if let Ok(e) = emoji.parse::<serenity::model::channel::ReactionType>() {
                    btn = btn.emoji(e);
                }
            }
            btn
        })
        .collect();

    let rows: Vec<CreateActionRow> = buttons
        .chunks(5)
        .map(|chunk| CreateActionRow::Buttons(chunk.to_vec()))
        .collect();

    channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(rows),
        )
        .await
}

/// Gere le clic sur un bouton de role (toggle).
async fn handle_role_button(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = &component.data.custom_id;
    let role_id_str = custom_id.strip_prefix("role_").unwrap_or("");
    let role_id: u64 = match role_id_str.parse() {
        Ok(id) => id,
        Err(_) => return,
    };

    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };

    // Rate limit anti-spam (cooldown configurable per-guild)
    {
        let data = ctx.data.read().await;
        let cooldown_secs = if let Some(base) = data.get::<ApiClientKey>() {
            let gc = base
                .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
                .await
                .unwrap_or_default();
            BaseApiClient::config_u64(&gc, "role_button_cooldown_secs", 2)
        } else {
            2
        };
        if let Some(cooldown) = data.get::<CooldownKey>() {
            let key = format!("role_{}", role_id);
            if let Some(remaining) =
                cooldown.check_and_set(component.user.id.get(), &key, cooldown_secs)
            {
                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "\u{23f1}\u{fe0f} Calme-toi un peu... attends {remaining}s avant de refaire cette action."
                        ))
                        .ephemeral(true),
                );
                let _ = component.create_response(&ctx.http, response).await;
                return;
            }
        }
    }

    let member = match guild_id.member(&ctx.http, component.user.id).await {
        Ok(m) => m,
        Err(_) => return,
    };

    let role = RoleId::new(role_id);
    let has_role = member.roles.contains(&role);

    let guild_config = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            match base
                .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for role button");
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        }
    };

    // GARDE ANTI-ESCALADE : on n'ATTRIBUE jamais un role privilegie / managed /
    // introuvable via un panneau self-service — meme si un admin l'a mis dans le
    // panneau, meme si le custom_id est forge. Discord ne bloque que les roles
    // AU-DESSUS du bot ; sans cette garde, tout role privilegie SOUS le bot
    // serait auto-attribuable par n'importe quel membre (escalade). Ne s'applique
    // qu'a l'ajout (retirer un role privilegie reste permis = de-escalade).
    if !has_role {
        let dangerous = serenity::all::Permissions::ADMINISTRATOR
            | serenity::all::Permissions::MANAGE_GUILD
            | serenity::all::Permissions::MANAGE_ROLES
            | serenity::all::Permissions::MANAGE_CHANNELS
            | serenity::all::Permissions::MANAGE_WEBHOOKS
            | serenity::all::Permissions::BAN_MEMBERS
            | serenity::all::Permissions::KICK_MEMBERS
            | serenity::all::Permissions::MODERATE_MEMBERS
            | serenity::all::Permissions::MANAGE_MESSAGES
            | serenity::all::Permissions::MENTION_EVERYONE
            | serenity::all::Permissions::MANAGE_NICKNAMES
            | serenity::all::Permissions::MANAGE_THREADS
            | serenity::all::Permissions::MANAGE_EVENTS;
        let safe = ctx
            .cache
            .guild(guild_id)
            .map(|g| match g.roles.get(&role) {
                Some(r) => !r.managed && (r.permissions & dangerous).is_empty(),
                None => false, // role introuvable en cache -> refus (fail-closed)
            })
            .unwrap_or(false);
        if !safe {
            warn!(role = %role_id, user = %component.user.id, "Refus attribution d'un role privilegie/introuvable via panneau");
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(
                        neutral_embed("Role non attribuable")
                            .description("Ce role ne peut pas etre attribue via ce panneau."),
                    )
                    .ephemeral(true),
            );
            let _ = component.create_response(&ctx.http, response).await;
            return;
        }
    }

    let embed = if has_role {
        if let Ok(m) = guild_id.member(&ctx.http, component.user.id).await {
            if let Err(e) = m.remove_role(&ctx.http, role).await {
                warn!(error = %e, "Failed to remove role");
            }
        }
        neutral_embed("\u{21a9}\u{fe0f} Role retire")
            .description(format!("Le role <@&{}> vous a ete retire.", role_id))
    } else {
        // Verifier les prerequis : DECISION server-side. Le bot ne fournit que
        // les donnees Discord (roles actuels + date de join) ; l'API lit la
        // config (`role_prerequisites`) et evalue les regles.
        let user_roles: Vec<u64> = member.roles.iter().map(|r| r.get()).collect();
        let joined_at_unix = member.joined_at.map(|j| j.unix_timestamp());

        let decision = {
            let data = ctx.data.read().await;
            match data.get::<RolesApiKey>() {
                Some(api) => api
                    .check_role_eligibility(
                        &guild_id.to_string(),
                        role_id,
                        user_roles,
                        joined_at_unix,
                    )
                    .await
                    .unwrap_or_else(|e| {
                        // Fail-closed : en cas d'erreur API, on refuse l'ajout.
                        warn!(error = %e, "Echec API check_role_eligibility — refus");
                        crate::modules::community::api_client::EligibilityDecision {
                            allowed: false,
                            reason: Some(
                                "Verification des prerequis indisponible, reessaie plus tard."
                                    .to_string(),
                            ),
                        }
                    }),
                None => crate::modules::community::api_client::EligibilityDecision {
                    allowed: true,
                    reason: None,
                },
            }
        };

        if !decision.allowed {
            let msg = decision
                .reason
                .unwrap_or_else(|| "Prerequis non remplis.".to_string());
            let embed = neutral_embed("Prerequis non remplis").description(msg);
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send prerequisite check response");
            }
            return;
        }

        // Temp roles
        let temp_raw = BaseApiClient::config_or(&guild_config, "temp_roles", "");
        let temp_roles_config = temp_roles::parse_temp_roles(&temp_raw);
        let temp_duration = temp_roles::get_temp_duration(&temp_roles_config, role_id);

        if let Some(duration) = temp_duration {
            let expires_at =
                (chrono::Utc::now() + chrono::Duration::seconds(duration as i64)).to_rfc3339();
            let api_result = {
                let data = ctx.data.read().await;
                if let Some(api) = data.get::<RolesApiKey>() {
                    api.create_temp_role(
                        &guild_id.to_string(),
                        &component.user.id.to_string(),
                        &role_id.to_string(),
                        &expires_at,
                    )
                    .await
                } else {
                    Err("ApiClient indisponible".to_string())
                }
            };

            if let Err(e) = api_result {
                warn!(error = %e, "Echec persistance temp_role — abort");
                let embed = neutral_embed("Erreur")
                    .description("Impossible d'enregistrer le role temporaire cote serveur.");
                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                );
                let _ = component.create_response(&ctx.http, response).await;
                return;
            }

            let data = ctx.data.read().await;
            if let Some(tracker) = data.get::<TempRoleKey>() {
                tracker.add(guild_id.get(), component.user.id.get(), role_id, duration);
            }
        }

        // Exclusive groups
        let groups_raw = BaseApiClient::config_or(&guild_config, "exclusive_groups", "");
        let groups = exclusive_groups::parse_groups(&groups_raw);
        let conflicts = exclusive_groups::get_conflicting_roles(&groups, role_id);
        if !conflicts.is_empty() {
            if let Ok(m) = guild_id.member(&ctx.http, component.user.id).await {
                for conflict_id in &conflicts {
                    if let Err(e) = m.remove_role(&ctx.http, RoleId::new(*conflict_id)).await {
                        warn!(error = %e, conflict_role = %conflict_id, "Failed to remove conflicting role");
                    }
                }
            }
        }

        // Ajouter le role
        if let Ok(m) = guild_id.member(&ctx.http, component.user.id).await {
            if let Err(e) = m.add_role(&ctx.http, role).await {
                warn!(error = %e, "Failed to add role");
                if temp_duration.is_some() {
                    let data = ctx.data.read().await;
                    if let Some(tracker) = data.get::<TempRoleKey>() {
                        tracker.remove(guild_id.get(), component.user.id.get(), role_id);
                    }
                    if let Some(api) = data.get::<RolesApiKey>() {
                        api.delete_temp_role(
                            &guild_id.to_string(),
                            &component.user.id.to_string(),
                            &role_id.to_string(),
                        )
                        .await;
                    }
                }
            }
        }

        let mut desc = format!("Le role <@&{}> vous a ete attribue.", role_id);
        if !conflicts.is_empty() {
            desc.push_str("\n*(roles exclusifs retires automatiquement)*");
        }
        success_embed("\u{2705} Role attribue").description(desc)
    };

    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(msg);
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Failed to send role toggle response");
    }
}
