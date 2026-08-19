//! Horaires d'ouverture recurrents d'un serveur de jeu.
//!
//! Un serveur de soiree n'a pas besoin de tourner la journee : il consomme
//! memoire et processeur pour personne. Ce module decide, a un instant donne,
//! si un serveur DOIT etre allume, eteint, ou si ses joueurs doivent etre
//! prevenus d'une fermeture prochaine.
//!
//! Trois pieges, tous couverts par des tests :
//!
//!   - **le fuseau horaire.** Les plages sont saisies en heure LOCALE
//!     (« 19h-minuit »), et l'heure d'ete decale l'UTC d'une heure deux fois
//!     par an. Un decalage fixe ferait ouvrir le serveur a 18h ou 20h la
//!     moitie de l'annee ;
//!   - **les plages qui passent minuit.** « 22h-02h » ne s'ecrit pas
//!     `debut <= maintenant < fin` : sans traiter le franchissement, la plage
//!     n'est jamais active ;
//!   - **la fin de session.** Passe la date de fermeture, plus rien ne
//!     redemarre — sans quoi un serveur ressusciterait chaque jour a 19h
//!     jusqu'a la fin des temps.

use chrono::{DateTime, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

/// Une plage d'ouverture quotidienne, en heure locale.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeRange {
    /// Minutes depuis minuit (0..1440). `19:00` vaut 1140.
    pub start_minute: u16,
    pub end_minute: u16,
}

impl TimeRange {
    /// La plage franchit-elle minuit ? (`22:00` -> `02:00`)
    pub fn crosses_midnight(&self) -> bool {
        self.end_minute <= self.start_minute
    }

    /// La minute donnee tombe-t-elle dans la plage ?
    ///
    /// La fin est EXCLUE : deux plages qui se touchent (`12:00-14:00` puis
    /// `14:00-16:00`) ne doivent pas se chevaucher a 14h00 pile.
    pub fn contains(&self, minute_of_day: u16) -> bool {
        if self.crosses_midnight() {
            minute_of_day >= self.start_minute || minute_of_day < self.end_minute
        } else {
            minute_of_day >= self.start_minute && minute_of_day < self.end_minute
        }
    }

    /// Minutes restantes avant la fin de la plage, si l'on est dedans.
    pub fn minutes_until_end(&self, minute_of_day: u16) -> Option<u16> {
        if !self.contains(minute_of_day) {
            return None;
        }
        Some(if self.end_minute > minute_of_day {
            self.end_minute - minute_of_day
        } else {
            // Traverse minuit : le reste de la journee, plus le debut de la
            // suivante.
            (1440 - minute_of_day) + self.end_minute
        })
    }
}

/// Reglages d'ouverture automatique d'un serveur.
#[derive(Debug, Clone)]
pub struct AutoSchedule {
    pub enabled: bool,
    /// Nom IANA du fuseau (« Europe/Paris »). Les plages sont exprimees dedans.
    pub timezone: String,
    pub ranges: Vec<TimeRange>,
    /// Preavis avant fermeture, en minutes. 0 = pas d'annonce.
    pub warn_minutes: u16,
    /// Date de fin de session : au-dela, plus aucun demarrage.
    pub closes_at: Option<DateTime<Utc>>,
}

/// Ce qu'il faut faire du serveur, maintenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleAction {
    /// Rien a faire : l'etat courant est le bon.
    Nothing,
    /// Demarrer : on est dans une plage et le serveur est eteint.
    Start,
    /// Arreter : hors plage, ou session terminee.
    Stop { reason: StopReason },
    /// Prevenir les joueurs d'une fermeture prochaine.
    Warn { minutes_left: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Fin de la plage horaire du jour : le serveur rouvrira.
    OutsideRange,
    /// Date de fin de session atteinte : il ne rouvrira plus.
    SessionOver,
}

/// Decide de l'action a mener.
///
/// `running` est l'etat courant du conteneur. `already_warned` evite de
/// repeter l'annonce a chaque passage du job : sans lui, les joueurs
/// recevraient le meme message chaque minute pendant tout le preavis.
pub fn decide(
    schedule: &AutoSchedule,
    running: bool,
    already_warned: bool,
    now: DateTime<Utc>,
) -> ScheduleAction {
    if !schedule.enabled {
        return ScheduleAction::Nothing;
    }

    // Session terminee : on eteint, et plus rien ne redemarre. Sans cette
    // porte, un serveur ressusciterait chaque jour a 19h indefiniment.
    if let Some(fin) = schedule.closes_at {
        if now >= fin {
            return if running {
                ScheduleAction::Stop {
                    reason: StopReason::SessionOver,
                }
            } else {
                ScheduleAction::Nothing
            };
        }
    }

    // Un fuseau inconnu ne doit pas faire tourner un serveur a contretemps :
    // on s'abstient plutot que de retomber sur UTC en silence.
    let Ok(tz) = schedule.timezone.parse::<Tz>() else {
        return ScheduleAction::Nothing;
    };
    let locale = now.with_timezone(&tz);
    let minute_of_day = (locale.hour() * 60 + locale.minute()) as u16;

    let plage_active = schedule
        .ranges
        .iter()
        .find(|plage| plage.contains(minute_of_day));

    match plage_active {
        Some(plage) => {
            if !running {
                return ScheduleAction::Start;
            }
            // Preavis : seulement dans la fenetre, et une seule fois.
            if schedule.warn_minutes > 0 && !already_warned {
                if let Some(restant) = plage.minutes_until_end(minute_of_day) {
                    if restant <= schedule.warn_minutes {
                        return ScheduleAction::Warn {
                            minutes_left: restant,
                        };
                    }
                }
            }
            ScheduleAction::Nothing
        }
        None => {
            if running {
                ScheduleAction::Stop {
                    reason: StopReason::OutsideRange,
                }
            } else {
                ScheduleAction::Nothing
            }
        }
    }
}

/// Prochaine ouverture, pour l'annoncer aux joueurs.
///
/// Cherche sur 48 h : au-dela, une configuration sans plage exploitable ne
/// donnerait de toute facon jamais de reponse.
pub fn next_opening(schedule: &AutoSchedule, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if !schedule.enabled || schedule.ranges.is_empty() {
        return None;
    }
    let tz = schedule.timezone.parse::<Tz>().ok()?;
    let locale = now.with_timezone(&tz);

    let mut debuts: Vec<u16> = schedule.ranges.iter().map(|r| r.start_minute).collect();
    debuts.sort_unstable();

    for jour in 0..2 {
        let date = (locale + chrono::Duration::days(jour)).date_naive();
        for debut in &debuts {
            let heure = NaiveTime::from_hms_opt((*debut / 60) as u32, (*debut % 60) as u32, 0)?;
            let naive = date.and_time(heure);
            // Heure locale inexistante (passage a l'heure d'ete) : on passe au
            // creneau suivant plutot que d'annoncer une heure qui n'existe pas.
            let chrono::LocalResult::Single(instant) = tz.from_local_datetime(&naive) else {
                continue;
            };
            let instant = instant.with_timezone(&Utc);
            if instant > now && schedule.closes_at.is_none_or(|fin| instant < fin) {
                return Some(instant);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// « 19:00 » -> 1140.
    fn hm(h: u16, m: u16) -> u16 {
        h * 60 + m
    }

    fn plage(h1: u16, m1: u16, h2: u16, m2: u16) -> TimeRange {
        TimeRange {
            start_minute: hm(h1, m1),
            end_minute: hm(h2, m2),
        }
    }

    /// Instant UTC. En aout, Paris est a UTC+2 : 17h UTC = 19h locales.
    fn utc(jour: u32, heure: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, jour, heure, minute, 0)
            .unwrap()
    }

    fn horaire(ranges: Vec<TimeRange>) -> AutoSchedule {
        AutoSchedule {
            enabled: true,
            timezone: "Europe/Paris".into(),
            ranges,
            warn_minutes: 10,
            closes_at: None,
        }
    }

    #[test]
    fn une_plage_normale_ouvre_et_ferme_a_l_heure_locale() {
        // 12h-14h locales = 10h-12h UTC en aout.
        let h = horaire(vec![plage(12, 0, 14, 0)]);

        assert_eq!(
            decide(&h, false, false, utc(19, 10, 30)),
            ScheduleAction::Start
        );
        assert_eq!(
            decide(&h, true, false, utc(19, 12, 30)),
            ScheduleAction::Stop {
                reason: StopReason::OutsideRange
            }
        );
    }

    #[test]
    fn l_heure_d_ete_ne_decale_pas_l_ouverture() {
        // C'est le piege qu'un decalage fixe ne verrait pas : la meme plage
        // locale correspond a deux heures UTC differentes selon la saison.
        let h = horaire(vec![plage(19, 0, 23, 0)]);

        // Aout (UTC+2) : 19h locales = 17h UTC.
        assert_eq!(
            decide(&h, false, false, utc(19, 17, 5)),
            ScheduleAction::Start
        );
        assert_eq!(
            decide(&h, false, false, utc(19, 16, 30)),
            ScheduleAction::Nothing
        );

        // Janvier (UTC+1) : 19h locales = 18h UTC.
        let hiver = Utc.with_ymd_and_hms(2026, 1, 15, 18, 5, 0).unwrap();
        let avant = Utc.with_ymd_and_hms(2026, 1, 15, 17, 30, 0).unwrap();
        assert_eq!(decide(&h, false, false, hiver), ScheduleAction::Start);
        assert_eq!(decide(&h, false, false, avant), ScheduleAction::Nothing);
    }

    #[test]
    fn une_plage_qui_passe_minuit_reste_active() {
        // « 19h - minuit » puis « 22h - 02h » : sans traitement du
        // franchissement, ces plages ne seraient jamais actives.
        let p = plage(22, 0, 2, 0);
        assert!(p.crosses_midnight());
        assert!(p.contains(hm(23, 30)));
        assert!(p.contains(hm(1, 0)));
        assert!(!p.contains(hm(3, 0)));
        assert!(!p.contains(hm(21, 0)));
    }

    #[test]
    fn deux_plages_qui_se_touchent_ne_se_chevauchent_pas() {
        // A 14h00 pile, la premiere est finie et la seconde commence : la fin
        // est exclue, sinon un serveur serait « dans deux plages » a la fois.
        let midi = plage(12, 0, 14, 0);
        let apres = plage(14, 0, 16, 0);
        assert!(!midi.contains(hm(14, 0)));
        assert!(apres.contains(hm(14, 0)));
    }

    #[test]
    fn le_preavis_ne_part_qu_une_fois_et_dans_la_fenetre() {
        let h = horaire(vec![plage(12, 0, 14, 0)]);

        // 13h55 locales = 11h55 UTC : 5 minutes avant la fin.
        assert_eq!(
            decide(&h, true, false, utc(19, 11, 55)),
            ScheduleAction::Warn { minutes_left: 5 }
        );
        // Deja annonce : on ne repete pas a chaque passage du job.
        assert_eq!(
            decide(&h, true, true, utc(19, 11, 55)),
            ScheduleAction::Nothing
        );
        // Trop tot pour prevenir.
        assert_eq!(
            decide(&h, true, false, utc(19, 11, 0)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn le_preavis_traverse_minuit_sans_se_tromper() {
        // Plage 22h-02h, il est 01h55 locales : 5 minutes restent, pas 1435.
        let h = horaire(vec![plage(22, 0, 2, 0)]);
        let p = h.ranges[0];
        assert_eq!(p.minutes_until_end(hm(1, 55)), Some(5));
        assert_eq!(p.minutes_until_end(hm(23, 0)), Some(180));
    }

    #[test]
    fn la_fin_de_session_eteint_et_ne_rouvre_plus() {
        // Sans cette porte, le serveur ressusciterait chaque jour a 12h.
        let mut h = horaire(vec![plage(12, 0, 14, 0)]);
        h.closes_at = Some(utc(19, 9, 0));

        assert_eq!(
            decide(&h, true, false, utc(19, 10, 30)),
            ScheduleAction::Stop {
                reason: StopReason::SessionOver
            }
        );
        // Deja eteint : on n'insiste pas, et on ne redemarre surtout pas.
        assert_eq!(
            decide(&h, false, false, utc(19, 10, 30)),
            ScheduleAction::Nothing
        );
        assert_eq!(
            decide(&h, false, false, utc(20, 10, 30)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn desactive_le_systeme_ne_touche_a_rien() {
        // L'administrateur garde la main : hors plage, un serveur qu'il a
        // demarre lui-meme doit le rester.
        let mut h = horaire(vec![plage(12, 0, 14, 0)]);
        h.enabled = false;
        assert_eq!(
            decide(&h, true, false, utc(19, 20, 0)),
            ScheduleAction::Nothing
        );
        assert_eq!(
            decide(&h, false, false, utc(19, 10, 30)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn un_fuseau_inconnu_ne_declenche_rien() {
        // Retomber sur UTC en silence ferait tourner le serveur a contretemps
        // — deux heures a cote en ete, sans que personne ne comprenne.
        let mut h = horaire(vec![plage(12, 0, 14, 0)]);
        h.timezone = "Mars/Olympus".into();
        assert_eq!(
            decide(&h, false, false, utc(19, 10, 30)),
            ScheduleAction::Nothing
        );
    }

    #[test]
    fn la_prochaine_ouverture_est_annoncable() {
        let h = horaire(vec![plage(12, 0, 14, 0), plage(19, 0, 23, 0)]);

        // 08h UTC = 10h locales : la prochaine ouverture est a 12h locales.
        let suivante = next_opening(&h, utc(19, 8, 0)).unwrap();
        assert_eq!(suivante, utc(19, 10, 0));

        // 13h UTC = 15h locales : la plage de midi est passee, reste 19h.
        let suivante = next_opening(&h, utc(19, 13, 0)).unwrap();
        assert_eq!(suivante, utc(19, 17, 0));

        // Apres la derniere plage du jour : on bascule au lendemain.
        let suivante = next_opening(&h, utc(19, 22, 0)).unwrap();
        assert_eq!(suivante, utc(20, 10, 0));
    }

    #[test]
    fn aucune_ouverture_apres_la_fin_de_session() {
        let mut h = horaire(vec![plage(12, 0, 14, 0)]);
        h.closes_at = Some(utc(19, 9, 0));
        assert_eq!(next_opening(&h, utc(19, 8, 0)), None);
    }
}
