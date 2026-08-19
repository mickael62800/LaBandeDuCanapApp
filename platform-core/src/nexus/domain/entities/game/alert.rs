//! Alertes de supervision d'un serveur de jeu.
//!
//! La regle qui decide s'il faut prevenir vit ici, pure et testable. L'envoi
//! du message, lui, appartient a l'infrastructure.
//!
//! Trois seuils pour trois questions differentes : le processeur et la memoire
//! disent ce que le conteneur CONSOMME, le temps de reponse dit ce que les
//! joueurs SUBISSENT. Un serveur peut ramer a 30 % de processeur — c'est
//! precisement pour cela que la troisieme mesure existe.

use chrono::{DateTime, Duration, Utc};

/// Delai minimal entre deux alertes de MEME nature.
///
/// Sans lui, un serveur qui reste au-dessus du seuil enverrait un message a
/// chaque controle, soit toutes les 30 secondes : le salon devient illisible
/// et l'alerte cesse d'etre lue, ce qui revient a ne pas alerter du tout.
pub const ALERT_COOLDOWN_MINUTES: i64 = 5;

/// Reglages d'alerte d'un serveur.
#[derive(Debug, Clone)]
pub struct AlertSettings {
    pub cpu_threshold: i32,
    pub ram_threshold: i32,
    pub latency_threshold_ms: i32,
    pub last_cpu_alert_at: Option<DateTime<Utc>>,
    pub last_ram_alert_at: Option<DateTime<Utc>>,
    pub last_latency_alert_at: Option<DateTime<Utc>>,
}

/// Mesures relevees sur un serveur au moment du controle.
#[derive(Debug, Clone, Copy)]
pub struct AlertSample {
    pub cpu_percent: f64,
    pub memory_used_mb: u64,
    pub memory_limit_mb: u64,
    /// `None` quand le jeu n'a pas ete interroge (pas de RCON) : on ne peut
    /// alors rien dire de sa reactivite.
    pub latency_ms: Option<i32>,
}

/// Nature d'une alerte declenchee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Cpu,
    Ram,
    Latency,
}

impl AlertKind {
    /// Couleur de l'embed Discord. Le temps de reponse est en jaune : il
    /// annonce une gene, pas encore une panne.
    pub fn color(self) -> u32 {
        match self {
            Self::Cpu => 0xe7_4c_3c,
            Self::Ram => 0xe6_7e_22,
            Self::Latency => 0xf1_c4_0f,
        }
    }
}

/// Une alerte a envoyer.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggeredAlert {
    pub kind: AlertKind,
    pub title: String,
    pub message: String,
}

/// Determine ce qu'il faut annoncer, sans rien envoyer.
///
/// Un seuil atteint ne suffit pas : il faut aussi que le delai depuis la
/// derniere alerte de meme nature soit ecoule.
pub fn evaluate_alerts(
    server_name: &str,
    settings: &AlertSettings,
    sample: &AlertSample,
    now: DateTime<Utc>,
) -> Vec<TriggeredAlert> {
    let cooldown = Duration::minutes(ALERT_COOLDOWN_MINUTES);
    let pret = |dernier: Option<DateTime<Utc>>| match dernier {
        None => true,
        Some(t) => now - t >= cooldown,
    };

    let mut alertes = Vec::new();

    if sample.cpu_percent >= settings.cpu_threshold as f64 && pret(settings.last_cpu_alert_at) {
        alertes.push(TriggeredAlert {
            kind: AlertKind::Cpu,
            title: "Processeur au-dessus du seuil".into(),
            message: format!(
                "**{server_name}** consomme **{:.1} %** de processeur (seuil : {} %).",
                sample.cpu_percent, settings.cpu_threshold
            ),
        });
    }

    // La limite peut valoir zero si le conteneur n'en declare pas : diviser
    // par elle donnerait un pourcentage absurde, et donc une fausse alerte.
    if sample.memory_limit_mb > 0 {
        let pct = sample.memory_used_mb as f64 / sample.memory_limit_mb as f64 * 100.0;
        if pct >= settings.ram_threshold as f64 && pret(settings.last_ram_alert_at) {
            alertes.push(TriggeredAlert {
                kind: AlertKind::Ram,
                title: "Mémoire au-dessus du seuil".into(),
                message: format!(
                    "**{server_name}** utilise **{:.1} %** de sa mémoire ({} Mo sur {} Mo, seuil : {} %).",
                    pct, sample.memory_used_mb, sample.memory_limit_mb, settings.ram_threshold
                ),
            });
        }
    }

    if let Some(latence) = sample.latency_ms {
        if latence >= settings.latency_threshold_ms && pret(settings.last_latency_alert_at) {
            alertes.push(TriggeredAlert {
                kind: AlertKind::Latency,
                title: "Serveur lent à répondre".into(),
                message: format!(
                    "**{server_name}** met **{latence} ms** à répondre (seuil : {} ms). \
                     Les joueurs ressentent probablement du lag. Si la charge de l'hôte dépasse \
                     son nombre de cœurs, la machine est en cause plutôt que ce jeu.",
                    settings.latency_threshold_ms
                ),
            });
        }
    }

    alertes
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 19, 12, minute, 0).unwrap()
    }

    fn reglages() -> AlertSettings {
        AlertSettings {
            cpu_threshold: 85,
            ram_threshold: 90,
            latency_threshold_ms: 500,
            last_cpu_alert_at: None,
            last_ram_alert_at: None,
            last_latency_alert_at: None,
        }
    }

    fn mesure() -> AlertSample {
        AlertSample {
            cpu_percent: 10.0,
            memory_used_mb: 100,
            memory_limit_mb: 1000,
            latency_ms: Some(50),
        }
    }

    #[test]
    fn rien_a_signaler_quand_tout_va_bien() {
        assert!(evaluate_alerts("Palworld", &reglages(), &mesure(), t(0)).is_empty());
    }

    #[test]
    fn chaque_seuil_declenche_sa_propre_alerte() {
        let sample = AlertSample {
            cpu_percent: 92.0,
            memory_used_mb: 950,
            memory_limit_mb: 1000,
            latency_ms: Some(1200),
        };
        let alertes = evaluate_alerts("Palworld", &reglages(), &sample, t(0));
        let kinds: Vec<_> = alertes.iter().map(|a| a.kind).collect();
        assert_eq!(
            kinds,
            vec![AlertKind::Cpu, AlertKind::Ram, AlertKind::Latency]
        );
    }

    #[test]
    fn le_delai_empeche_de_repeter_la_meme_alerte() {
        // Sans lui, un serveur durablement charge ecrirait toutes les 30 s :
        // le salon devient illisible et l'alerte cesse d'etre lue.
        let mut settings = reglages();
        settings.last_cpu_alert_at = Some(t(0));
        let sample = AlertSample {
            cpu_percent: 99.0,
            ..mesure()
        };

        assert!(evaluate_alerts("Palworld", &settings, &sample, t(2)).is_empty());
        assert_eq!(
            evaluate_alerts("Palworld", &settings, &sample, t(6)).len(),
            1,
            "passe le delai, l'alerte repart"
        );
    }

    #[test]
    fn une_limite_memoire_nulle_ne_declenche_pas_une_fausse_alerte() {
        // Certains conteneurs ne declarent pas de limite : diviser par zero
        // donnerait un pourcentage absurde, donc une alerte permanente.
        let sample = AlertSample {
            memory_used_mb: 500,
            memory_limit_mb: 0,
            ..mesure()
        };
        assert!(evaluate_alerts("Palworld", &reglages(), &sample, t(0)).is_empty());
    }

    #[test]
    fn sans_mesure_de_latence_on_ne_conclut_pas() {
        // Serveur sans RCON : on ne sait rien de sa reactivite, et l'inventer
        // ferait alerter au hasard.
        let sample = AlertSample {
            latency_ms: None,
            ..mesure()
        };
        assert!(evaluate_alerts("Palworld", &reglages(), &sample, t(0)).is_empty());
    }
}
