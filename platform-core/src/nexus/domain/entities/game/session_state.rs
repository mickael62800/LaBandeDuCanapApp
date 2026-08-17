//! Ce qu'une session de jeu annonce aux joueurs : ouvert, bientôt, fermé.
//!
//! L'état affiché n'est PAS l'état du conteneur. Un conteneur arrêté peut être
//! une session qui n'a pas encore commencé, ou une session en pause qui va
//! reprendre, ou une session terminée — et les trois ne se racontent pas de la
//! même façon. C'est la fenêtre horaire de la session qui tranche, le conteneur
//! ne dit que s'il est joignable maintenant.
//!
//! La règle, telle qu'elle a été posée :
//!
//!   - avant l'heure d'ouverture, c'est TOUJOURS « ouvre bientôt », même si le
//!     conteneur tourne déjà (le worker le démarre en avance, mais l'adresse
//!     n'est pas encore donnée) ;
//!   - pendant la fenêtre, conteneur en marche : « ouvert » ;
//!   - pendant la fenêtre, conteneur arrêté : « ouvre bientôt » quand même,
//!     parce que la session va reprendre ;
//!   - après l'heure de fermeture, conteneur arrêté : « fermé ».
//!
//! Révéler l'adresse repousse l'heure d'ouverture à l'instant du clic (plus le
//! préavis configuré) : la session bascule donc d'elle-même vers « ouvert »
//! quand ce préavis s'achève, c'est-à-dire au moment où l'adresse est publiée.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::server::GameServerStatus;

/// État de session tel qu'il est montré aux joueurs, sur Discord comme sur le
/// site. Une seule règle pour les deux surfaces : elles se contredisaient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionDisplayState {
    /// La session n'a pas encore commencé, ou reprend bientôt.
    Waiting,
    /// On peut jouer maintenant.
    Open,
    /// C'est terminé.
    Closed,
}

impl SessionDisplayState {
    /// Suffixe de la jaquette correspondante (`palworld_game_attente.jpg`).
    /// Vide pour une session ouverte : c'est l'image de base.
    pub fn cover_suffix(self) -> &'static str {
        match self {
            Self::Open => "",
            Self::Waiting => "_attente",
            Self::Closed => "_offline",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Open => "open",
            Self::Closed => "closed",
        }
    }
}

/// Détermine ce que la session annonce.
///
/// `opens_at` est l'heure d'ouverture (la révélation de l'adresse), `closes_at`
/// l'heure de fin annoncée. L'une comme l'autre peuvent manquer : un serveur
/// créé sans programmation n'a pas de fenêtre, et retombe alors sur l'état de
/// son conteneur — c'est tout ce qu'on sait de lui.
pub fn session_display_state(
    status: GameServerStatus,
    opens_at: Option<DateTime<Utc>>,
    closes_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> SessionDisplayState {
    // Un serveur supprimé ou en erreur ne rouvrira pas tout seul : aucune
    // promesse d'ouverture ne tiendrait, quelle que soit la fenêtre.
    if matches!(status, GameServerStatus::Deleted | GameServerStatus::Error) {
        return SessionDisplayState::Closed;
    }

    // Avant l'heure dite, la session n'a pas commencé — même si le conteneur
    // tourne déjà. Le worker le démarre en avance ; annoncer « ouvert » ferait
    // venir des joueurs qui n'ont pas encore l'adresse.
    if let Some(opens_at) = opens_at {
        if now < opens_at {
            return SessionDisplayState::Waiting;
        }
    }

    let en_marche = matches!(
        status,
        GameServerStatus::Running | GameServerStatus::Starting
    );

    if en_marche {
        return SessionDisplayState::Open;
    }

    // Conteneur arrêté. La fenêtre décide : dedans, la session reprendra ;
    // au-delà, elle est finie.
    match closes_at {
        Some(closes_at) if now <= closes_at => SessionDisplayState::Waiting,
        Some(_) => SessionDisplayState::Closed,
        // Sans heure de fin, on ne peut rien promettre : un serveur arrêté
        // reste annoncé fermé plutôt que « bientôt » pour toujours.
        None => SessionDisplayState::Closed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(heure: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(heure)
            .unwrap()
            .with_timezone(&Utc)
    }

    const OUVERTURE: &str = "2026-08-17T20:00:00Z";
    const FERMETURE: &str = "2026-08-17T23:00:00Z";

    fn etat(status: GameServerStatus, maintenant: &str) -> SessionDisplayState {
        session_display_state(
            status,
            Some(t(OUVERTURE)),
            Some(t(FERMETURE)),
            t(maintenant),
        )
    }

    #[test]
    fn avant_l_ouverture_c_est_toujours_bientot() {
        // Y compris conteneur en marche : le worker le démarre en avance, mais
        // l'adresse n'est pas encore donnée.
        assert_eq!(
            etat(GameServerStatus::Stopped, "2026-08-17T18:00:00Z"),
            SessionDisplayState::Waiting
        );
        assert_eq!(
            etat(GameServerStatus::Running, "2026-08-17T19:55:00Z"),
            SessionDisplayState::Waiting
        );
        assert_eq!(
            etat(GameServerStatus::Created, "2026-08-17T10:00:00Z"),
            SessionDisplayState::Waiting
        );
    }

    #[test]
    fn pendant_la_fenetre_le_conteneur_decide() {
        assert_eq!(
            etat(GameServerStatus::Running, "2026-08-17T21:00:00Z"),
            SessionDisplayState::Open
        );
        // Arrêté au milieu de la session : elle va reprendre, on ne l'enterre
        // pas.
        assert_eq!(
            etat(GameServerStatus::Stopped, "2026-08-17T21:00:00Z"),
            SessionDisplayState::Waiting
        );
        assert_eq!(
            etat(GameServerStatus::Stopping, "2026-08-17T21:00:00Z"),
            SessionDisplayState::Waiting
        );
    }

    #[test]
    fn apres_la_fermeture_et_conteneur_arrete_c_est_fini() {
        assert_eq!(
            etat(GameServerStatus::Stopped, "2026-08-18T01:00:00Z"),
            SessionDisplayState::Closed
        );
    }

    #[test]
    fn un_serveur_qui_tourne_encore_apres_l_heure_reste_ouvert() {
        // On peut y jouer : l'annoncer fermé serait un mensonge visible.
        assert_eq!(
            etat(GameServerStatus::Running, "2026-08-18T01:00:00Z"),
            SessionDisplayState::Open
        );
    }

    #[test]
    fn le_demarrage_compte_comme_ouvert_dans_la_fenetre() {
        // `starting` dure quelques minutes : afficher « fermé » entre-temps
        // ferait clignoter la carte pour rien.
        assert_eq!(
            etat(GameServerStatus::Starting, "2026-08-17T20:05:00Z"),
            SessionDisplayState::Open
        );
    }

    #[test]
    fn sans_fenetre_on_ne_dit_que_ce_qu_on_sait() {
        // Serveur jamais programmé : pas de promesse d'ouverture possible.
        assert_eq!(
            session_display_state(GameServerStatus::Running, None, None, t(OUVERTURE)),
            SessionDisplayState::Open
        );
        assert_eq!(
            session_display_state(GameServerStatus::Stopped, None, None, t(OUVERTURE)),
            SessionDisplayState::Closed
        );
    }

    #[test]
    fn sans_heure_de_fin_un_serveur_arrete_est_ferme() {
        // Sinon la carte annoncerait « ouvre bientôt » indéfiniment.
        assert_eq!(
            session_display_state(
                GameServerStatus::Stopped,
                Some(t(OUVERTURE)),
                None,
                t("2026-08-17T21:00:00Z")
            ),
            SessionDisplayState::Closed
        );
    }

    #[test]
    fn une_erreur_ou_une_suppression_ferme_la_session() {
        // Rien ne rouvrira tout seul : promettre une reprise serait faux.
        assert_eq!(
            etat(GameServerStatus::Error, "2026-08-17T21:00:00Z"),
            SessionDisplayState::Closed
        );
        assert_eq!(
            etat(GameServerStatus::Deleted, "2026-08-17T18:00:00Z"),
            SessionDisplayState::Closed
        );
    }

    #[test]
    fn les_suffixes_de_jaquette_suivent_l_etat() {
        assert_eq!(SessionDisplayState::Open.cover_suffix(), "");
        assert_eq!(SessionDisplayState::Waiting.cover_suffix(), "_attente");
        assert_eq!(SessionDisplayState::Closed.cover_suffix(), "_offline");
    }
}
