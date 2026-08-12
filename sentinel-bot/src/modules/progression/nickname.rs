//! Gestion des prefixes de pseudo du module progression.
//!
//! La logique pure (strip/parse des prefixes `[NN]`, emojis staff, troncature
//! 32 chars) vit dans le core hexagonal
//! (`sentinel_core::domain::services::progression::nickname`) avec ses tests.
//! Ce module ne garde que l'orchestration Discord : fetch member, config
//! guild, positions de roles depuis le cache, et le rename via `EditMember`.

use std::collections::HashMap;

use serenity::all::{Context, EditMember, GuildId, UserId};
use serenity::model::guild::Member;
use tracing::warn;

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::MODULE_BOT_NAME;

pub use sentinel_core::domain::services::progression::nickname::{
    build_nickname_full, parse_role_emojis, pick_emoji, strip_all_prefixes,
};

/// Resultat d'une tentative de renommage. Permet a la commande resync de
/// produire un bilan precis.
#[derive(Debug)]
pub enum ResyncOutcome {
    /// Le pseudo a effectivement ete modifie.
    Renamed,
    /// Le prefixe attendu etait deja en place — rien a faire.
    AlreadyOk,
    /// Cas non actionnable : owner du serveur (Discord interdit), member
    /// introuvable, etc. — pas une erreur, juste un skip silencieux.
    Skipped,
    /// Echec Discord (perms manquantes, rate limit) — message inclus.
    Error(String),
}

/// Charge la config guild du module progression (best-effort, defaut vide).
async fn load_guild_config(ctx: &Context, guild_id: GuildId) -> HashMap<String, String> {
    let base = ctx.data.read().await.get::<ApiClientKey>().cloned();
    match base {
        Some(base) => base
            .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
            .await
            .unwrap_or_default(),
        None => HashMap::new(),
    }
}

/// Recompute le pseudo complet `{emoji}{[level]}{base}` a partir de l'etat
/// courant du membre et de la config guild, puis le met a jour si besoin.
///
/// - `level` : niveau a stamper, ou `None` pour ne pas (re)poser de `[NN]`.
/// - L'emoji staff est calcule depuis les roles du membre + la config.
/// - Owner du serveur ignore (Discord refuse le rename de l'owner).
///
/// Best-effort : log + ignore en cas d'echec. Retourne un `ResyncOutcome`.
pub async fn apply_prefixes(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    level: Option<i32>,
) -> ResyncOutcome {
    // Owner du serveur : Discord refuse `Modify Nicknames` sur l'owner.
    let is_owner = ctx
        .cache
        .guild(guild_id)
        .map(|g| g.owner_id == user_id)
        .unwrap_or(false);
    if is_owner {
        return ResyncOutcome::Skipped;
    }

    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, %user_id, "nickname: echec fetch member");
            return ResyncOutcome::Skipped;
        }
    };

    let guild_config = load_guild_config(ctx, guild_id).await;
    let staff_enabled = BaseApiClient::config_bool(&guild_config, "staff_prefix_enabled", false);
    let mappings = if staff_enabled {
        parse_role_emojis(&BaseApiClient::config_or(
            &guild_config,
            "staff_role_emojis",
            "",
        ))
    } else {
        Vec::new()
    };

    // Emoji = celui du role mappe le plus haut (par position). Necessite les
    // positions des roles depuis le cache guild ; cache manquant -> pas d'emoji.
    let emoji: Option<String> = if !mappings.is_empty() {
        let positions: Vec<(u64, i64)> = ctx
            .cache
            .guild(guild_id)
            .map(|g| {
                member
                    .roles
                    .iter()
                    .filter_map(|rid| g.roles.get(rid).map(|r| (rid.get(), r.position as i64)))
                    .collect()
            })
            .unwrap_or_default();
        pick_emoji(&positions, &mappings)
    } else {
        None
    };

    // Base = ce qui est REELLEMENT affiche, pour ne faire qu'ajouter les
    // prefixes sans ecraser le nom du membre :
    //   1. pseudo serveur (`nick`) s'il existe,
    //   2. sinon le nom d'affichage global Discord (`global_name`),
    //   3. sinon le nom de compte (`name`).
    let current = member
        .nick
        .clone()
        .or_else(|| member.user.global_name.clone())
        .unwrap_or_else(|| member.user.name.clone());
    let known: Vec<&str> = mappings.iter().map(|(_, e)| e.as_str()).collect();
    let base = strip_all_prefixes(&current, &known);
    let new_nick = build_nickname_full(base, level, emoji.as_deref());

    if new_nick == current {
        return ResyncOutcome::AlreadyOk;
    }

    match guild_id
        .edit_member(&ctx.http, user_id, EditMember::new().nickname(&new_nick))
        .await
    {
        Ok(_) => ResyncOutcome::Renamed,
        Err(e) => {
            warn!(error = %e, %user_id, new_nick, "nickname: echec rename");
            ResyncOutcome::Error(e.to_string())
        }
    }
}

/// Declencheur changement de role (guild_member_update) : quand
/// `staff_prefix_enabled`, recompute le pseudo pour refleter le role staff
/// courant, en preservant le prefixe de niveau `[NN]`.
///
/// Le niveau est recupere via l'API ; en cas d'echec, on retombe sur le `[NN]`
/// deja present dans le pseudo pour ne pas le perdre.
pub async fn on_member_update(ctx: &Context, member: &Member) {
    let guild_id = member.guild_id;
    let guild_config = load_guild_config(ctx, guild_id).await;
    if !BaseApiClient::config_bool(&guild_config, "staff_prefix_enabled", false) {
        return;
    }

    apply_prefixes(ctx, guild_id, member.user.id, None).await;
}

/// Declencheur (re)join (guild_member_addition) : un membre qui rejoint en
/// portant deja son role staff doit voir son emoji applique des l'arrivee (sans
/// attendre un level-up ou un changement de role ulterieur). Meme logique que
/// `on_member_update` : garde `staff_prefix_enabled`, best-effort, owner ignore,
/// niveau recupere via l'API avec fallback sur le `[NN]` deja present. Ne touche
/// que le pseudo, comme le chemin role-change — ne concurrence pas le welcome.
pub async fn on_member_add(ctx: &Context, member: &Member) {
    on_member_update(ctx, member).await;
}
