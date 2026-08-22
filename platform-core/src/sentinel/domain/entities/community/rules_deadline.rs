//! Delai d'acceptation du reglement, pour les arrivants ORDINAIRES.
//!
//! # Pourquoi ce module existe a cote de la quarantaine
//!
//! La quarantaine (`system::quarantine`) traite les comptes SUSPECTS : pattern
//! de raid, arrivees en rafale, compte trop recent, alt d'un banni. Elle pose un
//! role a acces ultra-restreint et exige un captcha. Un membre qui arrive
//! normalement n'y entre jamais.
//!
//! Il manquait donc l'autre moitie : quelqu'un rejoint, on lui presente le
//! reglement, et plus rien. Il pouvait rester indefiniment sans avoir clique,
//! occupant une place et ne voyant qu'un salon.
//!
//! Confondre les deux serait une faute de conception : un raid se traite en
//! secondes et se solde par une expulsion massive ; quelqu'un qui tarde a
//! cliquer merite des jours et une relance. Les deux n'ont ni le meme rythme,
//! ni le meme role Discord, ni le meme message.
//!
//! # Ce que ce module decide
//!
//! Rien qui touche Discord ni la base : seulement des dates. Quand expire le
//! delai d'un arrivant, quand lui envoyer une relance, et si l'expulsion est
//! permise. Le reste vit dans les jobs et le bot.

use chrono::{DateTime, Duration, Utc};

/// Plancher du delai. Une heure : en dessous, on expulse quelqu'un qui n'a pas
/// eu le temps d'ouvrir Discord depuis son telephone. Ce n'est pas un raid, il
/// n'y a aucune urgence.
pub const MIN_DEADLINE_SECS: i64 = 3600;

/// Plafond : trente jours. Au-dela, le delai n'attend plus une reponse, il
/// laisse s'installer une file d'attente — ce que `kick_enabled = false`
/// exprime deja, et plus honnetement.
pub const MAX_DEADLINE_SECS: i64 = 30 * 24 * 3600;

/// Delai par defaut : trois jours. Assez long pour couvrir un week-end sans
/// connexion, assez court pour que la file ne s'allonge pas indefiniment.
pub const DEFAUT_DEADLINE_SECS: i64 = 3 * 24 * 3600;

/// Relance par defaut : vingt-quatre heures avant l'echeance.
pub const DEFAUT_RELANCE_SECS: i64 = 24 * 3600;

/// Ce que la guilde a decide pour ses arrivants qui n'ont pas encore accepte le
/// reglement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulesDeadlineSettings {
    /// Faux : aucune echeance n'est posee, le systeme dort entierement. C'est
    /// le defaut — activer une expulsion automatique doit etre un geste
    /// delibere, jamais un effet de bord de mise a jour.
    pub enabled: bool,
    /// Temps laisse pour cliquer sur « J'accepte ».
    pub deadline_secs: i64,
    /// Relance envoyee ce nombre de secondes AVANT l'echeance. Zero : aucune.
    pub reminder_secs: i64,
    /// Faux : personne n'est expulse, l'echeance sert seulement a relancer et a
    /// rendre la file visible.
    pub kick_enabled: bool,
}

impl Default for RulesDeadlineSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            deadline_secs: DEFAUT_DEADLINE_SECS,
            reminder_secs: DEFAUT_RELANCE_SECS,
            kick_enabled: true,
        }
    }
}

impl RulesDeadlineSettings {
    /// Ramene des valeurs saisies a la main dans un domaine ou elles ont un
    /// sens, plutot que de refuser la configuration : une faute de frappe ne
    /// doit pas figer l'accueil, ni surtout raccourcir un delai au point
    /// d'expulser tout le monde.
    pub fn sanitized(mut self) -> Self {
        self.deadline_secs = self
            .deadline_secs
            .clamp(MIN_DEADLINE_SECS, MAX_DEADLINE_SECS);
        // Une relance prevue plus tot que l'arrivee partirait en meme temps que
        // le message d'accueil : deux messages dans la meme seconde, dont un
        // qui menace d'expulsion. On la replace a mi-parcours.
        if self.reminder_secs >= self.deadline_secs {
            self.reminder_secs = self.deadline_secs / 2;
        }
        if self.reminder_secs < 0 {
            self.reminder_secs = 0;
        }
        self
    }

    /// Echeance d'un membre arrivant maintenant.
    pub fn expires_from(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now + Duration::seconds(self.deadline_secs)
    }

    /// Moment ou la relance doit partir. `None` quand la guilde n'en veut pas.
    pub fn reminder_due_at(&self, expires_at: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if self.reminder_secs <= 0 {
            return None;
        }
        Some(expires_at - Duration::seconds(self.reminder_secs))
    }
}

/// Une echeance en cours, telle qu'elle est suivie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRulesDeadline {
    pub guild_id: String,
    pub user_id: String,
    pub expires_at: DateTime<Utc>,
    /// Date de la relance deja envoyee. `None` tant qu'aucune n'est partie.
    pub reminded_at: Option<DateTime<Utc>>,
}

/// Ce qu'il faut faire d'une echeance, a un instant donne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesDeadlineAction {
    /// Le delai court encore, et la relance n'est pas due.
    Nothing,
    /// Envoyer la relance en message prive.
    Remind,
    /// Le delai est ecoule : expulser.
    Kick,
    /// Le delai est ecoule mais l'expulsion est desactivee : le membre reste,
    /// et son echeance avec — c'est la file d'attente que l'administrateur a
    /// choisi de laisser grandir.
    KeepWaiting,
}

/// Decision PURE.
///
/// L'ordre compte : l'expiration passe AVANT la relance. Une relance due en
/// meme temps que l'echeance n'aurait aucun interet — prevenir quelqu'un a
/// l'instant ou on l'expulse est pire que de ne rien dire.
pub fn decide(
    settings: &RulesDeadlineSettings,
    pending: &PendingRulesDeadline,
    now: DateTime<Utc>,
) -> RulesDeadlineAction {
    if !settings.enabled {
        return RulesDeadlineAction::Nothing;
    }

    if now >= pending.expires_at {
        return if settings.kick_enabled {
            RulesDeadlineAction::Kick
        } else {
            RulesDeadlineAction::KeepWaiting
        };
    }

    // Une relance deja partie ne repart pas : sans cette memoire, un balayage
    // toutes les minutes enverrait un message prive toutes les minutes pendant
    // toute la fenetre de relance.
    if pending.reminded_at.is_some() {
        return RulesDeadlineAction::Nothing;
    }

    match settings.reminder_due_at(pending.expires_at) {
        Some(due) if now >= due => RulesDeadlineAction::Remind,
        _ => RulesDeadlineAction::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000 + h * 3600, 0).unwrap()
    }

    fn reglages() -> RulesDeadlineSettings {
        RulesDeadlineSettings {
            enabled: true,
            deadline_secs: 72 * 3600, // 3 jours
            reminder_secs: 24 * 3600, // relance a J-1
            kick_enabled: true,
        }
    }

    fn attente(expire_a: DateTime<Utc>, relance: Option<DateTime<Utc>>) -> PendingRulesDeadline {
        PendingRulesDeadline {
            guild_id: "g".into(),
            user_id: "u".into(),
            expires_at: expire_a,
            reminded_at: relance,
        }
    }

    #[test]
    fn le_systeme_dort_tant_qu_il_n_est_pas_active() {
        // Le defaut n'expulse personne : activer une expulsion automatique doit
        // etre un geste delibere, pas un effet de bord de mise a jour.
        let d = RulesDeadlineSettings::default();
        assert!(!d.enabled);

        let mut r = reglages();
        r.enabled = false;
        // Meme largement expiree, une echeance ne declenche rien.
        assert_eq!(
            decide(&r, &attente(t(0), None), t(100)),
            RulesDeadlineAction::Nothing
        );
    }

    #[test]
    fn rien_ne_se_passe_tant_que_le_delai_court() {
        let r = reglages();
        // Echeance dans 72 h, relance prevue a J-1 : a l'arrivee, rien.
        assert_eq!(
            decide(&r, &attente(t(72), None), t(0)),
            RulesDeadlineAction::Nothing
        );
    }

    #[test]
    fn la_relance_part_a_la_fenetre_prevue() {
        let r = reglages();
        // Relance due 24 h avant l'echeance de t(72), donc a t(48).
        assert_eq!(
            decide(&r, &attente(t(72), None), t(48)),
            RulesDeadlineAction::Remind
        );
        // Une heure trop tot : pas encore.
        assert_eq!(
            decide(&r, &attente(t(72), None), t(47)),
            RulesDeadlineAction::Nothing
        );
    }

    #[test]
    fn la_relance_ne_part_qu_une_fois() {
        // Sans cette memoire, un balayage regulier enverrait un message prive a
        // chaque passage pendant toute la fenetre.
        let r = reglages();
        assert_eq!(
            decide(&r, &attente(t(72), Some(t(48))), t(50)),
            RulesDeadlineAction::Nothing
        );
    }

    #[test]
    fn l_expiration_prime_sur_la_relance() {
        // Prevenir quelqu'un a l'instant ou on l'expulse est pire que de ne
        // rien dire : l'echeance est verifiee en premier.
        let mut r = reglages();
        r.reminder_secs = 72 * 3600 - 1; // fenetre de relance quasi permanente
        let r = r.sanitized();
        assert_eq!(
            decide(&r, &attente(t(72), None), t(72)),
            RulesDeadlineAction::Kick
        );
    }

    #[test]
    fn sans_expulsion_le_membre_reste_en_attente() {
        let mut r = reglages();
        r.kick_enabled = false;
        assert_eq!(
            decide(&r, &attente(t(72), None), t(80)),
            RulesDeadlineAction::KeepWaiting
        );
    }

    #[test]
    fn un_delai_aberrant_est_ramene_dans_les_bornes() {
        // Refuser la configuration figerait l'accueil ; pire, un delai a zero
        // expulserait tout le monde des l'arrivee.
        let r = RulesDeadlineSettings {
            enabled: true,
            deadline_secs: 0,
            reminder_secs: 0,
            kick_enabled: true,
        }
        .sanitized();
        assert_eq!(r.deadline_secs, MIN_DEADLINE_SECS);

        let r = RulesDeadlineSettings {
            enabled: true,
            deadline_secs: 10 * 365 * 24 * 3600,
            reminder_secs: 0,
            kick_enabled: true,
        }
        .sanitized();
        assert_eq!(r.deadline_secs, MAX_DEADLINE_SECS);
    }

    #[test]
    fn une_relance_plus_longue_que_le_delai_est_replacee_a_mi_parcours() {
        // Sinon elle partirait en meme temps que le message d'accueil : deux
        // messages dans la meme seconde, dont un qui menace d'expulsion.
        let r = RulesDeadlineSettings {
            enabled: true,
            deadline_secs: 72 * 3600,
            reminder_secs: 100 * 3600,
            kick_enabled: true,
        }
        .sanitized();
        assert_eq!(r.reminder_secs, 36 * 3600);
    }

    #[test]
    fn une_relance_negative_vaut_pas_de_relance() {
        let r = RulesDeadlineSettings {
            enabled: true,
            deadline_secs: 72 * 3600,
            reminder_secs: -5,
            kick_enabled: true,
        }
        .sanitized();
        assert_eq!(r.reminder_secs, 0);
        assert_eq!(r.reminder_due_at(t(72)), None);
        // Et aucune relance ne part jamais.
        assert_eq!(
            decide(&r, &attente(t(72), None), t(71)),
            RulesDeadlineAction::Nothing
        );
    }

    #[test]
    fn l_echeance_se_calcule_depuis_l_arrivee() {
        let r = reglages();
        assert_eq!(r.expires_from(t(0)), t(72));
        assert_eq!(r.reminder_due_at(t(72)), Some(t(48)));
    }

    #[test]
    fn a_la_seconde_pres_l_echeance_expulse() {
        // Frontiere exacte : `>=`, comme partout ailleurs dans le domaine.
        let r = reglages();
        assert_eq!(
            decide(&r, &attente(t(72), None), t(72)),
            RulesDeadlineAction::Kick
        );
    }
}
