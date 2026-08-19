//! Surveillance des serveurs de jeu, cote SERVEUR.
//!
//! Les alertes vivaient dans le navigateur : seuils et webhook en
//! `localStorage`, verification a chaque rafraichissement de la page. Fermer
//! l'onglet arretait donc la surveillance — or c'est la nuit, page fermee,
//! qu'un serveur sature.
//!
//! Ce passage lit les mesures relevees par le controle de sante et envoie ce
//! qu'il faut. La regle de declenchement vit dans le domaine
//! (`game::alert::evaluate_alerts`) ; ici on ne fait que l'alimenter et
//! transmettre.

use std::time::Duration;

use platform_core::nexus::domain::entities::game::alert::{
    evaluate_alerts, AlertSample, TriggeredAlert,
};
use platform_core::nexus::domain::errors::DomainError;

use crate::nexus::bootstrap::AppState;

/// Un webhook peut ne pas repondre : on ne bloque pas le tour des autres.
const TIMEOUT_WEBHOOK: Duration = Duration::from_secs(5);

pub struct AlertReport {
    pub checked: usize,
    pub sent: usize,
    pub errors: usize,
}

pub async fn run(state: &AppState) -> Result<AlertReport, DomainError> {
    let servers = state.game_server_repo.list_running().await?;
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT_WEBHOOK)
        .build()
        .map_err(|e| DomainError::Internal(format!("client HTTP: {e}")))?;

    let mut rapport = AlertReport {
        checked: 0,
        sent: 0,
        errors: 0,
    };

    for server in servers {
        let Some(config) = state.game_alert_repo.find(server.id).await? else {
            continue; // aucune alerte configuree pour ce serveur
        };
        rapport.checked += 1;

        let stats = match state.game_servers_uc.get_stats(server.id).await {
            Ok(stats) => stats,
            Err(error) => {
                tracing::warn!(%error, server_id = %server.id, "alertes : statistiques indisponibles");
                rapport.errors += 1;
                continue;
            }
        };

        // La latence vient du dernier controle de sante : c'est lui qui
        // interroge le jeu, et la mesurer ici doublerait le travail.
        let sample = AlertSample {
            cpu_percent: stats.cpu_percent,
            memory_used_mb: stats.memory_used_bytes / (1024 * 1024),
            memory_limit_mb: stats.memory_limit_bytes / (1024 * 1024),
            latency_ms: server.rcon_latency_ms,
        };

        let alertes = evaluate_alerts(&server.name, &config.settings, &sample, chrono::Utc::now());

        for alerte in alertes {
            if envoyer(&client, &config.webhook_url, &server.name, &alerte).await {
                // Marque APRES un envoi accepte : compter une alerte perdue
                // ouvrirait un silence de cinq minutes sans que personne n'ait
                // rien recu.
                if let Err(error) = state
                    .game_alert_repo
                    .mark_sent(server.id, alerte.kind)
                    .await
                {
                    tracing::warn!(%error, server_id = %server.id, "alertes : anti-spam non enregistre");
                }
                rapport.sent += 1;
            } else {
                rapport.errors += 1;
            }
        }
    }

    Ok(rapport)
}

/// Envoie l'alerte sur le webhook.
///
/// L'URL est un secret : elle n'apparait dans aucun log, seulement le nom du
/// serveur concerne.
async fn envoyer(
    client: &reqwest::Client,
    webhook: &str,
    server_name: &str,
    alerte: &TriggeredAlert,
) -> bool {
    let body = serde_json::json!({
        "username": "Nexus · Supervision",
        "embeds": [{
            "title": format!("⚠️ {}", alerte.title),
            "description": alerte.message,
            "color": alerte.kind.color(),
            "fields": [{ "name": "Serveur", "value": server_name, "inline": true }],
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }],
    });

    match client.post(webhook).json(&body).send().await {
        Ok(response) if response.status().is_success() => true,
        Ok(response) => {
            tracing::warn!(statut = %response.status(), server = server_name, "alertes : webhook refuse");
            false
        }
        Err(error) => {
            tracing::warn!(%error, server = server_name, "alertes : webhook injoignable");
            false
        }
    }
}
