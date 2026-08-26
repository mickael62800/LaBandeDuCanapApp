//! Rassemble les faits d'une session et confie la plume a Atrium.
//!
//! Ce service n'ecrit pas une phrase et ne publie rien. Il repond a une seule
//! question — « que faut-il dire de cette soiree ? » — et laisse le domaine
//! voisin la formuler, le bot la publier.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::TimeZone;
use chrono_tz::Tz;
use uuid::Uuid;

use crate::nexus::domain::entities::game::schedule::{
    etiquette_des_plages, next_opening, AutoSchedule,
};
use crate::nexus::ports::inbound::game::session_announcement::{
    SessionAnnouncementError, SessionAnnouncementUseCase,
};
use crate::nexus::ports::outbound::game::announcement_gateway::{
    GameAnnouncementGateway, SessionFacts,
};
use crate::nexus::ports::outbound::game::game_server_config_repository::GameServerConfigRepository;
use crate::nexus::ports::outbound::game::game_server_repository::GameServerRepository;
use crate::nexus::ports::outbound::game::game_template_repository::GameTemplateRepository;
use crate::nexus::ports::outbound::game::schedule_repository::{
    GameScheduleRepository, StoredSchedule,
};

/// Au-dela, on cesse de retenter.
///
/// La reprise passe toutes les cinq minutes : douze tentatives couvrent une
/// heure de panne. Au-dela, l'incident n'est plus passager et la bonne reponse
/// est de prevenir un humain, pas d'accumuler des appels qui echouent tous de
/// la meme facon — chacun consommant un quota d'IA.
pub const TENTATIVES_MAX: i32 = 12;

pub struct SessionAnnouncementService {
    pub server_repo: Arc<dyn GameServerRepository>,
    pub template_repo: Arc<dyn GameTemplateRepository>,
    pub config_repo: Arc<dyn GameServerConfigRepository>,
    pub schedule_repo: Arc<dyn GameScheduleRepository>,
    pub gateway: Arc<dyn GameAnnouncementGateway>,
}

#[async_trait]
impl SessionAnnouncementUseCase for SessionAnnouncementService {
    async fn rediger(&self, server_id: Uuid) -> Result<String, SessionAnnouncementError> {
        let serveur = self
            .server_repo
            .find_by_id(server_id)
            .await
            .map_err(|e| SessionAnnouncementError::Interne(e.to_string()))?
            .ok_or(SessionAnnouncementError::Introuvable(server_id))?;

        if serveur.announcement_posted_at.is_some() {
            return Err(SessionAnnouncementError::RienAAnnoncer);
        }
        if serveur.announcement_attempts >= TENTATIVES_MAX {
            return Err(SessionAnnouncementError::AbandonApresPlafond);
        }

        // Compte la tentative AVANT l'appel, jamais apres. Comptee apres, une
        // panne qui interrompt le processus entre l'appel et l'ecriture ne
        // laisserait aucune trace : la reprise repartirait de zero a chaque
        // passage et le plafond ne serait jamais atteint.
        self.server_repo
            .compter_tentative_annonce(server_id)
            .await
            .map_err(|e| SessionAnnouncementError::Interne(e.to_string()))?;

        let modele = self
            .template_repo
            .find_by_id(serveur.template_id)
            .await
            .map_err(|e| SessionAnnouncementError::Interne(e.to_string()))?
            .ok_or_else(|| {
                SessionAnnouncementError::Interne("template du serveur introuvable".into())
            })?;

        // Une configuration ou un horaire illisibles ne doivent pas empecher
        // l'annonce : ils la rendent seulement plus pauvre. Le jeu et le nom du
        // serveur suffisent a dire l'essentiel.
        let config = self
            .config_repo
            .get_all(server_id)
            .await
            .unwrap_or_default();
        let horaire = self.schedule_repo.find(server_id).await.ok().flatten();

        let (opening_label, schedule_label) = match horaire {
            Some(stored) => {
                let auto = vers_auto_schedule(&stored);
                (etiquette_d_ouverture(&auto), etiquette_des_plages(&auto))
            }
            None => (None, None),
        };

        let faits = SessionFacts {
            guild_id: serveur.guild_id,
            game_name: modele.name,
            server_name: serveur.name,
            max_players: jauge_de_joueurs(&config, &modele.default_env),
            opening_label,
            schedule_label,
        };

        Ok(self.gateway.rediger(faits).await?)
    }

    async fn marquer_publiee(&self, server_id: Uuid) -> Result<(), SessionAnnouncementError> {
        self.server_repo
            .marquer_annonce_publiee(server_id)
            .await
            .map_err(|e| SessionAnnouncementError::Interne(e.to_string()))
    }
}

/// Jauge de joueurs annoncee.
///
/// L'override du serveur prime sur le defaut du modele : c'est le chiffre que
/// l'exploitant a choisi, et celui que les joueurs constateront. Une valeur
/// illisible vaut une absence — mieux vaut ne pas annoncer de jauge que d'en
/// annoncer une fausse.
pub fn jauge_de_joueurs(
    config: &std::collections::HashMap<String, String>,
    default_env: &serde_json::Value,
) -> Option<u32> {
    let depuis_config = config
        .get("MAX_PLAYERS")
        .and_then(|v| v.trim().parse().ok());
    depuis_config.or_else(|| {
        default_env
            .get("MAX_PLAYERS")
            .and_then(|valeur| match valeur {
                serde_json::Value::String(texte) => texte.trim().parse().ok(),
                serde_json::Value::Number(nombre) => {
                    nombre.as_u64().and_then(|n| u32::try_from(n).ok())
                }
                _ => None,
            })
    })
}

/// Ouverture prevue, mise en forme dans le fuseau de la guilde.
///
/// Nexus formate la date parce que lui seul connait le fuseau et les plages.
/// Un fuseau illisible rend `None` : annoncer une heure fausse serait pire que
/// de n'en annoncer aucune.
pub fn etiquette_d_ouverture(schedule: &AutoSchedule) -> Option<String> {
    let instant = next_opening(schedule, chrono::Utc::now())?;
    let tz: Tz = schedule.timezone.parse().ok()?;
    Some(format_francais(tz.from_utc_datetime(&instant.naive_utc())))
}

/// Date en francais, sans dependance de locale.
///
/// `chrono` ne traduit pas les noms de jours et de mois sans fonctionnalite
/// supplementaire, et cette annonce est lue par des joueurs francophones : une
/// table de sept et douze entrees coute moins qu'une dependance de plus.
fn format_francais(instant: chrono::DateTime<Tz>) -> String {
    use chrono::{Datelike, Timelike};
    const JOURS: [&str; 7] = [
        "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
    ];
    const MOIS: [&str; 12] = [
        "janvier",
        "fevrier",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "aout",
        "septembre",
        "octobre",
        "novembre",
        "decembre",
    ];
    let jour = JOURS[instant.weekday().num_days_from_monday() as usize];
    let mois = MOIS[instant.month0() as usize];
    let heure = if instant.minute() == 0 {
        format!("{}h", instant.hour())
    } else {
        format!("{}h{:02}", instant.hour(), instant.minute())
    };
    format!("{jour} {} {mois} a {heure}", instant.day())
}

fn vers_auto_schedule(stored: &StoredSchedule) -> AutoSchedule {
    AutoSchedule {
        enabled: stored.enabled,
        mode: stored.mode,
        timezone: stored.timezone.clone(),
        ranges: stored.ranges.clone(),
        warn_minutes: stored.warn_minutes,
        opens_at: None,
        closes_at: None,
        restart_interval_hours: stored.restart_interval_hours,
        restart_anchor_minute: stored.restart_anchor_minute,
        last_restart_at: None,
        last_warned_at: stored.last_warned_at,
        last_final_warned_at: None,
    }
}

#[cfg(test)]
#[path = "tests/session_announcement_service.rs"]
mod tests;
