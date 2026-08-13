//! Creation et suppression des salons vocaux temporaires.
//!
//! Un salon temporaire = un vocal unique dont le panneau admin est poste
//! dans le chat integre du vocal (text-in-voice natif Discord). Plus de
//! categorie ni de salon texte separe. Pour les salons `game`, une file
//! d'attente (vocal secondaire) est creee en parallele.

use serenity::all::ButtonStyle;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateChannel, CreateEmbed, CreateMessage, CreateSelectMenu,
    CreateSelectMenuKind,
};
use serenity::model::channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType};
use serenity::model::id::{ChannelId, GuildId, UserId};
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::shared::heartbeat::ApiClientKey;

use crate::modules::voice::api_client::{
    ApiClient, CreateVoiceChannelRequest, UpdateVoiceChannelRequest,
};
use crate::modules::voice::embeds;
use crate::modules::voice::{CooldownTrackerKey, VoiceOwnerMapKey};

/// Cree un salon vocal temporaire (et sa queue si `kind == "game"`), deplace
/// l'utilisateur dedans et poste le panneau admin dans le chat integre du
/// vocal. `kind` = `"public"`, `"private"` ou `"game"`.
pub(super) async fn create_temp_channel(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    kind: &str,
) {
    // Cooldown check (anti-spam creation)
    {
        let data = ctx.data.read().await;
        if let Some(cooldowns) = data.get::<CooldownTrackerKey>() {
            if let Some(remaining) = cooldowns.check_and_set(user_id) {
                tracing::info!(user = %user_id, remaining = remaining, "Cooldown actif, creation ignoree");
                return;
            }
        }
    }

    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(_) => return,
    };
    let display_name = member.display_name().to_string();

    // Preset memorise par ce proprietaire (bouton "Sauvegarder params"). Sert
    // a reappliquer nom / limite / visibilite / verrou + whitelist a chaque
    // nouvelle creation. None si l'utilisateur n'a jamais sauvegarde.
    let preset = {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            api.get_preset(&guild_id.to_string(), &user_id.get().to_string())
                .await
        } else {
            None
        }
    };

    // Nom du vocal : preset si defini, sinon prefix special pour les game.
    let voice_name = match preset.as_ref().and_then(|p| p.channel_name.clone()) {
        Some(name) if !name.trim().is_empty() => name,
        _ if kind == "game" => format!("\u{1f3ae} {display_name}"),
        _ => format!("Salon de {display_name}"),
    };
    let everyone_role = guild_id.everyone_role();
    // user_limit : preset prioritaire, sinon theme API, sinon 0 (illimite).
    let theme_user_limit: u32 = {
        let data = ctx.data.read().await;
        data.get::<crate::modules::voice::ThemeCacheKey>()
            .and_then(|themes| {
                themes
                    .iter()
                    .find(|t| t.name == kind)
                    .and_then(|t| t.member_limit)
            })
            .unwrap_or(0) as u32
    };
    let default_user_limit: u32 = preset
        .as_ref()
        .and_then(|p| p.member_limit)
        .map(|l| l.max(0) as u32)
        .unwrap_or(theme_user_limit);

    // Lire la categorie ancre depuis la config guild (pour le positionnement).
    let anchor_category_id: Option<u64> = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            base.get_guild_config_for(
                &guild_id.to_string(),
                crate::modules::voice::MODULE_BOT_NAME,
            )
            .await
            .ok()
            .and_then(|cfg| {
                cfg.get("voice_anchor_category_id")
                    .and_then(|v| v.parse::<u64>().ok())
                    .filter(|id| *id > 0)
            })
        } else {
            None
        }
    };

    // 1. Creer le salon vocal. Si voice_anchor_category_id est configure,
    // on place le salon DANS cette categorie (Discord le met automatiquement
    // en bas de la categorie). Sinon, salon a la racine du serveur.
    let mut voice_builder = CreateChannel::new(&voice_name).kind(ChannelType::Voice);
    if default_user_limit > 0 {
        voice_builder = voice_builder.user_limit(default_user_limit);
    }
    if let Some(cat_id) = anchor_category_id {
        voice_builder = voice_builder.category(ChannelId::new(cat_id));
    }
    let voice_channel = match guild_id.create_channel(&ctx.http, voice_builder).await {
        Ok(ch) => ch,
        Err(why) => {
            error!(error = %why, "Erreur creation salon vocal");
            return;
        }
    };
    let voice_channel_id = voice_channel.id;

    // Permissions owner sur le vocal (inclut SEND_MESSAGES pour le chat integre).
    let owner_perm = PermissionOverwrite {
        allow: Permissions::CONNECT
            | Permissions::VIEW_CHANNEL
            | Permissions::SPEAK
            | Permissions::SEND_MESSAGES
            | Permissions::MOVE_MEMBERS
            | Permissions::MUTE_MEMBERS
            | Permissions::DEAFEN_MEMBERS
            | Permissions::MANAGE_CHANNELS,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(user_id),
    };
    if let Err(e) = voice_channel_id
        .create_permission(&ctx.http, owner_perm)
        .await
    {
        tracing::warn!(error = %e, "failed to set owner permission on voice channel");
    }

    // ── Application du preset (visibilite/verrou) + whitelist ──
    // Pour les salons `game`, la file d'attente gere deja les overwrites
    // @everyone (verrou derriere la queue) : on ne superpose pas le preset.
    let preset_hidden = preset
        .as_ref()
        .map(|p| p.visibility == "hidden")
        .unwrap_or(false);
    let preset_locked = preset.as_ref().map(|p| p.locked).unwrap_or(false);

    // Un salon 'private' SANS preset sauvegarde doit etre prive PAR DEFAUT (deny
    // @everyone). Avant, faute de preset, aucun deny n'etait pose -> le salon
    // "prive" etait en fait connectable/visible par tout le monde.
    let private_default = kind == "private" && preset.is_none();
    let deny_hidden = preset_hidden || private_default;
    let deny_locked = preset_locked || private_default;

    if kind != "game" && (deny_hidden || deny_locked) {
        let mut deny = Permissions::empty();
        if deny_hidden {
            deny |= Permissions::VIEW_CHANNEL | Permissions::CONNECT;
        }
        if deny_locked {
            deny |= Permissions::CONNECT;
        }
        let everyone_overwrite = PermissionOverwrite {
            allow: Permissions::empty(),
            deny,
            kind: PermissionOverwriteType::Role(everyone_role),
        };
        if let Err(e) = voice_channel_id
            .create_permission(&ctx.http, everyone_overwrite)
            .await
        {
            warn!(error = %e, "failed to apply preset @everyone overwrite");
        }
    }

    // Whitelist (liste d'amis persistante) : chaque membre whiteliste obtient
    // l'acces, ce qui le rend visible meme si le salon est cache. Tolerant aux
    // erreurs (rate-limit Discord) : on log et on continue.
    let whitelist = {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            api.get_whitelist(&guild_id.to_string(), &user_id.get().to_string())
                .await
        } else {
            Vec::new()
        }
    };
    for entry in &whitelist {
        let Ok(target) = entry.target_id.parse::<u64>() else {
            continue;
        };
        let overwrite = PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT | Permissions::SPEAK,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(UserId::new(target)),
        };
        if let Err(e) = voice_channel_id
            .create_permission(&ctx.http, overwrite)
            .await
        {
            warn!(error = %e, target = %target, "failed to apply whitelist overwrite");
        }
    }

    // Bans persistants (issue #2) : un ban est memorise par (guild, owner,
    // banned_user) et doit etre re-applique a chaque recreation du salon de ce
    // proprietaire, sinon le banni n'a qu'a attendre la recreation pour revenir.
    // On pose un overwrite deny VIEW_CHANNEL|CONNECT par utilisateur banni.
    // Tolerant aux erreurs (rate-limit Discord) : on log et on continue.
    let owner_bans = {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            api.list_owner_bans(&guild_id.to_string(), &user_id.get().to_string())
                .await
        } else {
            Vec::new()
        }
    };
    for ban in &owner_bans {
        let Ok(banned) = ban.user_id.parse::<u64>() else {
            continue;
        };
        let overwrite = PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
            kind: PermissionOverwriteType::Member(UserId::new(banned)),
        };
        if let Err(e) = voice_channel_id
            .create_permission(&ctx.http, overwrite)
            .await
        {
            warn!(error = %e, banned = %banned, "failed to re-apply ban overwrite");
        }
    }

    info!(channel = %voice_name, kind = %kind, whitelist = whitelist.len(), bans = owner_bans.len(), "Salon vocal temporaire cree");

    // Stocker les mappings locaux AVANT le move.
    {
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<VoiceOwnerMapKey>() {
            map.insert(voice_channel_id, user_id);
        }
    }

    // Deplacer l'utilisateur dans le vocal.
    if let Err(why) = guild_id
        .move_member(&ctx.http, user_id, voice_channel_id)
        .await
    {
        warn!(error = %why, "Erreur deplacement membre");
    }

    // Pour les salons "game", creer automatiquement la file d'attente.
    let queue_channel_id: Option<ChannelId> = if kind == "game" {
        let queue_name = format!("File d'attente - {display_name}");
        let mut queue_builder = CreateChannel::new(&queue_name).kind(ChannelType::Voice);
        if let Some(cat_id) = anchor_category_id {
            queue_builder = queue_builder.category(ChannelId::new(cat_id));
        }
        match guild_id.create_channel(&ctx.http, queue_builder).await {
            Ok(qch) => {
                let queue_overwrite = PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
                    deny: Permissions::SPEAK,
                    kind: PermissionOverwriteType::Role(everyone_role),
                };
                if let Err(e) = qch.id.create_permission(&ctx.http, queue_overwrite).await {
                    warn!(error = %e, "failed to set queue channel permissions (game)");
                }
                let voice_overwrite = PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::CONNECT,
                    kind: PermissionOverwriteType::Role(everyone_role),
                };
                if let Err(e) = voice_channel_id
                    .create_permission(&ctx.http, voice_overwrite)
                    .await
                {
                    warn!(error = %e, "failed to lock game voice channel behind queue");
                }
                place_queue_above_voice(ctx, guild_id, qch.id, voice_channel_id).await;
                Some(qch.id)
            }
            Err(e) => {
                error!(error = %e, "Erreur creation queue channel (game)");
                None
            }
        }
    } else {
        None
    };

    // Envoyer le panneau de controle dans le chat integre du vocal
    // (prive + game uniquement ; les publics n'ont pas de panneau).
    // Le toggle `panel_post_enabled` (voice-bot config) permet de desactiver
    // entierement la pose du panneau (la sync bilaterale devient sans objet).
    let panel_post_enabled = {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<crate::shared::heartbeat::ApiClientKey>() {
            let cfg = api
                .get_guild_config_for(&guild_id.to_string(), "voice-bot")
                .await
                .unwrap_or_default();
            cfg.get("panel_post_enabled")
                .map(|v| platform_core::sentinel::domain::entities::system::config_parsers::parse_bool_str(v))
                .unwrap_or(true)
        } else {
            true
        }
    };
    let panel_msg_id = if panel_post_enabled && (kind == "private" || kind == "game") {
        let queue_enabled_init = queue_channel_id.is_some();
        send_control_panel(
            ctx,
            voice_channel_id,
            preset_hidden,
            queue_enabled_init,
            preset_locked,
            user_id.get(),
        )
        .await
    } else {
        None
    };

    // Enregistrer via l'API. Capture l'`id` (UUID) retourne pour pouvoir
    // enregistrer le mapping `discord_action_messages` (sync bilateral).
    let voice_record_uuid: Option<uuid::Uuid> = {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            let request = CreateVoiceChannelRequest {
                guild_id: guild_id.get().to_string(),
                owner_id: user_id.get().to_string(),
                owner_name: display_name.clone(),
                channel_id: voice_channel_id.get().to_string(),
                text_channel_id: None,
                members_channel_id: None,
                queue_channel_id: queue_channel_id.map(|id| id.get().to_string()),
                category_id: None,
                channel_name: voice_name.clone(),
                kind: kind.to_string(),
                visibility: if preset_hidden {
                    "hidden".to_string()
                } else {
                    "visible".to_string()
                },
                queue_enabled: queue_channel_id.is_some(),
            };

            match api.create_channel(&request).await {
                Ok(resp) => uuid::Uuid::parse_str(&resp.id).ok(),
                Err(e) => {
                    warn!(error = %e, "Erreur API create_channel");
                    None
                }
            }
        } else {
            None
        }
    };

    // Le create RPC ne porte pas `locked` : on persiste le verrou du preset via
    // un update de suivi pour que la DB et la web admin restent coherentes avec
    // l'overwrite Discord deja applique. (Non applicable aux salons `game`.)
    if preset_locked && kind != "game" {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            let upd = UpdateVoiceChannelRequest {
                visibility: None,
                locked: Some(true),
                queue_enabled: None,
                name: None,
                status: None,
                member_limit: None,
                queue_channel_id: None,
            };
            if let Err(e) = api
                .update_channel(&voice_channel_id.get().to_string(), &upd)
                .await
            {
                warn!(error = %e, "Erreur API update_channel (verrou preset)");
            }
        }
    }

    // Sync bilateral : enregistre le panneau pour permettre au web de
    // declencher un re-render quand un admin change l etat du salon.
    if let (Some(record_uuid), Some(msg_id)) = (voice_record_uuid, panel_msg_id) {
        let data = ctx.data.read().await;
        if let Some(grpc) = data.get::<crate::shared::grpc_client::GrpcClientKey>() {
            let grpc = std::sync::Arc::clone(grpc);
            let guild_str = guild_id.to_string();
            let ch_str = voice_channel_id.to_string();
            let msg_str = msg_id.to_string();
            drop(data);
            crate::sync::register_action_message(
                &grpc,
                record_uuid,
                "voice_panel",
                &guild_str,
                &ch_str,
                &msg_str,
            )
            .await;
        }
    }

    // Creer la carte de session dans le salon de logs.
    let creator_label = {
        let name = user_id
            .to_user(&ctx.http)
            .await
            .map(|u| u.name)
            .unwrap_or_else(|_| user_id.to_string());
        format!("{} (`{}`)", name, user_id)
    };
    embeds::create_session_card(ctx, guild_id, voice_channel_id, &creator_label, kind).await;
}

/// Detecte si un salon temporaire est maintenant vide et, le cas echeant,
/// supprime le vocal (et la queue associee s'il y en a une).
///
/// Pour preserver la compat avec des salons pre-refacto encore en circulation,
/// on nettoie aussi les eventuels `text_channel_id` / `members_channel_id` /
/// `category_id` presents cote API mais nouvellement crees en vocal pur.
pub(super) async fn check_and_delete_empty(
    ctx: &Context,
    voice_channel_id: ChannelId,
    guild_id: GuildId,
) {
    let is_temp = {
        let data = ctx.data.read().await;
        data.get::<VoiceOwnerMapKey>()
            .map(|map| map.contains_key(&voice_channel_id))
            .unwrap_or(false)
    };

    if !is_temp {
        return;
    }

    let cleanup_delay = {
        let data = ctx.data.read().await;
        data.get::<crate::modules::voice::VoiceConfigKey>()
            .map(|c| c.empty_cleanup_delay_secs)
            .unwrap_or(2)
    };
    tokio::time::sleep(std::time::Duration::from_secs(cleanup_delay)).await;

    let is_empty = if let Some(guild) = ctx.cache.guild(guild_id) {
        guild
            .voice_states
            .values()
            .filter(|vs| vs.channel_id == Some(voice_channel_id))
            .count()
            == 0
    } else {
        false
    };

    if !is_empty {
        return;
    }

    let channel_name = embeds::get_channel_name(ctx, voice_channel_id).await;

    // Recupere les eventuels salons annexes legacy (queue + text + members + cat).
    let (queue_channel_id, legacy_text_id, legacy_members_id) = {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            if let Ok(Some(ch)) = api.get_channel(&voice_channel_id.get().to_string()).await {
                let queue = ch
                    .queue_channel_id
                    .and_then(|id| id.parse::<u64>().ok())
                    .map(ChannelId::new);
                let text = ch
                    .text_channel_id
                    .and_then(|id| id.parse::<u64>().ok())
                    .map(ChannelId::new);
                let members = ch
                    .members_channel_id
                    .and_then(|id| id.parse::<u64>().ok())
                    .map(ChannelId::new);
                (queue, text, members)
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        }
    };

    // Supprimer via l'API.
    {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            if let Err(e) = api
                .delete_channel(&voice_channel_id.get().to_string())
                .await
            {
                warn!(error = %e, "Erreur API delete_channel");
            }
        }
    }

    // Queue associee (game) : deconnecter les membres et supprimer.
    if let Some(queue_id) = queue_channel_id {
        let queue_members: Vec<_> = ctx
            .cache
            .guild(guild_id)
            .map(|guild| {
                guild
                    .voice_states
                    .values()
                    .filter(|vs| vs.channel_id == Some(queue_id))
                    .map(|vs| vs.user_id)
                    .collect()
            })
            .unwrap_or_default();

        for uid in queue_members {
            if let Err(e) = guild_id.disconnect_member(&ctx.http, uid).await {
                tracing::warn!(error = %e, user = %uid, "failed to disconnect member from queue");
            }
        }
        if let Err(e) = queue_id.delete(&ctx.http).await {
            tracing::warn!(error = %e, "failed to delete queue channel");
        }
        info!("Salon d'attente supprime: {queue_id}");
    }

    // Legacy text/members channels : si presents (salon cree avant la refonte),
    // on les supprime aussi pour que le menage reste complet.
    let legacy_category_id = if legacy_text_id.is_some() || legacy_members_id.is_some() {
        voice_channel_id
            .to_channel(&ctx.http)
            .await
            .ok()
            .and_then(|ch| ch.guild())
            .and_then(|gc| gc.parent_id)
    } else {
        None
    };

    if let Some(mid) = legacy_members_id {
        if let Err(e) = mid.delete(&ctx.http).await {
            warn!(error = %e, channel = %mid, "Erreur suppression panel membres legacy");
        }
    }

    if let Some(text_id) = legacy_text_id {
        if let Err(e) = text_id.delete(&ctx.http).await {
            warn!(error = %e, channel = %text_id, "Erreur suppression panel config legacy");
        }
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<crate::modules::voice::TextToVoiceMapKey>() {
            map.remove(&text_id);
        }
    }

    if let Err(why) = voice_channel_id.delete(&ctx.http).await {
        error!(error = %why, "Erreur suppression salon vocal");
    } else {
        info!(channel = %channel_name, "Salon vocal supprime");
        embeds::session_closed(ctx, voice_channel_id, "session terminee").await;
    }

    if let Some(cat_id) = legacy_category_id {
        if let Err(e) = cat_id.delete(&ctx.http).await {
            warn!(error = %e, "Erreur suppression categorie legacy");
        }
    }

    {
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<VoiceOwnerMapKey>() {
            map.remove(&voice_channel_id);
        }
    }
}

/// Place la file d'attente juste au-dessus du salon vocal principal.
pub async fn place_queue_above_voice(
    ctx: &Context,
    guild_id: GuildId,
    queue_channel_id: ChannelId,
    voice_channel_id: ChannelId,
) {
    let voice_pos = ctx
        .cache
        .guild(guild_id)
        .and_then(|g| g.channels.get(&voice_channel_id).map(|c| c.position));

    let voice_pos = match voice_pos {
        Some(p) => p as u64,
        None => return,
    };

    if let Err(e) = guild_id
        .reorder_channels(&ctx.http, [(queue_channel_id, voice_pos)])
        .await
    {
        warn!(
            error = %e,
            queue = %queue_channel_id,
            voice = %voice_channel_id,
            "reorder_channels echoue — la file d'attente reste en bas"
        );
    }
}

// ── Builders UI pour le panneau admin ──

/// Handler Redis Stream : `voice_channel_updated` (ou closed) depuis web.
/// Edite le panel embed pour refleter le nouvel etat. Skip si le payload
/// vient deja du bot (anti-boucle : actor.source != "web" => ignore).
pub async fn handle_voice_redis_event(ctx: &Context, payload: &str) {
    use serenity::all::{ChannelId, GetMessages, MessageId};

    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };
    let event_type = event.get("event").and_then(|v| v.as_str()).unwrap_or("");
    if !matches!(event_type, "voice_channel_updated" | "voice_channel_closed") {
        return;
    }
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };
    let source = data
        .get("actor")
        .and_then(|a| a.get("source"))
        .and_then(|s| s.as_str())
        .unwrap_or("");
    if source != "web" {
        return;
    }
    // L'API peut publier `id` (UUID DB) ou `voice_id`/`channel_id`. On
    // accepte plusieurs cles pour rester souple.
    let action_id = data
        .get("id")
        .or_else(|| data.get("voice_id"))
        .or_else(|| data.get("voice_channel_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if action_id.is_empty() {
        return;
    }

    let grpc = {
        let data_lock = ctx.data.read().await;
        match data_lock.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(g) => g.clone(),
            None => return,
        }
    };
    let mappings = match crate::sync::list_action_messages(&grpc, action_id).await {
        Ok(list) => list,
        Err(e) => {
            warn!(error = %e, action_id, "Echec fetch mapping voice_panel");
            return;
        }
    };
    let panel = match mappings.into_iter().find(|m| m.kind == "voice_panel") {
        Some(m) => m,
        None => return,
    };

    let channel_id = match panel.channel_id.parse::<u64>() {
        Ok(v) => ChannelId::new(v),
        Err(_) => return,
    };
    let msg_id_u64 = match panel.message_id.parse::<u64>() {
        Ok(v) => v,
        Err(_) => return,
    };
    let msg_id = MessageId::new(msg_id_u64);

    if event_type == "voice_channel_closed" {
        // Le salon est ferme — grise le panel et vire les boutons.
        if let Ok(messages) = channel_id
            .messages(&ctx.http, GetMessages::new().limit(1).around(msg_id))
            .await
        {
            if let Some(original) = messages.into_iter().find(|m| m.id == msg_id) {
                if let Some(existing_embed) = original.embeds.first() {
                    let new_embed = serenity::builder::CreateEmbed::from(existing_embed.clone())
                        .color(0x95A5A6)
                        .footer(serenity::builder::CreateEmbedFooter::new(
                            "Salon ferme depuis la web",
                        ));
                    let _ = channel_id
                        .edit_message(
                            &ctx.http,
                            msg_id,
                            serenity::builder::EditMessage::new()
                                .embed(new_embed)
                                .components(vec![]),
                        )
                        .await;
                }
            }
        }
        info!(action_id, "Voice panel grise (close depuis web)");
        return;
    }

    // event = voice_channel_updated. Re-render le panel.
    let locked = data
        .get("locked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let queue_enabled = data
        .get("queue_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let visibility = data
        .get("visibility")
        .and_then(|v| v.as_str())
        .unwrap_or("visible");
    let is_hidden = visibility == "hidden";
    let owner_id = data
        .get("owner_id")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    rerender_control_panel(
        ctx,
        channel_id,
        msg_id,
        is_hidden,
        queue_enabled,
        locked,
        owner_id,
    )
    .await;
    info!(action_id, "Voice panel re-rendu suite update web");
}

async fn rerender_control_panel(
    ctx: &Context,
    text_channel_id: ChannelId,
    panel_msg_id: serenity::model::id::MessageId,
    is_hidden: bool,
    queue_enabled: bool,
    locked: bool,
    owner_id: u64,
) {
    let visibility = if is_hidden { "Cache" } else { "Visible" };
    let queue_status = if queue_enabled {
        "Activee"
    } else {
        "Desactivee"
    };
    let lock_status = if locked { "Verrouille" } else { "Ouvert" };

    let embed = CreateEmbed::new()
        .title("Panneau de controle")
        .description(format!(
            "Salon prive de <@{owner_id}>\n\n\
            **Statut du salon :**\n\
            Visibilite : **{visibility}**\n\
            File d'attente : **{queue_status}**\n\
            Acces : **{lock_status}**\n\n\
            Etat synchronise avec la web admin."
        ))
        .color(if locked {
            0xe67e22
        } else if is_hidden {
            0xe74c3c
        } else {
            0x2ecc71
        });

    if let Err(e) = text_channel_id
        .edit_message(
            &ctx.http,
            panel_msg_id,
            serenity::builder::EditMessage::new().embed(embed),
        )
        .await
    {
        warn!(error = %e, %text_channel_id, "Echec re-render voice panel");
    }
}

/// Retourne l'`MessageId` du panel poste pour permettre l'enregistrement
/// du mapping `discord_action_messages` (sync Discord <-> web).
async fn send_control_panel(
    ctx: &Context,
    text_channel_id: ChannelId,
    is_hidden: bool,
    queue_enabled: bool,
    locked: bool,
    owner_id: u64,
) -> Option<serenity::model::id::MessageId> {
    let visibility = if is_hidden { "Cache" } else { "Visible" };
    let queue_status = if queue_enabled {
        "Activee"
    } else {
        "Desactivee"
    };
    let lock_status = if locked { "Verrouille" } else { "Ouvert" };

    let embed = CreateEmbed::new()
        .title("Panneau de controle")
        .description(format!(
            "Salon prive de <@{owner_id}>\n\n\
            **Statut du salon :**\n\
            Visibilite : **{visibility}**\n\
            File d'attente : **{queue_status}**\n\
            Acces : **{lock_status}**\n\n\
            Utilise les **boutons** ci-dessous pour editer ton salon."
        ))
        .color(if locked {
            0xe67e22
        } else if is_hidden {
            0xe74c3c
        } else {
            0x2ecc71
        });

    let hide_label = if is_hidden {
        "Rendre visible"
    } else {
        "Cacher"
    };
    let queue_label = if queue_enabled {
        "Desactiver attente"
    } else {
        "File d'attente"
    };

    let mut row1 = vec![CreateButton::new("btn_hide")
        .label(hide_label)
        .style(if is_hidden {
            ButtonStyle::Success
        } else {
            ButtonStyle::Secondary
        })];

    if !is_hidden && !locked {
        row1.push(
            CreateButton::new("btn_queue")
                .label(queue_label)
                .style(if queue_enabled {
                    ButtonStyle::Success
                } else {
                    ButtonStyle::Secondary
                }),
        );
    }

    let lock_label = if locked {
        "Deverrouiller"
    } else {
        "Verrouiller"
    };

    let row2 = vec![
        CreateButton::new("btn_kick")
            .label("Kick")
            .style(ButtonStyle::Secondary),
        CreateButton::new("btn_ban")
            .label("Ban")
            .style(ButtonStyle::Danger),
        CreateButton::new("btn_lock")
            .label(lock_label)
            .style(if locked {
                ButtonStyle::Success
            } else {
                ButtonStyle::Secondary
            }),
        CreateButton::new("btn_limit")
            .label("Limite")
            .style(ButtonStyle::Secondary),
    ];

    let row3 = vec![
        CreateButton::new("btn_rename")
            .label("Renommer")
            .style(ButtonStyle::Secondary),
        CreateButton::new("btn_status")
            .label("Statut")
            .style(ButtonStyle::Secondary),
        CreateButton::new("btn_coadmin")
            .label("Co-admin")
            .style(ButtonStyle::Secondary),
        CreateButton::new("btn_transfer")
            .label("Transferer")
            .style(ButtonStyle::Secondary),
        CreateButton::new("btn_save_prefs")
            .label("Sauvegarder params")
            .style(ButtonStyle::Primary),
    ];

    let user_select = CreateSelectMenu::new(
        "select_invite",
        CreateSelectMenuKind::User {
            default_users: None,
        },
    )
    .placeholder("Inviter des membres dans le salon")
    .min_values(1)
    .max_values(25);

    let message = CreateMessage::new().embed(embed).components(vec![
        CreateActionRow::Buttons(row1),
        CreateActionRow::Buttons(row2),
        CreateActionRow::Buttons(row3),
        CreateActionRow::SelectMenu(user_select),
    ]);

    match text_channel_id.send_message(&ctx.http, message).await {
        Ok(posted) => Some(posted.id),
        Err(why) => {
            error!(error = %why, "Erreur envoi panneau de controle");
            None
        }
    }
}
