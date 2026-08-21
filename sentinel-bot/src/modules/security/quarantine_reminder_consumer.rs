//! Consumer Redis pour `quarantine_rules_reminder`, publie par le job
//! `remind-quarantine-rules` : envoie en message prive un rappel d'accepter le
//! reglement, avant que l'expulsion ne tombe.
//!
//! Pourquoi un rappel. Le message de verification part a la seconde ou la
//! personne rejoint — souvent au pire moment, parfois dans des messages prives
//! fermes. Sans piqure de rappel, la premiere nouvelle qu'elle a du reglement
//! est son expulsion.
//!
//! Le bouton de verification n'est PAS renvoye ici : il vit dans le premier
//! message, que ce rappel invite a rouvrir. Un second bouton laisserait deux
//! defis actifs pour la meme personne, dont un seul serait attendu par le
//! suivi en memoire.

use serenity::all::{Context, GuildId, UserId};
use std::str::FromStr;
use tracing::{info, warn};

use super::QuarantineKey;
use crate::shared::embeds::warn_embed;

pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "security-bot-quarantine-reminder".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move {
                    handle_event(&ctx, &payload_json).await;
                }
            },
        )
        .await;
    });
}

/// Rend une duree en secondes sous une forme lisible par un humain.
///
/// Annoncer « 82800 secondes » a quelqu'un qu'on menace d'expulser serait une
/// facon de ne pas etre lu.
pub fn duree_lisible(secondes: i64) -> String {
    let secondes = secondes.max(0);
    let heures = secondes / 3600;
    let minutes = (secondes % 3600) / 60;
    if heures >= 24 {
        let jours = heures / 24;
        let reste = heures % 24;
        if reste == 0 {
            return format!("{jours} jour{}", if jours > 1 { "s" } else { "" });
        }
        return format!(
            "{jours} jour{} et {reste} heure{}",
            if jours > 1 { "s" } else { "" },
            if reste > 1 { "s" } else { "" }
        );
    }
    if heures >= 1 {
        if minutes == 0 {
            return format!("{heures} heure{}", if heures > 1 { "s" } else { "" });
        }
        return format!(
            "{heures} heure{} et {minutes} minute{}",
            if heures > 1 { "s" } else { "" },
            if minutes > 1 { "s" } else { "" }
        );
    }
    if minutes >= 1 {
        return format!("{minutes} minute{}", if minutes > 1 { "s" } else { "" });
    }
    "moins d'une minute".to_string()
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let event: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if event.get("event").and_then(|v| v.as_str()) != Some("quarantine_rules_reminder") {
        return;
    }
    let Some(data) = event.get("data") else {
        return;
    };
    let guild_id_str = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let user_id_str = data.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
    let seconds_left = data
        .get("seconds_left")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let (Ok(guild_id), Ok(user_id)) = (u64::from_str(guild_id_str), u64::from_str(user_id_str))
    else {
        return;
    };
    let guild_id = GuildId::new(guild_id);
    let user_id = UserId::new(user_id);

    // La personne s'est peut-etre verifiee entre le balayage du job et la
    // reception de cet evenement : lui envoyer une menace d'expulsion serait
    // au mieux inquietant, au pire un motif de depart.
    {
        let bot_data = ctx.data.read().await;
        if let Some(q) = bot_data.get::<QuarantineKey>() {
            if !q.is_quarantined(guild_id, user_id) {
                info!(guild = %guild_id, user = %user_id, "rappel reglement ignore : deja verifie");
                return;
            }
        }
    }

    let nom_serveur = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name.clone())
        .unwrap_or_else(|_| "le serveur".to_string());

    let user = match user_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, user = %user_id, "rappel reglement : utilisateur introuvable");
            return;
        }
    };
    // Messages prives fermes : rien a faire de plus. Le premier message avait
    // deja echoue pour la meme raison, et l'expulsion suivra son cours.
    let dm = match user.create_dm_channel(&ctx.http).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, user = %user_id, "rappel reglement : messages prives fermes");
            return;
        }
    };

    let embed = warn_embed("\u{23f3} Il vous reste peu de temps").description(format!(
        "Vous n'avez pas encore accepte le reglement de **{nom_serveur}**.\n\n\
             Sans validation, votre acces sera retire dans **{}**.\n\n\
             Reprenez le message de verification recu a votre arrivee et cliquez \
             sur son bouton : c'est tout ce qui manque.",
        duree_lisible(seconds_left)
    ));

    match dm
        .send_message(
            &ctx.http,
            serenity::builder::CreateMessage::new().embed(embed),
        )
        .await
    {
        Ok(_) => info!(guild = %guild_id, user = %user_id, "rappel du reglement envoye"),
        Err(e) => warn!(error = %e, user = %user_id, "rappel reglement : envoi impossible"),
    }
}

#[cfg(test)]
mod tests {
    use super::duree_lisible;

    #[test]
    fn les_durees_sont_annoncees_en_francais_lisible() {
        assert_eq!(duree_lisible(86400), "1 jour");
        assert_eq!(duree_lisible(90000), "1 jour et 1 heure");
        assert_eq!(duree_lisible(3600), "1 heure");
        assert_eq!(duree_lisible(5400), "1 heure et 30 minutes");
        assert_eq!(duree_lisible(600), "10 minutes");
        assert_eq!(duree_lisible(30), "moins d'une minute");
    }

    #[test]
    fn une_duree_negative_ne_produit_pas_un_message_absurde() {
        // L'evenement peut arriver apres l'echeance si le bot etait arrete.
        assert_eq!(duree_lisible(-500), "moins d'une minute");
    }
}
