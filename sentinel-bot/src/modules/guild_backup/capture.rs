//! Capture de la structure d'un serveur Discord -> `GuildSnapshot`.
//!
//! Lit la guild via serenity (HTTP, pas le cache pour avoir des donnees a jour)
//! et construit le contrat serde partage avec l'API. Tous les `old_id` sont les
//! IDs Discord ACTUELS (source), remappes a la restauration.
//!
//! Best-effort documente : bans (peut echouer si perms manquantes), emojis
//! (URL CDN uniquement, image non telechargee), member_roles (borne a 1000
//! membres charges).

use std::collections::BTreeMap;

use serenity::all::{ChannelType, Context, GuildId, PermissionOverwriteType};
use tracing::{info, warn};

use platform_core::sentinel::domain::entities::guild_backup::snapshot::{
    GuildSettings, GuildSnapshot, SnapshotBan, SnapshotCategory, SnapshotChannel, SnapshotEmoji,
    SnapshotMeta, SnapshotOverwrite, SnapshotRole, SCHEMA_VERSION,
};

/// Nombre max de membres charges pour la capture des roles par membre.
const MEMBER_ROLES_LIMIT: usize = 1000;

/// Traduit un [`ChannelType`] serenity vers le `kind` textuel du snapshot.
/// Retourne `None` pour les types non pertinents (threads, DM, categorie...).
fn channel_kind(kind: ChannelType) -> Option<&'static str> {
    match kind {
        ChannelType::Text => Some("text"),
        ChannelType::Voice => Some("voice"),
        ChannelType::Forum => Some("forum"),
        ChannelType::News => Some("announcement"),
        ChannelType::Stage => Some("stage"),
        _ => None,
    }
}

/// Capture complete de la structure du serveur `guild_id`.
///
/// `label` : libelle humain de la sauvegarde. `created_by` : ID Discord de
/// l'auteur (owner qui declenche la commande).
pub async fn capture(
    ctx: &Context,
    guild_id: GuildId,
    label: &str,
    created_by: &str,
) -> Result<GuildSnapshot, String> {
    let gid = guild_id.to_string();

    // 1. Guild (settings, roles, emojis).
    let partial = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map_err(|e| format!("Lecture du serveur impossible : {e}"))?;

    // 2. Roles : exclut @everyone (id == guild_id) et les roles managed (bot /
    //    integration / boost) qui ne sont pas recreables.
    let mut roles: Vec<SnapshotRole> = partial
        .roles
        .values()
        .filter(|r| r.id != guild_id.everyone_role() && !r.managed)
        .map(|r| SnapshotRole {
            old_id: r.id.to_string(),
            name: r.name.clone(),
            color: r.colour.0,
            permissions: r.permissions.bits().to_string(),
            hoist: r.hoist,
            mentionable: r.mentionable,
            position: r.position as i32,
        })
        .collect();
    // Ordre hierarchique croissant : cree du bas vers le haut a la restauration.
    roles.sort_by_key(|r| r.position);

    // 3. Categories + salons.
    let channels = guild_id
        .channels(&ctx.http)
        .await
        .map_err(|e| format!("Lecture des salons impossible : {e}"))?;

    let mut categories: Vec<SnapshotCategory> = Vec::new();
    let mut snap_channels: Vec<SnapshotChannel> = Vec::new();

    for ch in channels.values() {
        if ch.kind == ChannelType::Category {
            categories.push(SnapshotCategory {
                old_id: ch.id.to_string(),
                name: ch.name.clone(),
                position: ch.position as i32,
            });
            continue;
        }
        let Some(kind) = channel_kind(ch.kind) else {
            continue;
        };

        let overwrites = ch
            .permission_overwrites
            .iter()
            .map(|ow| {
                let (target_type, target_old_id) = match ow.kind {
                    PermissionOverwriteType::Role(r) => ("role", r.to_string()),
                    PermissionOverwriteType::Member(u) => ("member", u.to_string()),
                    // Variante future non geree : on la classe en role par defaut.
                    _ => ("role", "0".to_string()),
                };
                SnapshotOverwrite {
                    target_old_id,
                    target_type: target_type.to_string(),
                    allow: ow.allow.bits().to_string(),
                    deny: ow.deny.bits().to_string(),
                }
            })
            .collect();

        snap_channels.push(SnapshotChannel {
            old_id: ch.id.to_string(),
            kind: kind.to_string(),
            name: ch.name.clone(),
            parent_old_id: ch.parent_id.map(|p| p.to_string()),
            topic: ch.topic.clone(),
            nsfw: ch.nsfw,
            slowmode: ch.rate_limit_per_user.unwrap_or(0) as u32,
            bitrate: ch.bitrate,
            user_limit: ch.user_limit,
            position: ch.position as i32,
            overwrites,
        });
    }
    categories.sort_by_key(|c| c.position);
    snap_channels.sort_by_key(|c| c.position);

    // 4. Settings.
    let (afk_channel_old_id, afk_timeout) = match &partial.afk_metadata {
        Some(a) => (
            Some(a.afk_channel_id.to_string()),
            u16::from(a.afk_timeout) as u32,
        ),
        None => (None, 300),
    };
    let settings = GuildSettings {
        name: partial.name.clone(),
        icon: partial.icon_url(),
        verification_level: u8::from(partial.verification_level) as u32,
        default_notifications: u8::from(partial.default_message_notifications) as u32,
        explicit_content_filter: u8::from(partial.explicit_content_filter) as u32,
        afk_channel_old_id,
        afk_timeout,
        system_channel_old_id: partial.system_channel_id.map(|c| c.to_string()),
        // Permissions de base de @everyone. Ce role est exclu de la liste
        // des roles (il ne se recree pas), ses permissions se perdaient donc.
        everyone_permissions: partial
            .roles
            .get(&guild_id.everyone_role())
            .map(|r| r.permissions.bits().to_string())
            .unwrap_or_default(),
    };

    // 5. Bans (best-effort : perms MANAGE_GUILD/BAN requises).
    let bans = match guild_id.bans(&ctx.http, None, None).await {
        Ok(list) => list
            .into_iter()
            .map(|b| SnapshotBan {
                user_id: b.user.id.to_string(),
                reason: b.reason,
            })
            .collect(),
        Err(e) => {
            warn!(error = %e, guild = %gid, "guild_backup: bans non captures (best-effort)");
            Vec::new()
        }
    };

    // 6. Emojis (best-effort : on stocke l'URL CDN, pas les octets).
    let emojis: Vec<SnapshotEmoji> = partial
        .emojis
        .values()
        .map(|e| SnapshotEmoji {
            name: e.name.clone(),
            image_ref: e.url(),
        })
        .collect();

    // 7. member_roles (best-effort, borne a MEMBER_ROLES_LIMIT membres).
    let mut member_roles: BTreeMap<String, Vec<String>> = BTreeMap::new();
    match guild_id
        .members(&ctx.http, Some(MEMBER_ROLES_LIMIT as u64), None)
        .await
    {
        Ok(members) => {
            for m in members {
                if m.roles.is_empty() {
                    continue;
                }
                member_roles.insert(
                    m.user.id.to_string(),
                    m.roles.iter().map(|r| r.to_string()).collect(),
                );
            }
        }
        Err(e) => {
            warn!(error = %e, guild = %gid, "guild_backup: member_roles non captures (best-effort)");
        }
    }

    let snapshot = GuildSnapshot {
        guild_id: gid.clone(),
        meta: SnapshotMeta {
            label: label.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            created_by: Some(created_by.to_string()),
            schema_version: SCHEMA_VERSION,
        },
        settings,
        roles,
        categories,
        channels: snap_channels,
        bans,
        emojis,
        member_roles,
    };

    info!(
        guild = %gid,
        roles = snapshot.roles.len(),
        categories = snapshot.categories.len(),
        channels = snapshot.channels.len(),
        bans = snapshot.bans.len(),
        emojis = snapshot.emojis.len(),
        members = snapshot.member_roles.len(),
        "guild_backup: capture terminee"
    );

    Ok(snapshot)
}
