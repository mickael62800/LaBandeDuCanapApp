//! Carte de changement de roles « vivante » (anti-spam).
//!
//! Probleme : Discord emet un `GUILD_MEMBER_UPDATE` par changement de role ->
//! une carte par role ajoute/retire = spam. Solution : une SEULE carte par
//! membre qui reste active pendant une fenetre glissante (defaut 2 min) et se
//! met a jour (edition) avec l'HISTORIQUE COMPLET des mouvements.
//!
//! L'ÉTAT (map fenêtrée, bornes, troncature) vit dans le core
//! (`services::audit::role_card`) ; ce module garde la config, le post/édit
//! Discord et l'embed.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use sentinel_core::domain::services::audit::role_card::{
    clamp_role_log_window, visible_movements, RoleMovement,
};
use serenity::all::{
    ChannelId, Context, CreateEmbed, CreateMessage, EditMessage, Member, MessageId, RoleId,
};
use serenity::prelude::TypeMapKey;

use crate::shared::heartbeat::ApiClientKey;

pub type RoleCardTracker =
    sentinel_core::domain::services::audit::role_card::RoleCardTracker<(String, String)>;

pub struct RoleCardTrackerKey;
impl TypeMapKey for RoleCardTrackerKey {
    type Value = std::sync::Arc<RoleCardTracker>;
}

/// Nombre max de lignes affichees dans la carte (limite champ embed).
const MAX_MOVEMENTS: usize = 20;

/// Traite un changement de roles : cree ou met a jour la carte vivante.
pub async fn handle_role_change(
    ctx: &Context,
    guild_id: &str,
    member: &Member,
    added_now: &[RoleId],
    removed_now: &[RoleId],
) {
    if added_now.is_empty() && removed_now.is_empty() {
        return;
    }

    // Config audit-bot (fenetre + salon).
    let cfg = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(api) => api
                .get_guild_config_for(guild_id, super::MODULE_BOT_NAME)
                .await
                .unwrap_or_default(),
            None => return,
        }
    };
    let window = clamp_role_log_window(
        cfg.get("role_log_window_secs")
            .and_then(|v| v.parse::<u64>().ok()),
    );

    let tracker = {
        let data = ctx.data.read().await;
        match data.get::<RoleCardTrackerKey>() {
            Some(t) => t.clone(),
            None => return,
        }
    };

    let now = Instant::now();
    let key = (guild_id.to_string(), member.user.id.to_string());

    // Snapshot de la carte active (et purge des expirees). Lock court, pas d'await.
    let active = tracker.active(&key, now);

    // Historique cumule = existant + mouvements de cet evenement.
    let mut movements = active
        .as_ref()
        .map(|(_, _, m)| m.clone())
        .unwrap_or_default();
    for r in added_now {
        movements.push((true, r.to_string()));
    }
    for r in removed_now {
        movements.push((false, r.to_string()));
    }

    let embed = build_embed(member, &movements, window);
    let expires_at = now + Duration::from_secs(window);

    if let Some((chan, msg, _)) = active {
        // Edite la carte existante.
        let _ = ChannelId::new(chan)
            .edit_message(
                &ctx.http,
                MessageId::new(msg),
                EditMessage::new().embed(embed),
            )
            .await;
        tracker.update(&key, movements, expires_at);
    } else {
        // Nouvelle carte : resout le salon puis poste.
        let Some(chan) = resolve_channel(&cfg) else {
            return;
        };
        if let Ok(m) = chan
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await
        {
            tracker.insert(key, chan.get(), m.id.get(), movements, expires_at);
        }
    }
}

/// Salon cible : `profile_edit_channel_id` puis fallback `log_channel_id`.
fn resolve_channel(cfg: &HashMap<String, String>) -> Option<ChannelId> {
    for key in ["profile_edit_channel_id", "log_channel_id"] {
        if let Some(id) = cfg
            .get(key)
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&n| n > 0)
        {
            return Some(ChannelId::new(id));
        }
    }
    None
}

fn build_embed(member: &Member, movements: &[RoleMovement], window: u64) -> CreateEmbed {
    let (hidden, visible) = visible_movements(movements, MAX_MOVEMENTS);
    let mut added_roles: Vec<&str> = Vec::new();
    let mut removed_roles: Vec<&str> = Vec::new();

    for (added, role) in visible {
        if *added {
            added_roles.push(role);
        } else {
            removed_roles.push(role);
        }
    }

    let hidden_label = if hidden > 0 {
        format!(
            " · {hidden} plus ancienne{} masquée{}",
            if hidden > 1 { "s" } else { "" },
            if hidden > 1 { "s" } else { "" }
        )
    } else {
        String::new()
    };
    let summary = format!(
        "<@{}>\n`ID : {}` · **{} modification{}**{}",
        member.user.id,
        member.user.id,
        movements.len(),
        if movements.len() > 1 { "s" } else { "" },
        hidden_label,
    );

    let mut embed = crate::shared::embeds::info_embed("🎭 Mise à jour des rôles")
        .description(summary)
        .thumbnail(member.user.face())
        .footer(serenity::builder::CreateEmbedFooter::new(format!(
            "Sentinel Audit · regroupement actif {}",
            format_duration(window),
        )));

    if !added_roles.is_empty() {
        embed = embed.field(
            format!("🟢 Ajoutés · {}", added_roles.len()),
            format_roles(&added_roles),
            false,
        );
    }
    if !removed_roles.is_empty() {
        embed = embed.field(
            format!("🔴 Retirés · {}", removed_roles.len()),
            format_roles(&removed_roles),
            false,
        );
    }
    embed
}

fn format_roles(roles: &[&str]) -> String {
    roles
        .iter()
        .map(|role| format!("• <@&{role}>"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_duration(seconds: u64) -> String {
    if seconds % 60 == 0 {
        format!("{} min", seconds / 60)
    } else {
        format!("{seconds} s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_list_is_vertical_without_repeating_status_icons() {
        assert_eq!(format_roles(&["12", "34"]), "• <@&12>\n• <@&34>");
    }

    #[test]
    fn duration_is_compact_and_readable() {
        assert_eq!(format_duration(120), "2 min");
        assert_eq!(format_duration(45), "45 s");
    }
}
