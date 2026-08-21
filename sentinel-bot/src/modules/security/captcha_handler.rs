//! Handler des interactions captcha (bouton classique + math).

use serenity::all::{ComponentInteraction, Context};
use serenity::model::id::{GuildId, RoleId};
use tracing::{error, warn};

use super::api_client::ReglementApplique;
use crate::shared::embeds::{danger_embed, success_embed, warn_embed};
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::SecurityEvent;
use super::detectors::captcha;
use super::{CaptchaPendingKey, QuarantineKey, SecurityApiKey, SecurityConfigKey};

/// Gere les interactions captcha (bouton + math).
pub(super) async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = &component.data.custom_id;
    let is_button_captcha = custom_id.starts_with(captcha::CAPTCHA_BUTTON_PREFIX);
    let is_math_captcha = custom_id.starts_with(captcha::CAPTCHA_MATH_PREFIX);

    if !is_button_captcha && !is_math_captcha {
        return;
    }

    let user_id = component.user.id;

    let data = ctx.data.read().await;
    let (quarantine, env_config, base, sec_api) = match (
        data.get::<QuarantineKey>(),
        data.get::<SecurityConfigKey>(),
        data.get::<ApiClientKey>(),
        data.get::<SecurityApiKey>(),
    ) {
        (Some(q), Some(c), Some(a), Some(s)) => (q, c, a, s),
        _ => {
            error!("TypeMap incomplete pour interaction captcha");
            return;
        }
    };

    // ── Captcha math : verifier la reponse ──
    if is_math_captcha {
        let captcha_pending = match data.get::<CaptchaPendingKey>() {
            Some(p) => p,
            None => {
                error!("CaptchaPendingKey manquant");
                return;
            }
        };

        // custom_id = "{PREFIX}{guild_id}_{index}" : on lit le guild ET l'index
        // encodes -> on agit sur CE serveur (plus de scan qui liberait un
        // serveur arbitraire).
        let payload = custom_id
            .strip_prefix(captcha::CAPTCHA_MATH_PREFIX)
            .unwrap_or("");
        let (guild_str, index_str) = match payload.rsplit_once('_') {
            Some(parts) => parts,
            None => {
                tracing::warn!(user=%user_id, payload=%payload, "custom_id captcha math invalide");
                return;
            }
        };
        let pressed_index: usize = match index_str.parse::<usize>() {
            Ok(i) if i < 4 => i,
            _ => {
                tracing::warn!(user=%user_id, index=%index_str, "Index captcha invalide");
                return;
            }
        };
        let parsed_guild = guild_str.parse::<u64>().ok().map(GuildId::new);

        let guild_id = match parsed_guild {
            Some(g) => g,
            None => {
                let embed = warn_embed("\u{26a0}\u{fe0f} Deja verifie")
                    .description("Vous n'etes pas en quarantaine.");
                let response = serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                );
                if let Err(e) = component.create_response(&ctx.http, response).await {
                    warn!(error = %e, "Failed to send already-verified response");
                }
                return;
            }
        };

        match captcha_pending.verify((guild_id, user_id), pressed_index) {
            Some(true) => {
                // Bonne reponse — liberer
                captcha_pending.remove((guild_id, user_id));

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
                let role_id = guild_config
                    .get("quarantine_role_id")
                    .and_then(|v| v.parse::<u64>().ok())
                    .or(env_config.quarantine_role_id);

                if let Some(role_id) = role_id {
                    quarantine
                        .release_user(ctx, guild_id, user_id, RoleId::new(role_id))
                        .await;
                }

                // Phase 5F — supprime la row DB pour eviter que le worker
                // ne kick le user qui vient juste de valider.
                if let Some(sec_api) = data.get::<super::SecurityApiKey>() {
                    let _ = sec_api
                        .lift_quarantine(&guild_id.to_string(), &user_id.to_string())
                        .await;
                }

                let event = SecurityEvent {
                    guild_id: guild_id.to_string(),
                    event_type: "captcha_verified".to_string(),
                    severity: "info".to_string(),
                    description: format!(
                        "Utilisateur {} a passe le captcha math",
                        component.user.name
                    ),
                    user_ids: vec![user_id.to_string()],
                };
                if let Err(e) = sec_api.report_event(&event).await {
                    warn!(error = %e, "Failed to report captcha_verified event");
                }

                let embed = success_embed("\u{2705} Verification reussie")
                    .description("Bonne reponse ! Vous avez maintenant acces au serveur.");
                let response = serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                );
                if let Err(e) = component.create_response(&ctx.http, response).await {
                    warn!(error = %e, "Failed to send captcha success response");
                }
            }
            Some(false) => {
                // Mauvaise reponse — log pour detection brute-force
                tracing::warn!(guild=%guild_id, user=%user_id, index=%pressed_index, "Echec captcha math");
                let embed = danger_embed("\u{274c} Mauvaise reponse")
                    .description("Ce n'est pas la bonne reponse. Reessayez.");
                let response = serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                );
                if let Err(e) = component.create_response(&ctx.http, response).await {
                    warn!(error = %e, "Failed to send captcha failure response");
                }
            }
            None => {
                // Pas de captcha en attente : soit expire, soit perdu apres un
                // reboot du bot (l'index correct etait en RAM). Si l'utilisateur
                // est ENCORE quarantine (rehydrate depuis la DB), on lui renvoie
                // un NOUVEAU captcha au lieu de le laisser bloque -> il peut se
                // verifier malgre le redemarrage.
                if quarantine.is_quarantined(guild_id, user_id) {
                    let guild_name = guild_id
                        .to_partial_guild(&ctx.http)
                        .await
                        .map(|g| g.name)
                        .unwrap_or_else(|_| "le serveur".to_string());
                    // Lecture SEULE du reglage : reposer la quarantaine
                    // relancerait le compte a rebours, et un clic sur un
                    // bouton perime suffirait a rester indefiniment.
                    // Injoignable : on retombe sur des valeurs plausibles
                    // plutot que de priver la personne de son captcha.
                    let reglement = match ctx.data.read().await.get::<super::SecurityApiKey>() {
                        Some(api) => api
                            .quarantine_settings(&guild_id.to_string())
                            .await
                            .unwrap_or_default(),
                        None => ReglementApplique::default(),
                    };
                    captcha::send_math_challenge(
                        ctx,
                        user_id,
                        guild_id,
                        &guild_name,
                        captcha_pending,
                        reglement.timeout_secs,
                        reglement.kick_enabled,
                    )
                    .await;
                    let embed = warn_embed("\u{1f504} Nouveau captcha envoye").description(
                        "Un nouveau captcha vient de vous etre envoye en message prive.",
                    );
                    let response = serenity::builder::CreateInteractionResponse::Message(
                        serenity::builder::CreateInteractionResponseMessage::new()
                            .embed(embed)
                            .ephemeral(true),
                    );
                    let _ = component.create_response(&ctx.http, response).await;
                    return;
                }
                let embed = warn_embed("\u{26a0}\u{fe0f} Captcha expire")
                    .description("Ce captcha n'est plus valide.");
                let response = serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                );
                if let Err(e) = component.create_response(&ctx.http, response).await {
                    warn!(error = %e, "Failed to send captcha expired response");
                }
            }
        }
        return;
    }

    // ── Captcha bouton classique ──
    // Le guild est encode dans le custom_id -> on agit UNIQUEMENT sur ce serveur
    // (avant : scan de tous les serveurs -> un clic liberait partout).
    let target_guild = match custom_id
        .strip_prefix(captcha::CAPTCHA_BUTTON_PREFIX)
        .and_then(|s| s.parse::<u64>().ok())
        .map(GuildId::new)
    {
        Some(g) => g,
        None => return,
    };

    // Anti-bypass : si un captcha MATH est en attente pour cet utilisateur sur ce
    // serveur, le bouton simple ne doit PAS le liberer (il doit resoudre le math)
    // -> empeche un self-bot de sauter l'epreuve via le custom_id du bouton simple.
    if data
        .get::<CaptchaPendingKey>()
        .map(|cp| cp.is_pending((target_guild, user_id)))
        .unwrap_or(false)
    {
        let response = serenity::builder::CreateInteractionResponse::Message(
            serenity::builder::CreateInteractionResponseMessage::new()
                .embed(
                    warn_embed("\u{26a0}\u{fe0f} Captcha requis")
                        .description("Repondez a la question du captcha ci-dessus."),
                )
                .ephemeral(true),
        );
        let _ = component.create_response(&ctx.http, response).await;
        return;
    }

    let mut released = false;

    {
        let guild_id = target_guild;
        if quarantine.is_quarantined(guild_id, user_id) {
            let guild_config = match base
                .get_guild_config_for(
                    &guild_id.to_string(),
                    crate::modules::security::MODULE_BOT_NAME,
                )
                .await
            {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild (captcha bouton)");
                    std::collections::HashMap::new()
                }
            };
            let role_id = guild_config
                .get("quarantine_role_id")
                .and_then(|v| v.parse::<u64>().ok())
                .or(env_config.quarantine_role_id);

            if let Some(role_id) = role_id {
                quarantine
                    .release_user(ctx, guild_id, user_id, RoleId::new(role_id))
                    .await;

                // Phase 5F — supprime la row DB pour eviter kick worker.
                if let Some(sec_api) = data.get::<super::SecurityApiKey>() {
                    let _ = sec_api
                        .lift_quarantine(&guild_id.to_string(), &user_id.to_string())
                        .await;
                }

                let event = SecurityEvent {
                    guild_id: guild_id.to_string(),
                    event_type: "captcha_verified".to_string(),
                    severity: "info".to_string(),
                    description: format!("Utilisateur {} a passe le captcha", component.user.name),
                    user_ids: vec![user_id.to_string()],
                };
                if let Err(e) = sec_api.report_event(&event).await {
                    warn!(error = %e, "Failed to report captcha_verified event");
                }

                released = true;
            }
        }
    }

    let embed = if released {
        success_embed("\u{2705} Verification reussie")
            .description("Vous avez maintenant acces au serveur.")
    } else {
        warn_embed("\u{26a0}\u{fe0f} Deja verifie")
            .description("Vous n'etes pas en quarantaine ou la verification a deja ete effectuee.")
    };

    let response = serenity::builder::CreateInteractionResponse::Message(
        serenity::builder::CreateInteractionResponseMessage::new()
            .embed(embed)
            .ephemeral(true),
    );

    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Failed to send captcha final response");
    }
}
