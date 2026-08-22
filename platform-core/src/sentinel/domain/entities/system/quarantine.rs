//! Quarantaine de securite : un compte juge SUSPECT a l'arrivee (pattern de
//! raid, arrivees en rafale, compte trop recent, alt d'un banni) place en acces
//! ultra-restreint le temps qu'il passe un captcha, avec expulsion automatique
//! si le delai expire.
//!
//! A ne pas confondre avec l'acceptation du reglement, qui vit dans le module
//! Accueil et concerne TOUS les arrivants. Ici, un membre qui arrive
//! normalement n'entre jamais en quarantaine.
//!
//! Le delai protege surtout les faux positifs : un membre legitime dont le
//! compte vient d'etre cree est classe suspect, et un delai trop court
//! l'expulserait avant qu'il ait vu le message prive.

use chrono::{DateTime, Duration, Utc};

/// Une quarantaine encore active (non expiree), utilisee pour rehydrater le
/// tracker RAM du bot au demarrage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveQuarantine {
    pub guild_id: String,
    pub user_id: String,
}

/// Delai minimum avant expulsion. Une seconde suffisait a l'ancienne regle
/// (garde anti-configuration nulle) ; une minute est le vrai plancher utile,
/// puisqu'un message prive doit avoir le temps d'etre lu.
pub const MIN_TIMEOUT_SECS: i64 = 60;

/// Plafond : trente jours. Au-dela, une quarantaine n'attend plus une reponse,
/// elle est devenue un etat permanent — ce que `kick_enabled = false` exprime
/// deja, et plus honnetement.
pub const MAX_TIMEOUT_SECS: i64 = 30 * 24 * 3600;

/// Ce que la guilde a decide pour les comptes suspects en attente de
/// verification.
///
/// Ces valeurs vivaient dans une variable d'environnement globale a cinq
/// minutes. Elles appartiennent au serveur : le rythme d'une petite communaute
/// ou les gens rejoignent le soir n'est pas celui d'un serveur public sous
/// raid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantineSettings {
    /// Temps laisse au membre pour se verifier.
    pub timeout_secs: i64,
    /// Faux : personne n'est jamais expulse automatiquement, la quarantaine
    /// attend une decision humaine.
    pub kick_enabled: bool,
    /// Rappel envoye ce nombre de secondes AVANT l'echeance. Zero : aucun.
    pub reminder_secs: i64,
    /// Salon du reglement, cite dans le rappel. Absent : message general.
    pub rules_channel_id: Option<String>,
}

impl Default for QuarantineSettings {
    fn default() -> Self {
        Self {
            timeout_secs: 24 * 3600,
            kick_enabled: true,
            reminder_secs: 3600,
            rules_channel_id: None,
        }
    }
}

impl QuarantineSettings {
    /// Ramene des valeurs saisies a la main dans un domaine ou elles ont un
    /// sens, plutot que de refuser la configuration : un reglage aberrant ne
    /// doit pas empecher la quarantaine de fonctionner, sinon une faute de
    /// frappe ouvrirait le serveur.
    pub fn sanitized(mut self) -> Self {
        self.timeout_secs = self.timeout_secs.clamp(MIN_TIMEOUT_SECS, MAX_TIMEOUT_SECS);
        // Un rappel prevu plus tot que l'arrivee elle-meme partirait en meme
        // temps que le premier message, ce qui ferait deux messages identiques
        // dans la meme seconde. On le ramene a la moitie du delai : il reste un
        // rappel, place la ou il sert.
        if self.reminder_secs >= self.timeout_secs {
            self.reminder_secs = self.timeout_secs / 2;
        }
        if self.reminder_secs < 0 {
            self.reminder_secs = 0;
        }
        self.rules_channel_id = self
            .rules_channel_id
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());
        self
    }

    /// Date d'expulsion pour un membre arrivant maintenant.
    pub fn expires_from(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now + Duration::seconds(self.timeout_secs)
    }

    /// Moment ou le rappel doit partir, pour une echeance donnee. `None` quand
    /// la guilde n'en veut pas.
    pub fn reminder_due_at(&self, expires_at: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if self.reminder_secs <= 0 {
            return None;
        }
        Some(expires_at - Duration::seconds(self.reminder_secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_delai_aberrant_est_ramene_dans_les_bornes_au_lieu_d_etre_refuse() {
        // Refuser la configuration laisserait la quarantaine sans delai, donc
        // sans expulsion : une faute de frappe ouvrirait le serveur.
        let court = QuarantineSettings {
            timeout_secs: 0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(court.timeout_secs, MIN_TIMEOUT_SECS);

        let long = QuarantineSettings {
            timeout_secs: 10 * 365 * 24 * 3600,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(long.timeout_secs, MAX_TIMEOUT_SECS);
    }

    #[test]
    fn un_rappel_plus_long_que_le_delai_est_replace_a_mi_parcours() {
        // Sinon il partirait a l'instant meme de l'arrivee, en double du
        // message de verification.
        let s = QuarantineSettings {
            timeout_secs: 3600,
            reminder_secs: 7200,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.reminder_secs, 1800);
    }

    #[test]
    fn un_rappel_a_zero_ne_programme_rien() {
        let s = QuarantineSettings {
            reminder_secs: 0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.reminder_due_at(Utc::now()), None);
    }

    #[test]
    fn le_rappel_precede_l_echeance_du_delai_configure() {
        let s = QuarantineSettings {
            timeout_secs: 24 * 3600,
            reminder_secs: 3600,
            ..Default::default()
        }
        .sanitized();
        let echeance = Utc::now();
        assert_eq!(
            s.reminder_due_at(echeance),
            Some(echeance - Duration::seconds(3600))
        );
    }

    #[test]
    fn un_salon_vide_vaut_pas_de_salon() {
        // Le message cite le salon ; une chaine vide afficherait un lien mort.
        let s = QuarantineSettings {
            rules_channel_id: Some("   ".into()),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.rules_channel_id, None);
    }
}
