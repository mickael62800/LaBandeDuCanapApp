//! Module progression — XP, levels, streaks (ex progression-bot).

pub const MODULE_BOT_NAME: &str = "progression-bot";

pub mod api_client;
pub mod classement_cmd;
pub mod level_channel;
pub mod level_cmd;
pub mod nickname;
pub mod resync_cmd;
pub mod role_tiers;
pub mod stats_cmd;
pub mod tracker;

use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{CommandInteraction, Context, CreateCommand, CreateMessage};
use serenity::model::channel::Message;
use serenity::model::guild::Member;
use serenity::model::id::{ChannelId, RoleId, UserId};
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::{
    guild_config_or_default, is_module_enabled, is_module_enabled_or_reply_command,
};
use crate::shared::embeds::success_embed;
use crate::shared::heartbeat::ApiClientKey;

use api_client::ApiClient;
use tracker::StatsTracker;

// ── TypeMapKeys ──

pub struct StatsApiKey;
impl TypeMapKey for StatsApiKey {
    type Value = ApiClient;
}

pub struct TrackerKey;
impl TypeMapKey for TrackerKey {
    type Value = StatsTracker;
}

// ── Init TypeMapKeys ──

/// Insere les TypeMapKeys du module progression dans le TypeMap partage.
///
/// Depuis le refactor P0, le bot n'accumule plus d'etat de calcul XP
/// (cooldown, streak) : ces logiques sont remontees dans l'API. Il ne reste
/// que le tracker de SESSION vocale (secondes brutes) et le client API.
pub fn init_typemap(
    data: &mut serenity::prelude::TypeMap,
    grpc: &Arc<crate::shared::grpc_client::SentinelGrpcClient>,
) {
    data.insert::<StatsApiKey>(api_client::ApiClient::new(Arc::clone(grpc)));
    data.insert::<TrackerKey>(tracker::StatsTracker::new());
}

/// Hydrate les sessions vocales depuis les voice_states Discord apres le boot.
/// Les users deja en vocal au restart du bot ne perdent pas leur temps.
pub async fn on_ready(ctx: &Context, ready: &serenity::model::gateway::Ready) {
    let data = ctx.data.read().await;
    let Some(tracker) = data.get::<TrackerKey>().cloned() else {
        return;
    };
    drop(data);

    let mut hydrated = 0usize;
    for guild in &ready.guilds {
        let entries: Vec<(u64, u64, bool)> = match ctx.cache.guild(guild.id) {
            Some(g) => g
                .voice_states
                .iter()
                .filter_map(|(uid, st)| {
                    st.channel_id
                        .map(|ch| (uid.get(), ch.get(), st.self_mute && st.self_deaf))
                })
                .collect(),
            None => continue,
        };
        for (user_id, channel_id, is_afk) in entries {
            tracker
                .hydrate(guild.id.get(), user_id, channel_id, is_afk)
                .await;
            hydrated += 1;
        }
    }
    info!(hydrated, "progression: sessions vocales hydratees au ready");
}

/// Helper level-up : applique max_level cap, custom template, toggle
/// announce, toggle DM. Appele pour text et voice level-ups.
///
/// Variables disponibles dans `levelup_message` :
///   {user}     -> mention <@id>
///   {level}    -> niveau atteint
///   {kind}     -> "texte" ou "vocal"
async fn announce_level_up(
    ctx: &Context,
    guild_config: &HashMap<String, String>,
    user_id: u64,
    level: i32,
    kind: &str,        // "texte" | "vocal"
    title_emoji: &str, // emoji du title
    // Salon declencheur (message). Utilise en fallback quand aucun salon
    // d'annonce n'est configure (le schema promet "poste dans le salon
    // courant"). None pour le vocal : pas de salon courant.
    current_channel_id: Option<ChannelId>,
) {
    let max_level = BaseApiClient::config_u64(guild_config, "max_level", 0) as i32;
    if max_level > 0 && level > max_level {
        return;
    }

    let template = BaseApiClient::config_or(guild_config, "levelup_message", "");
    let description = if template.is_empty() {
        format!(
            "<@{}> est maintenant **niveau {} en {}** !",
            user_id, level, kind
        )
    } else {
        template
            .replace("{user}", &format!("<@{}>", user_id))
            .replace("{level}", &level.to_string())
            .replace("{kind}", kind)
    };

    let embed = success_embed(format!("{} LEVEL UP {} !", title_emoji, capitalize(kind)))
        .description(&description);

    let announce_enabled =
        BaseApiClient::config_bool(guild_config, "levelup_announce_enabled", true);
    if announce_enabled {
        // Salon configure, sinon fallback sur le salon courant (cf. schema :
        // "Si vide, l'annonce est postee dans le salon courant").
        let target = level_channel::resolve_level_up_channel(guild_config)
            .map(ChannelId::new)
            .or(current_channel_id);
        if let Some(target) = target {
            if let Err(e) = target
                .send_message(&ctx.http, CreateMessage::new().embed(embed.clone()))
                .await
            {
                warn!(error = %e, kind, "Failed to send level-up channel message");
            }
        }
    }

    let dm_enabled = BaseApiClient::config_bool(guild_config, "levelup_dm_enabled", false);
    if dm_enabled {
        let user = serenity::model::id::UserId::new(user_id);
        if let Ok(dm) = user.create_dm_channel(&ctx.http).await {
            if let Err(e) = dm
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await
            {
                warn!(error = %e, kind, "Failed to send level-up DM");
            }
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

pub fn spawn_voice_tick(ctx: Context) {
    use tokio::time::{interval, Duration};
    // Granularite du timer global qui credite l'XP vocal de TOUS les serveurs.
    // Ce n'est PAS un levier d'economie (le taux reel est gouverne par le
    // reglage per-serveur `xp_per_voice_minute`, le credit etant proportionnel
    // aux secondes reelles ecoulees) : le tick ne fait que fixer la frequence de
    // credit. Il reste donc bot-level, configurable via l'env `VOICE_XP_TICK_SECS`
    // (defaut 300). Borne a >= 1s.
    let tick_secs = std::env::var("VOICE_XP_TICK_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&s| s >= 1)
        .unwrap_or(300);
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(tick_secs));
        tick.tick().await; // skip first immediate tick
        loop {
            tick.tick().await;
            if let Err(e) = credit_voice_tick(&ctx).await {
                warn!(error = %e, "progression: erreur credit voice tick");
            }
        }
    });
}

async fn credit_voice_tick(ctx: &Context) -> Result<(), String> {
    let (credits, base) = {
        let data = ctx.data.read().await;
        let Some(tracker) = data.get::<TrackerKey>().cloned() else {
            return Ok(());
        };
        let Some(base) = data.get::<ApiClientKey>().cloned() else {
            return Ok(());
        };
        drop(data);
        let credits = tracker.credit_active_sessions().await;
        (credits, base)
    };
    if credits.is_empty() {
        return Ok(());
    }

    // On garde `base` uniquement pour eventuels usages futurs ; le calcul XP
    // (config serveur, multiplicateurs) est desormais entierement cote API.
    let _ = &base;

    for (guild_id, user_id, seconds, channel_id) in credits {
        if seconds == 0 {
            continue;
        }
        // Roles de l'utilisateur (fait brut envoye a l'API pour ses multiplicateurs).
        let role_ids: Vec<u64> = ctx
            .cache
            .guild(serenity::model::id::GuildId::new(guild_id))
            .and_then(|g| {
                g.members
                    .get(&UserId::new(user_id))
                    .map(|m| m.roles.iter().map(|r| r.get()).collect())
            })
            .unwrap_or_default();

        let username = UserId::new(user_id)
            .to_user(&ctx.http)
            .await
            .map(|u| u.name)
            .unwrap_or_else(|_| user_id.to_string());

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<StatsApiKey>() {
            // FAIT BRUT : N secondes vocales dans le salon. L'API calcule l'XP.
            if let Err(e) = api
                .record_voice_activity(
                    &guild_id.to_string(),
                    &user_id.to_string(),
                    &username,
                    channel_id,
                    &role_ids,
                    seconds,
                )
                .await
            {
                warn!(error = %e, guild = %guild_id, user = %user_id, "progression: record_voice_activity tick echoue");
            }
        }
        drop(data);
    }
    Ok(())
}

// ── Slash commands ──

pub fn register_commands() -> Vec<CreateCommand> {
    vec![
        level_cmd::register(),
        stats_cmd::register(),
        resync_cmd::register(),
        classement_cmd::register(),
    ]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    match command.data.name.as_str() {
        "level" => level_cmd::handle(ctx, command).await,
        "stats" => stats_cmd::handle(ctx, command).await,
        "progression-resync" => resync_cmd::handle(ctx, command).await,
        "classement" => classement_cmd::handle(ctx, command).await,
        _ => {}
    }
}

// ── Event handlers (free functions) ──

/// Appele sur chaque message — XP texte, streaks, multipliers, level-up.
pub async fn on_message(ctx: &Context, msg: &Message) {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return,
    };

    // Les bots et les webhooks ne gagnent pas d'XP : un bot present h24
    // (musique, outil de moderation) grimperait sinon en niveau sans fin, et
    // fausserait le classement. Exclusion la plus en amont possible.
    if msg.author.bot || msg.webhook_id.is_some() {
        return;
    }

    let guild_config = guild_config_or_default(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await;
    if !BaseApiClient::config_bool(&guild_config, "enabled", false) {
        return;
    }

    let min_len = BaseApiClient::config_u64(&guild_config, "min_message_length", 3) as usize;
    if msg.content.chars().count() < min_len {
        return;
    }

    let ignored_channels_csv = BaseApiClient::config_or(&guild_config, "ignored_channels", "");
    if !ignored_channels_csv.is_empty() {
        let cid = msg.channel_id.get().to_string();
        if ignored_channels_csv.split(',').any(|s| s.trim() == cid) {
            return;
        }
    }

    let ignored_roles_csv = BaseApiClient::config_or(&guild_config, "ignored_roles", "");
    if !ignored_roles_csv.is_empty() {
        if let Some(member) = msg.member.as_ref() {
            let user_role_ids: std::collections::HashSet<String> =
                member.roles.iter().map(|r| r.get().to_string()).collect();
            let has_ignored = ignored_roles_csv
                .split(',')
                .map(|s| s.trim())
                .any(|s| user_role_ids.contains(s));
            if has_ignored {
                return;
            }
        }
    }

    let data = ctx.data.read().await;

    if let Some(tracker) = data.get::<TrackerKey>() {
        tracker
            .record_message(guild_id.get(), msg.author.id.get())
            .await;
    }

    let api = match data.get::<StatsApiKey>() {
        Some(a) => a,
        None => return,
    };

    if let Err(e) = api
        .record_messages(
            &guild_id.to_string(),
            &msg.author.id.to_string(),
            &msg.author.name,
            1,
        )
        .await
    {
        warn!(error = %e, "Impossible d'envoyer les stats messages au backend");
    }

    // FAIT BRUT : "un message qualifiant a eu lieu". L'API calcule tout l'XP
    // (cooldown anti-farm, streak, multiplicateurs channel/role, clamp).
    let user_roles: Vec<u64> = msg
        .member
        .as_ref()
        .map(|m| m.roles.iter().map(|r| r.get()).collect())
        .unwrap_or_default();

    match api
        .record_text_activity(
            &guild_id.to_string(),
            &msg.author.id.to_string(),
            &msg.author.name,
            msg.channel_id.get(),
            &user_roles,
        )
        .await
    {
        Ok(result) => {
            if result.skipped {
                return;
            }
            if result.leveled_up {
                announce_level_up(
                    ctx,
                    &guild_config,
                    msg.author.id.get(),
                    result.user.level_text,
                    "texte",
                    "\u{1f4dd}",
                    Some(msg.channel_id),
                )
                .await;
            }
            // Paliers de roles uniquement quand le niveau global change. Plus de
            // renommage `[NN]` : le prefixe de niveau a ete retire.
            if result.user.level > result.old_level_global {
                drop(data);
                role_tiers::appliquer_paliers(ctx, guild_id, msg.author.id, result.user.level)
                    .await;
            }
        }
        Err(e) => {
            tracing::debug!(error = %e, "Erreur record_text_activity");
        }
    }
}

/// Appele sur voice_state_update — XP vocal, suivi sessions.
pub async fn on_voice_state_update(ctx: &Context, old: Option<VoiceState>, new: &VoiceState) {
    let guild_id = match new.guild_id {
        Some(id) => id,
        None => return,
    };

    let user_id = new.user_id;

    if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
        return;
    }

    // Exclusion XP vocal, en miroir du texte : les bots (musique, soundboard)
    // presents en permanence, et les membres portant un role ignore, ne
    // gagnent pas d'XP. Le membre vient de l'evenement (present pour les etats
    // vocaux de guilde), avec repli sur le cache.
    let membre = new
        .member
        .clone()
        .or_else(|| old.as_ref().and_then(|s| s.member.clone()));
    let est_bot = membre
        .as_ref()
        .map(|m| m.user.bot)
        .or_else(|| {
            ctx.cache
                .guild(guild_id)
                .and_then(|g| g.members.get(&user_id).map(|m| m.user.bot))
        })
        .unwrap_or(false);
    if est_bot {
        return;
    }
    let roles_membre: Vec<u64> = membre
        .as_ref()
        .map(|m| m.roles.iter().map(|r| r.get()).collect())
        .unwrap_or_default();
    if !roles_membre.is_empty() {
        let cfg = guild_config_or_default(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await;
        let ignored_roles_csv = BaseApiClient::config_or(&cfg, "ignored_roles", "");
        if !ignored_roles_csv.is_empty() {
            let porte: std::collections::HashSet<String> =
                roles_membre.iter().map(|r| r.to_string()).collect();
            let a_role_ignore = ignored_roles_csv
                .split(',')
                .map(|s| s.trim())
                .any(|s| porte.contains(s));
            if a_role_ignore {
                return;
            }
        }
    }

    let data = ctx.data.read().await;

    let was_in_voice = old.as_ref().and_then(|s| s.channel_id).is_some();
    let is_in_voice = new.channel_id.is_some();

    let is_afk_now = new.self_mute && new.self_deaf;
    let was_afk = old
        .as_ref()
        .map(|s| s.self_mute && s.self_deaf)
        .unwrap_or(false);

    let tracker = data.get::<TrackerKey>();
    let api = data.get::<StatsApiKey>();

    match (was_in_voice, is_in_voice) {
        (false, true) => {
            if let Some(tracker) = tracker {
                let ch = new.channel_id.map(|c| c.get()).unwrap_or(0);
                tracker
                    .voice_join(guild_id.get(), user_id.get(), ch, is_afk_now)
                    .await;
            }
        }
        (true, true) => {
            if was_afk != is_afk_now {
                if let Some(tracker) = tracker {
                    tracker
                        .set_voice_afk(guild_id.get(), user_id.get(), is_afk_now)
                        .await;
                }
            }
            let old_ch = old.as_ref().and_then(|s| s.channel_id);
            if old_ch != new.channel_id {
                if let Some(tracker) = tracker {
                    let ch = new.channel_id.map(|c| c.get()).unwrap_or(0);
                    tracker
                        .set_voice_channel(guild_id.get(), user_id.get(), ch)
                        .await;
                }
            }
        }
        (true, false) => {
            if let Some(tracker) = tracker {
                let seconds = tracker.voice_leave(guild_id.get(), user_id.get()).await;

                if seconds > 0 {
                    let username = user_id
                        .to_user(&ctx.http)
                        .await
                        .map(|u| u.name)
                        .unwrap_or_else(|_| user_id.to_string());

                    let (channel_id_str, channel_name) = if let Some(old_state) = &old {
                        if let Some(ch_id) = old_state.channel_id {
                            let name = ch_id
                                .to_channel(&ctx.http)
                                .await
                                .ok()
                                .and_then(|c| c.guild())
                                .map(|c| c.name.clone())
                                .unwrap_or_default();
                            (ch_id.to_string(), name)
                        } else {
                            (String::new(), String::new())
                        }
                    } else {
                        (String::new(), String::new())
                    };

                    if let Some(api) = api {
                        if let Err(e) = api
                            .record_voice(
                                &guild_id.to_string(),
                                &user_id.to_string(),
                                &username,
                                seconds,
                                &channel_id_str,
                                &channel_name,
                            )
                            .await
                        {
                            warn!(error = %e, "Impossible d'envoyer les stats vocal au backend");
                        }

                        // FAIT BRUT : `seconds` secondes vocales dans le salon
                        // quitte + roles de l'utilisateur. L'API calcule l'XP.
                        let ch_id_u64 = channel_id_str.parse::<u64>().unwrap_or(0);
                        let role_ids: Vec<u64> = old
                            .as_ref()
                            .and_then(|s| s.member.as_ref())
                            .map(|m| m.roles.iter().map(|r| r.get()).collect())
                            .unwrap_or_default();
                        match api
                            .record_voice_activity(
                                &guild_id.to_string(),
                                &user_id.to_string(),
                                &username,
                                ch_id_u64,
                                &role_ids,
                                seconds,
                            )
                            .await
                        {
                            Ok(result) => {
                                if !result.skipped {
                                    if result.leveled_up {
                                        let voice_guild_config =
                                            if let Some(base) = data.get::<ApiClientKey>() {
                                                base.get_guild_config_for(
                                                    &guild_id.to_string(),
                                                    MODULE_BOT_NAME,
                                                )
                                                .await
                                                .unwrap_or_default()
                                            } else {
                                                HashMap::new()
                                            };
                                        announce_level_up(
                                            ctx,
                                            &voice_guild_config,
                                            user_id.get(),
                                            result.user.level_voice,
                                            "vocal",
                                            "\u{1f3a4}",
                                            None,
                                        )
                                        .await;
                                    }
                                    if result.user.level > result.old_level_global {
                                        let new_level = result.user.level;
                                        drop(data);
                                        role_tiers::appliquer_paliers(
                                            ctx, guild_id, user_id, new_level,
                                        )
                                        .await;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::debug!(error = %e, "Erreur record_voice_activity");
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Attribue les roles par defaut au nouveau membre (config guild).
/// La config `default_role_ids` est une liste d IDs de roles separes par
/// des virgules ; chaque ID valide est attribue dans l ordre.
pub async fn assign_default_role(ctx: &Context, new_member: &Member) {
    let guild_id = new_member.guild_id;

    let data = ctx.data.read().await;
    let role_ids: Vec<u64> = if let Some(base) = data.get::<ApiClientKey>() {
        let config = match base
            .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
            .await
        {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                HashMap::new()
            }
        };
        let raw = BaseApiClient::config_or(&config, "default_role_ids", "");
        raw.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<u64>().ok())
            .collect()
    } else {
        Vec::new()
    };
    drop(data);

    for role_id in role_ids {
        match new_member.add_role(&ctx.http, RoleId::new(role_id)).await {
            Ok(_) => {
                info!(guild=%guild_id, user=%new_member.user.id, role=%role_id, "Role par defaut attribue")
            }
            Err(e) => {
                warn!(guild=%guild_id, user=%new_member.user.id, error=%e, "Echec attribution role par defaut")
            }
        }
    }
}

pub mod leaderboard_render;
