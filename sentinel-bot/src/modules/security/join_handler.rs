//! Handler du join d'un nouveau membre : analyse API + quarantaine + captcha.

use chrono::DateTime;
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, RoleId, UserId};
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::{MemberPayload, RecentJoinEntry, ReglementApplique};
use super::detectors::captcha::{self, CaptchaPending};
use super::detectors::raid_analyzer::JoinInfo;
use super::{
    CaptchaPendingKey, LockdownKey, QuarantineKey, RaidDetectorKey, RaidSuggestGuardKey,
    RecentJoinsKey, SecurityApiKey, SecurityConfigKey, SlowmodeKey,
};

/// Declenche a chaque nouveau membre qui rejoint un serveur.
pub(super) async fn on_member_add(ctx: &Context, new_member: &Member) {
    let guild_id = new_member.guild_id;
    let user = &new_member.user;

    info!(
        guild_id = %guild_id,
        user = %user.name,
        user_id = %user.id,
        "Nouveau membre (security)"
    );

    let data = ctx.data.read().await;

    // Enregistrer le membre dans la BDD
    if let Some(sec_api) = data.get::<SecurityApiKey>() {
        let roles: Vec<String> = new_member.roles.iter().map(|r| r.to_string()).collect();
        let member_payload = MemberPayload {
            guild_id: guild_id.to_string(),
            user_id: user.id.to_string(),
            username: user.name.clone(),
            display_name: new_member.nick.clone(),
            avatar: user.avatar.as_ref().map(|a| a.to_string()),
            roles: serde_json::json!(roles),
            joined_at: new_member
                .joined_at
                .and_then(|t| DateTime::from_timestamp(t.unix_timestamp(), 0)),
            account_created: Some(DateTime::from_timestamp(
                user.created_at().unix_timestamp(),
                0,
            ))
            .flatten(),
            is_bot: user.bot,
            last_seen_at: None,
        };
        if let Err(e) = sec_api.register_member(&member_payload).await {
            warn!(error = %e, "Erreur register_member");
        }
    }

    // Log l'arrivee dans le journal
    if let Some(base) = data.get::<ApiClientKey>() {
        base.send_log(
            "info",
            &guild_id.to_string(),
            &format!("Nouveau membre : {} ({})", user.name, user.id),
        );
    }

    let base = match data.get::<ApiClientKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "ApiClientKey manquant");
            return;
        }
    };
    let sec_api = match data.get::<SecurityApiKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "SecurityApiKey manquant");
            return;
        }
    };
    let raid_detector = match data.get::<RaidDetectorKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "RaidDetectorKey manquant");
            return;
        }
    };
    let env_config = match data.get::<SecurityConfigKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "SecurityConfigKey manquant");
            return;
        }
    };
    let quarantine = match data.get::<QuarantineKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "QuarantineKey manquant");
            return;
        }
    };
    let slowmode = match data.get::<SlowmodeKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "SlowmodeKey manquant");
            return;
        }
    };
    let lockdown = match data.get::<LockdownKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "LockdownKey manquant");
            return;
        }
    };
    let recent_joins = match data.get::<RecentJoinsKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "RecentJoinsKey manquant");
            return;
        }
    };
    let captcha_pending = match data.get::<CaptchaPendingKey>() {
        Some(a) => a,
        None => {
            error!(guild_id = %guild_id, "CaptchaPendingKey manquant");
            return;
        }
    };

    // Charger la config per-guild depuis l'API (fallback sur env vars)
    let guild_config = match base
        .get_guild_config_for(
            &guild_id.to_string(),
            crate::modules::security::MODULE_BOT_NAME,
        )
        .await
    {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
            std::collections::HashMap::new()
        }
    };

    if !BaseApiClient::config_bool(&guild_config, "enabled", false) {
        return;
    }

    let _min_account_age = BaseApiClient::config_u64(
        &guild_config,
        "min_account_age_secs",
        env_config.min_account_age_secs,
    );

    // Config quarantaine per-guild
    let _quarantine_enabled = guild_config
        .get("quarantine_enabled")
        .map(|v| {
            platform_core::sentinel::domain::entities::system::config_parsers::parse_bool_str(v)
        })
        .unwrap_or(env_config.quarantine_enabled);
    let quarantine_role_id = guild_config
        .get("quarantine_role_id")
        .and_then(|v| {
            v.parse::<u64>()
                .map_err(|_| {
                    tracing::warn!(guild=%guild_id, value=%v, "quarantine_role_id invalide dans la config guild");
                })
                .ok()
        })
        .or(env_config.quarantine_role_id);
    let _captcha_enabled = guild_config
        .get("captcha_enabled")
        .map(|v| {
            platform_core::sentinel::domain::entities::system::config_parsers::parse_bool_str(v)
        })
        .unwrap_or(env_config.captcha_enabled);
    let _slowmode_secs: u16 = guild_config
        .get("slowmode_seconds")
        .and_then(|v| v.parse().ok())
        .unwrap_or(env_config.slowmode_seconds);
    let _lockdown_enabled = guild_config
        .get("lockdown_enabled")
        .map(|v| {
            platform_core::sentinel::domain::entities::system::config_parsers::parse_bool_str(v)
        })
        .unwrap_or(env_config.lockdown_enabled);
    let captcha_type = guild_config
        .get("captcha_type")
        .cloned()
        .unwrap_or_else(|| env_config.captcha_type.clone());
    let _alt_detection_enabled = guild_config
        .get("alt_detection_enabled")
        .map(|v| {
            platform_core::sentinel::domain::entities::system::config_parsers::parse_bool_str(v)
        })
        .unwrap_or(env_config.alt_detection_enabled);
    let _raid_pattern_enabled = guild_config
        .get("raid_pattern_enabled")
        .map(|v| {
            platform_core::sentinel::domain::entities::system::config_parsers::parse_bool_str(v)
        })
        .unwrap_or(env_config.raid_pattern_enabled);
    let _raid_pattern_score_threshold = guild_config
        .get("raid_pattern_score_threshold")
        .and_then(|v| v.parse().ok())
        .unwrap_or(env_config.raid_pattern_score_threshold);

    // ── 0. Buffer temporel des joins (le bot garde le timing, pas le metier) ──
    // Les bots (ajoutes par un admin via OAuth, pas un vecteur de raid) ne sont
    // pas comptes dans le seuil de raid -> evite qu'un ajout d'integrations
    // gonfle le compteur et declenche un faux positif.
    if user.bot {
        return;
    }

    let join_info = JoinInfo {
        username: user.name.clone(),
        has_avatar: user.avatar.is_some(),
        account_created_timestamp: user.created_at().unix_timestamp(),
    };
    recent_joins.record(guild_id, join_info);

    // Simple detection seuil de joins rapides (buffer local).
    let simple_raid = raid_detector.record_join(guild_id);

    // ── 1. Appel API : l'API decide de tout ──
    let recent = recent_joins.recent(guild_id);
    let recent_entries: Vec<RecentJoinEntry> = recent
        .iter()
        .map(|j| RecentJoinEntry {
            username: j.username.clone(),
            has_avatar: j.has_avatar,
            account_created_timestamp: j.account_created_timestamp,
        })
        .collect();

    let decision = match sec_api
        .analyze_new_member(
            &guild_id.to_string(),
            &user.id.to_string(),
            &user.name,
            user.avatar.is_some(),
            user.created_at().unix_timestamp(),
            user.bot,
            recent_entries,
            simple_raid,
        )
        .await
    {
        Ok(d) => d,
        Err(e) => {
            error!(error = %e, "Erreur API analyze_new_member");
            // Fallback local (F2) : l'API porte toute la decision ; si elle tombe,
            // on n'est plus protege. Le detecteur local `simple_raid` prend le
            // relais avec une action CONSERVATRICE et reversible (slowmode, pas de
            // lockdown/ban sur une simple heuristique).
            if simple_raid {
                slowmode
                    .activate(
                        ctx,
                        guild_id,
                        env_config.slowmode_seconds,
                        env_config.slowmode_duration_secs,
                    )
                    .await;
                warn!(guild_id = %guild_id, "API indisponible + pic de joins local -> slowmode de repli applique");
            }
            return;
        }
    };

    let is_raid = simple_raid || decision.is_raid;

    // Salon d'alerte anti-raid (suggestions) : cle dediee, repli sur le salon
    // de logs securite. `None` => pas de salon configure.
    let parse_channel = |key: &str| {
        guild_config
            .get(key)
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|id| *id > 0)
            .map(serenity::model::id::ChannelId::new)
    };
    let suggest_channel =
        parse_channel("raid_suggest_channel_id").or_else(|| parse_channel("log_channel_id"));

    // ── 2. Executer les decisions de l'API ──

    if is_raid {
        warn!(guild_id = %guild_id, score = decision.raid_score, "RAID DETECTE");

        // Reponse GUILD-WIDE presente ? (lockdown / slowmode / bump verification)
        let has_guildwide = decision.activate_lockdown || decision.slowmode_secs > 0;

        // HYBRID : si la reponse guild-wide doit etre SUGGEREE (mode suggest, ou
        // hybrid sous le seuil) et qu'un salon d'alerte existe, on poste une
        // suggestion staff au lieu d'appliquer. Sinon (ou aucun salon) : auto,
        // protection avant silence.
        let suggested = if has_guildwide && decision.suggest_only {
            match suggest_channel {
                Some(channel) => {
                    let guard = data.get::<RaidSuggestGuardKey>();
                    let acquired = guard.map(|g| g.try_acquire(guild_id)).unwrap_or(true);
                    if acquired {
                        super::raid_suggest_handler::post_suggestion(
                            ctx,
                            channel,
                            guild_id,
                            decision.raid_score,
                            &decision.event_description,
                            decision.activate_lockdown,
                            decision.slowmode_secs,
                        )
                        .await;
                    }
                    // Qu'on ait poste ou dedupe, on n'applique pas la reponse.
                    true
                }
                None => {
                    // Aucun salon configure : repli sur l'application auto pour
                    // ne pas rester silencieux face a un raid.
                    warn!(
                        guild_id = %guild_id,
                        "Mode suggest anti-raid sans salon configure : application automatique (protection avant silence)"
                    );
                    false
                }
            }
        } else {
            false
        };

        if !suggested {
            if decision.activate_lockdown {
                if let Ok(mut guild) = guild_id.to_partial_guild(&ctx.http).await {
                    let edit = serenity::builder::EditGuild::new()
                        .verification_level(serenity::model::guild::VerificationLevel::Higher);
                    if let Err(e) = guild.edit(&ctx.http, edit).await {
                        error!(error = %e, "Impossible d'activer le lockdown");
                    }
                }
                lockdown
                    .activate(ctx, guild_id, env_config.lockdown_duration_secs)
                    .await;
            }

            if decision.slowmode_secs > 0 {
                slowmode
                    .activate(
                        ctx,
                        guild_id,
                        decision.slowmode_secs as u16,
                        env_config.slowmode_duration_secs,
                    )
                    .await;
            }
        }

        raid_detector.reset(guild_id);
        recent_joins.reset(guild_id);
    }

    // Quarantaine + captcha (decision API).
    if decision.quarantine {
        if let Some(role_id) = quarantine_role_id {
            quarantine
                .quarantine_user(ctx, guild_id, user.id, RoleId::new(role_id))
                .await;

            // Phase 5F — persiste la quarantaine en DB pour que le worker
            // `kick_expired_quarantine` puisse la kicker meme si le bot
            // redemarre. Le tracker RAM reste source de verite pour les
            // roles a restaurer (la persistance ne couvre que le timer).
            // Le delai n'est plus decide ici : il appartient au serveur, et
            // l'API le renvoie pour que le message annonce la bonne duree.
            // Choisir la duree cote bot obligeait a la repeter dans le texte du
            // message — deux endroits pour une seule verite, donc un texte qui
            // finit par mentir.
            let mut reglement = ReglementApplique::default();
            if let Some(sec_api) = data.get::<super::SecurityApiKey>() {
                match sec_api
                    .mark_quarantine(&guild_id.to_string(), &user.id.to_string())
                    .await
                {
                    Ok(applique) => reglement = applique,
                    Err(e) => {
                        tracing::warn!(error = %e, "Echec persistance quarantaine (best-effort)");
                    }
                }
            }

            if decision.send_captcha {
                let guild_name = guild_id
                    .to_partial_guild(&ctx.http)
                    .await
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|_| "Serveur".to_string());
                send_captcha(
                    ctx,
                    user.id,
                    guild_id,
                    &guild_name,
                    &captcha_type,
                    captcha_pending,
                    &reglement,
                )
                .await;
            }
        }
    }

    // Log si event detecte.
    if !decision.event_type.is_empty() {
        info!(
            guild_id = %guild_id,
            event = %decision.event_type,
            desc = %decision.event_description,
            raid = decision.is_raid,
            suspicious = decision.is_suspicious_account,
            alt = decision.is_alt_account,
            "Security decision appliquee"
        );
    }
}

/// Envoie le captcha adapte selon le type configure.
async fn send_captcha(
    ctx: &Context,
    user_id: UserId,
    guild_id: GuildId,
    guild_name: &str,
    captcha_type: &str,
    captcha_pending: &CaptchaPending,
    reglement: &ReglementApplique,
) {
    match captcha_type {
        "math" => {
            captcha::send_math_challenge(
                ctx,
                user_id,
                guild_id,
                guild_name,
                captcha_pending,
                reglement.timeout_secs,
                reglement.kick_enabled,
            )
            .await;
        }
        _ => {
            captcha::send_challenge(
                ctx,
                user_id,
                guild_id,
                guild_name,
                reglement.timeout_secs,
                reglement.kick_enabled,
            )
            .await;
        }
    }
}
