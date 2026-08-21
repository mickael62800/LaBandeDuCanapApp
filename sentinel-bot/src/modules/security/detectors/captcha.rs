//! Captcha — ADAPTATEUR Discord. La logique pure (génération du challenge,
//! suivi des captchas en attente avec TTL) vit dans le core hexagonal
//! (`platform_core::sentinel::domain::services::security::captcha`) ; ce module ne garde
//! que l'envoi en DM et le rendu des boutons.

use serenity::builder::{CreateActionRow, CreateButton, CreateMessage};
use serenity::model::id::{GuildId, UserId};
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::shared::embeds::info_embed;

/// Génération du challenge math : logique pure, réexportée du core.
pub use platform_core::sentinel::domain::services::security::captcha::generate_math_challenge;

/// Suivi des captchas en attente, clé `(GuildId, UserId)`. Logique dans le core.
pub type CaptchaPending =
    platform_core::sentinel::domain::services::security::captcha::CaptchaPending<(GuildId, UserId)>;

/// Prefixe du bouton de verification captcha simple. Le custom_id encode le
/// guild_id : "{PREFIX}{guild_id}" -> le handler agit sur CE serveur uniquement.
pub const CAPTCHA_BUTTON_PREFIX: &str = "sentinel_captcha_verify_";

/// Prefixe des boutons captcha math.
pub const CAPTCHA_MATH_PREFIX: &str = "sentinel_captcha_math_";

/// Phrase qui annonce le delai laisse au membre.
///
/// Elle disait « 5 minutes » en dur, ce qui etait vrai tant que le delai etait
/// une constante d'environnement. Depuis qu'il se regle par serveur, un texte
/// fige ferait mentir le bot — et promettrait cinq minutes a quelqu'un qui en a
/// vingt-quatre heures, ou l'inverse.
///
/// `kick_enabled` faux : la guilde ne veut expulser personne automatiquement.
/// Menacer d'une expulsion qui ne viendra pas serait un mensonge de plus.
fn phrase_delai(timeout_secs: i64, kick_enabled: bool) -> String {
    let duree = crate::modules::security::quarantine_reminder_consumer::duree_lisible(timeout_secs);
    if kick_enabled {
        format!("Vous avez **{duree}** pour vous verifier, sinon vous serez expulse.")
    } else {
        format!(
            "Prenez le temps qu'il vous faut : sans validation, votre acces reste              restreint (rappel dans **{duree}** au plus tard)."
        )
    }
}

/// Envoie un captcha math en DM avec 4 boutons.
pub async fn send_math_challenge(
    ctx: &Context,
    user_id: UserId,
    guild_id: GuildId,
    guild_name: &str,
    pending: &CaptchaPending,
    timeout_secs: i64,
    kick_enabled: bool,
) -> bool {
    let user = match user_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, user_id = %user_id, "Impossible de recuperer l'utilisateur pour captcha math");
            return false;
        }
    };

    let dm_channel = match user.create_dm_channel(&ctx.http).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, user_id = %user_id, "Impossible de creer le DM pour captcha math");
            return false;
        }
    };

    let (question, correct_index, labels) = generate_math_challenge();

    // Stocker la reponse correcte
    pending.store((guild_id, user_id), correct_index);

    let buttons: Vec<CreateButton> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            // On encode le guild_id dans le custom_id -> le handler agit sur CE
            // serveur (avant : il scannait tous les serveurs et prenait le
            // premier ou l'user etait quarantine -> un clic pouvait liberer un
            // serveur arbitraire / tous les serveurs).
            CreateButton::new(format!("{}{}_{}", CAPTCHA_MATH_PREFIX, guild_id.get(), i))
                .label(label)
                .style(serenity::all::ButtonStyle::Primary)
        })
        .collect();

    let row = CreateActionRow::Buttons(buttons);

    let embed = info_embed("\u{1f6e1}\u{fe0f} Verification requise")
        .description(format!(
            "**Verification de securite — {}**\n\n\
             Pour prouver que vous etes humain, repondez a cette question :\n\n\
             **{}**",
            guild_name, question
        ))
        .field(
            "\u{23f1}\u{fe0f}",
            phrase_delai(timeout_secs, kick_enabled),
            false,
        );

    let message = CreateMessage::new().embed(embed).components(vec![row]);

    match dm_channel.send_message(&ctx.http, message).await {
        Ok(_) => {
            info!(user_id = %user_id, "Challenge captcha math envoye en DM");
            true
        }
        Err(e) => {
            error!(error = %e, user_id = %user_id, "Impossible d'envoyer le captcha math en DM");
            pending.remove((guild_id, user_id));
            false
        }
    }
}

/// Envoie un message de verification en DM avec un bouton.
/// Le code captcha est encode dans le custom_id du bouton.
pub async fn send_challenge(
    ctx: &Context,
    user_id: UserId,
    guild_id: GuildId,
    guild_name: &str,
    timeout_secs: i64,
    kick_enabled: bool,
) -> bool {
    let user = match user_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, user_id = %user_id, "Impossible de recuperer l'utilisateur pour captcha");
            return false;
        }
    };

    let dm_channel = match user.create_dm_channel(&ctx.http).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, user_id = %user_id, "Impossible de creer le DM pour captcha");
            return false;
        }
    };

    let button = CreateButton::new(format!("{}{}", CAPTCHA_BUTTON_PREFIX, guild_id.get()))
        .label("Je suis humain — Verifier")
        .style(serenity::all::ButtonStyle::Success);

    let row = CreateActionRow::Buttons(vec![button]);

    let embed = info_embed("\u{1f6e1}\u{fe0f} Verification requise")
        .description(format!(
            "**Verification de securite — {}**\n\n\
             Votre compte a ete detecte comme potentiellement suspect.\n\
             Cliquez sur le bouton ci-dessous pour confirmer que vous etes humain.",
            guild_name
        ))
        .field(
            "\u{23f1}\u{fe0f}",
            phrase_delai(timeout_secs, kick_enabled),
            false,
        );

    let message = CreateMessage::new().embed(embed).components(vec![row]);

    match dm_channel.send_message(&ctx.http, message).await {
        Ok(_) => {
            info!(user_id = %user_id, "Challenge captcha envoye en DM");
            true
        }
        Err(e) => {
            error!(error = %e, user_id = %user_id, "Impossible d'envoyer le captcha en DM");
            false
        }
    }
}
